// crates/zettel-cli/src/main.rs - Command-line interface for Luhmann-style Zettelkasten management
//
// This is the main entry point for the zettel CLI tool. It implements a Unix-style command structure
// where each subcommand does one thing well and can be composed with other tools.
//
// ARCHITECTURE OVERVIEW:
// ┌─────────────────┐    ┌──────────────────┐    ┌─────────────────────┐
// │   User Input    │───▶│   CLI Parser     │───▶│  Command Handlers   │
// │ (clap commands) │    │ (main function)  │    │ (handle_* functions)│
// └─────────────────┘    └──────────────────┘    └─────────────────────┘
//                                 │                        │
//                                 ▼                        ▼
//                        ┌──────────────────┐    ┌─────────────────────┐
//                        │    Context       │    │   Core Services     │
//                        │ (vault + config) │    │ (zettel-core crate) │
//                        └──────────────────┘    └─────────────────────┘
//
// DESIGN PRINCIPLES:
// - Each command has a single responsibility (Unix philosophy)
// - Commands output machine-readable formats (JSON) for scripting
// - Context is passed explicitly (no global state)
// - Error handling follows Rust patterns (Result<T, E>)
// - Commands can be chained with pipes for complex workflows

use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use std::fs;
use std::env;
use zettel_core::id::{Id, IdManager, IdConfig};

/// Main CLI structure defining global options and subcommands
///
/// This uses clap's derive API for clean, declarative command definition.
/// The structure mirrors typical Unix tools where you have:
/// - Global options (like --vault) that apply to all commands
/// - Subcommands that implement specific functionality
///
/// Example usage:
/// ```bash
/// zettel --vault ~/notes init
/// zettel note create 1 "My First Note"
/// zettel list --json | jq '.[] | .id'
/// ```
#[derive(Parser)]
#[command(name = "zettel")]
#[command(about = "A CLI for Luhmann-style Zettelkasten management")]
#[command(version = "0.1.0")]
struct Cli {
    /// Vault directory (overrides ZETTEL_VAULT environment variable)
    ///
    /// This allows users to work with multiple vaults or override the default.
    /// Following Unix conventions: CLI args > environment > defaults
    #[arg(short, long, global = true)]
    vault: Option<PathBuf>,
    
    /// Main command dispatch - each variant corresponds to a major area of functionality
    #[command(subcommand)]
    command: Commands,
}

/// Top-level command categories
///
/// This groups related functionality and makes the CLI feel organized.
/// Each category can have its own subcommands for granular control.
///
/// Design: Start with few commands, add more as needed. Better to have
/// a small, well-designed CLI than a large, confusing one.
#[derive(Subcommand)]
enum Commands {
    /// Initialize a new vault
    ///
    /// Creates the directory structure and configuration files needed
    /// for a new zettelkasten vault. This is typically the first command
    /// users run when setting up a new knowledge base.
    Init {
        /// Path to create vault (defaults to current directory)
        ///
        /// If omitted, initializes in the current working directory.
        /// Creates the directory if it doesn't exist.
        path: Option<PathBuf>,
    },
    
    /// ID operations (generation, validation, parsing)
    ///
    /// Groups all ID-related functionality. This is the core of the zettelkasten
    /// system - everything revolves around proper ID generation and manipulation.
    ///
    /// These commands are designed to be composable:
    /// ```bash
    /// next_id=$(zettel id next-sibling $(zettel id parse current_file.md))
    /// zettel note create "$next_id" "My New Note"
    /// ```
    #[command(subcommand)]
    Id(IdCommands),
    
    /// Note operations (create, open, show)
    ///
    /// High-level note management. These commands coordinate between
    /// ID generation, file creation, and content management.
    #[command(subcommand)]
    Note(NoteCommands),
    
    /// List notes in the vault
    ///
    /// Provides different views of the note collection. Supports both
    /// human-readable and machine-readable output formats.
    List {
        /// Show full file paths instead of just IDs
        ///
        /// Useful for scripting or when you need to know exact file locations.
        #[arg(long)]
        full_paths: bool,
        
        /// Output as JSON for machine processing
        ///
        /// Enables integration with tools like jq, fzf, or custom scripts.
        /// JSON output includes metadata like creation time, parent relationships, etc.
        #[arg(long)]
        json: bool,
    },
    
    /// Search notes by content or title
    ///
    /// Simple text-based search. Future versions might add more sophisticated
    /// search capabilities (full-text indexing, regex, etc.).
    Search {
        /// Search query string
        ///
        /// Currently does simple case-insensitive substring matching.
        /// Searches both filenames and file content.
        query: String,
    },
}

/// ID-specific subcommands
///
/// These implement the core ID manipulation logic that powers the zettelkasten system.
/// Each command does one specific ID operation and outputs the result.
///
/// Design rationale: Fine-grained commands allow for maximum composability.
/// You can build complex workflows by chaining these simple operations.
#[derive(Subcommand)]
enum IdCommands {
    /// Generate next sibling ID
    ///
    /// Sibling = same level in hierarchy. Example: 1 -> 2, 1a -> 1b
    /// This finds the next available ID at the same hierarchical level.
    NextSibling {
        /// Current ID to increment from
        ///
        /// Must be a valid Luhmann-style ID (1, 1a, 1a2, etc.)
        id: String,
    },
    
    /// Generate next child ID
    ///
    /// Child = one level deeper in hierarchy. Example: 1 -> 1a, 1a -> 1a1
    /// This creates the next available child under the given parent.
    NextChild {
        /// Parent ID to create child under
        ///
        /// The child will follow Luhmann's alternating number/letter pattern.
        id: String,
    },
    
    /// Parse ID from filename
    ///
    /// Extracts the ID portion from a filename based on the vault's matching rules.
    /// Useful for getting the ID of the currently open file in your editor.
    Parse {
        /// Filename to extract ID from
        ///
        /// Can be just the basename (1a2-title.md) or full path.
        filename: String,
    },
    
    /// Validate ID format
    ///
    /// Checks if a string is a valid Luhmann-style ID and shows its structure.
    /// Useful for debugging ID generation or validating user input.
    Validate {
        /// ID string to validate
        ///
        /// Will show detailed information about the ID structure if valid.
        id: String,
    },
}

/// Note management subcommands
///
/// These operate at a higher level than ID commands - they create actual files,
/// manage content, and handle the user-facing aspects of note management.
#[derive(Subcommand)]
enum NoteCommands {
    /// Create a new note with given ID and title
    ///
    /// This is the main note creation command. It handles:
    /// - File creation with proper naming
    /// - Initial content generation
    /// - Template processing (if configured)
    /// - Optional editor launching
    Create {
        /// Note ID (must be valid Luhmann format)
        ///
        /// Should typically be generated using the `id` subcommands to ensure
        /// it fits properly in the hierarchy and doesn't conflict with existing notes.
        id: String,
        
        /// Note title (optional)
        ///
        /// If provided, gets added to filename (depending on config) and
        /// used as the note's heading. If omitted, note gets generic title.
        title: Option<String>,
        
        /// Open note in editor after creation
        ///
        /// Uses $ZETTEL_EDITOR, $EDITOR, or platform default.
        /// Useful for immediate editing after creation.
        #[arg(long)]
        open: bool,
    },
    
    /// Open existing note in editor
    ///
    /// Finds the note file with the given ID and opens it in the configured editor.
    /// Handles the ID-to-filename mapping based on vault configuration.
    Open {
        /// ID of note to open
        ///
        /// Must match an existing note in the vault.
        id: String,
    },
    
    /// Display note content to stdout
    ///
    /// Prints the note content without opening an editor. Useful for scripting
    /// or quick content inspection.
    Show {
        /// ID of note to display
        ///
        /// Must match an existing note in the vault.
        id: String,
    },
}

/// Application context that gets passed to command handlers
///
/// This encapsulates the vault state and configuration needed by most operations.
/// Instead of passing individual parameters everywhere, we bundle related data
/// into a context struct.
///
/// Benefits:
/// - Cleaner function signatures
/// - Easy to extend with new configuration
/// - Clear separation between vault state and command logic
struct Context {
    /// Path to the vault root directory
    ///
    /// All note operations happen relative to this path.
    /// Must be an existing directory with a .zettel subdirectory.
    vault_path: PathBuf,
    
    /// ID configuration rules
    ///
    /// Determines how IDs are parsed from filenames and how new IDs are generated.
    /// Loaded from vault configuration or defaults.
    id_config: IdConfig,
}

impl Context {
    /// Create new context with vault path and default configuration
    ///
    /// This initializes the context with sensible defaults. In a more mature version,
    /// this would load configuration from the vault's .zettel/config.toml file.
    ///
    /// Current defaults:
    /// - Fuzzy ID matching (most permissive)
    /// - " - " separator for readability
    /// - ASCII-only IDs for compatibility
    fn new(vault_path: PathBuf) -> Self {
        let id_config = IdConfig {
            match_rule: "fuzzy".to_string(), // Start with fuzzy for ease of use
            separator: " - ".to_string(),
            allow_unicode: false,
        };
        
        Self {
            vault_path,
            id_config,
        }
    }
    
    /// Create an ID manager with vault-specific existence checking
    ///
    /// The ID manager needs to know which IDs already exist to avoid conflicts.
    /// This creates a closure that checks the actual filesystem for existing notes.
    ///
    /// Design note: We use a closure here because the IdManager is generic over
    /// the existence checking function. This allows for different implementations
    /// (filesystem, database, etc.) while keeping the core logic pure.
    fn get_id_manager(&self) -> IdManager<impl Fn(&str) -> bool + '_> {
        let vault_path = &self.vault_path;
        IdManager::new(self.id_config.clone(), move |id: &str| {
            Self::id_exists_in_vault(vault_path, id)
        })
    }
    
    /// Check if a note with the given ID exists in the vault
    ///
    /// This implements the actual existence checking logic. It searches for any
    /// markdown file whose filename starts with the given ID according to the
    /// vault's matching rules.
    ///
    /// Algorithm:
    /// 1. List all .md files in vault directory
    /// 2. For each file, check if filename starts with the ID
    /// 3. Use simple heuristic: ID followed by non-alphanumeric char or end of string
    ///
    /// Note: This is a simplified implementation. A production version might:
    /// - Use the same parsing logic as the IdManager
    /// - Cache results for better performance
    /// - Handle subdirectories
    fn id_exists_in_vault(vault_path: &Path, id: &str) -> bool {
        if !vault_path.exists() {
            return false;
        }
        
        // Look for any markdown file that starts with this ID
        if let Ok(entries) = fs::read_dir(vault_path) {
            for entry in entries.flatten() {
                if let Some(filename) = entry.file_name().to_str() {
                    if filename.ends_with(".md") {
                        let stem = filename.strip_suffix(".md").unwrap_or(filename);
                        if stem.starts_with(id) {
                            // Simple check: if filename starts with ID followed by non-alphanumeric
                            // This catches: "1a2.md", "1a2-title.md", "1a2 - Title.md"
                            // But not: "1a23.md" when looking for "1a2"
                            if stem == id || 
                               (stem.len() > id.len() && 
                                !stem.chars().nth(id.len()).unwrap().is_alphanumeric()) {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        false
    }
    
    /// Get list of all markdown files in the vault, sorted by name
    ///
    /// This is a utility function used by listing and search commands.
    /// Returns full paths to enable further processing.
    ///
    /// Note: Currently only handles files in the vault root. A production version
    /// might recursively search subdirectories or respect ignore patterns.
    fn get_vault_files(&self) -> Vec<PathBuf> {
        let mut files = Vec::new();
        
        if let Ok(entries) = fs::read_dir(&self.vault_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "md") {
                    files.push(path);
                }
            }
        }
        
        files.sort();
        files
    }
    
    /// Get the editor command to use for opening files
    ///
    /// Editor selection follows Unix conventions:
    /// 1. ZETTEL_EDITOR environment variable (zettel-specific)
    /// 2. EDITOR environment variable (standard Unix)
    /// 3. Platform-specific default (vim on Unix, notepad on Windows)
    ///
    /// This allows users to configure different editors for zettel vs general editing.
    fn get_editor() -> String {
        env::var("ZETTEL_EDITOR")
            .or_else(|_| env::var("EDITOR"))
            .unwrap_or_else(|_| {
                if cfg!(windows) {
                    "notepad".to_string()
                } else {
                    "vim".to_string()
                }
            })
    }
}

/// Main entry point - parse arguments and dispatch to appropriate handler
///
/// This function implements the typical CLI application pattern:
/// 1. Parse command-line arguments
/// 2. Handle special cases (like init that doesn't need existing vault)
/// 3. Set up application context
/// 4. Dispatch to appropriate command handler
/// 5. Handle and report errors
///
/// Error handling strategy: Use anyhow for easy error propagation and
/// user-friendly error messages. Commands should return detailed errors
/// that help users understand what went wrong.
fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    // Special case: init command doesn't need an existing vault
    // Handle it separately before setting up context
    if let Commands::Init { path } = cli.command {
        return handle_init(path);
    }
    
    // Determine vault path using precedence: CLI arg > environment > current directory
    // This follows typical Unix tool behavior for configuration
    let vault_path = cli.vault
        .or_else(|| env::var("ZETTEL_VAULT").ok().map(PathBuf::from))
        .unwrap_or_else(|| env::current_dir().unwrap());
    
    // Create application context with vault and configuration
    let ctx = Context::new(vault_path);
    
    // Dispatch to appropriate command handler
    // Each handler is responsible for its own error handling and output formatting
    match cli.command {
        Commands::Id(cmd) => handle_id_command(&ctx, cmd),
        Commands::Note(cmd) => handle_note_command(&ctx, cmd),
        Commands::List { full_paths, json } => handle_list_command(&ctx, full_paths, json),
        Commands::Search { query } => handle_search_command(&ctx, query),
        Commands::Init { .. } => unreachable!(), // Handled above
    }
}

/// Initialize a new zettelkasten vault
///
/// This creates the directory structure and configuration files needed for a new vault.
/// It's designed to be safe (won't overwrite existing files) and provide helpful feedback.
///
/// Created structure:
/// ```
/// vault_directory/
/// ├── .zettel/
/// │   └── config.toml
/// └── (ready for notes)
/// ```
///
/// The .zettel directory serves as a marker that this is a zettel vault and
/// contains configuration files. Similar to .git directories in Git repositories.
fn handle_init(path: Option<PathBuf>) -> anyhow::Result<()> {
    let vault_path = path.unwrap_or_else(|| env::current_dir().unwrap());
    
    // Create directory if it doesn't exist
    fs::create_dir_all(&vault_path)?;
    
    // Create the .zettel metadata directory
    let zettel_dir = vault_path.join(".zettel");
    fs::create_dir_all(&zettel_dir)?;
    
    // Create a simple configuration file with sensible defaults and documentation
    let config_content = r#"# Zettel Configuration
# ID matching: strict, separator, fuzzy
match_rule = "fuzzy"
separator = " - "

# Editor (overrides EDITOR environment variable)
# editor = "code"
"#;
    fs::write(zettel_dir.join("config.toml"), config_content)?;
    
    // Provide helpful feedback and next steps
    println!("✅ Initialized zettel vault at: {}", vault_path.display());
    println!("💡 Try: zettel note create 1 \"My First Note\"");
    
    Ok(())
}

/// Handle ID-related commands (generation, validation, parsing)
///
/// These commands implement the core ID manipulation logic. They're designed to be
/// composable and scriptable - each command outputs exactly what it computes.
///
/// Design principle: Each command does one thing and does it well. Complex workflows
/// can be built by combining these simple operations.
fn handle_id_command(ctx: &Context, cmd: IdCommands) -> anyhow::Result<()> {
    let id_manager = ctx.get_id_manager();
    
    match cmd {
        IdCommands::NextSibling { id } => {
            // Parse the input ID and generate the next sibling
            let current_id = Id::parse(&id)?;
            let next_id = id_manager.next_available_sibling(&current_id)?;
            println!("{}", next_id);
        }
        
        IdCommands::NextChild { id } => {
            // Parse the parent ID and generate the first available child
            let parent_id = Id::parse(&id)?;
            let child_id = id_manager.next_available_child(&parent_id);
            println!("{}", child_id);
        }
        
        IdCommands::Parse { filename } => {
            // Extract ID from filename using vault's matching rules
            if let Some(id) = id_manager.extract_from_filename(&filename) {
                println!("{}", id);
            } else {
                eprintln!("No valid ID found in filename: {}", filename);
                std::process::exit(1);
            }
        }
        
        IdCommands::Validate { id } => {
            // Validate and show detailed information about the ID structure
            match Id::parse(&id) {
                Ok(parsed_id) => {
                    println!("✅ Valid ID: {}", parsed_id);
                    println!("   Depth: {}", parsed_id.depth());
                    println!("   Root: {}", parsed_id.is_root());
                    if let Ok(Some(parent)) = parsed_id.parent() {
                        println!("   Parent: {}", parent);
                    }
                }
                Err(e) => {
                    eprintln!("❌ Invalid ID: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
    
    Ok(())
}

/// Handle note management commands (create, open, show)
///
/// These commands operate at a higher level than ID commands - they manage actual files
/// and content. They coordinate between ID generation, file creation, and editor integration.
fn handle_note_command(ctx: &Context, cmd: NoteCommands) -> anyhow::Result<()> {
    let id_manager = ctx.get_id_manager();
    
    match cmd {
        NoteCommands::Create { id, title, open } => {
            let parsed_id = Id::parse(&id)?;
            
            // Safety check: don't overwrite existing notes
            if id_manager.id_exists(&parsed_id) {
                eprintln!("❌ Note with ID '{}' already exists", id);
                std::process::exit(1);
            }
            
            // Generate filename based on vault configuration
            let filename = if let Some(ref title) = title {
                format!("{} - {}.md", id, title)
            } else {
                format!("{}.md", id)
            };
            
            let note_path = ctx.vault_path.join(&filename);
            
            // Generate initial content - simple format for now
            let content = if let Some(ref title) = title {
                format!("# {}\n\n", title)
            } else {
                format!("# Note {}\n\n", id)
            };
            
            // Create the file atomically
            fs::write(&note_path, content)?;
            
            println!("✅ Created note: {}", note_path.display());
            
            // Optionally open in editor
            if open {
                let editor = Context::get_editor();
                let status = std::process::Command::new(&editor)
                    .arg(&note_path)
                    .status()?;
                
                if !status.success() {
                    eprintln!("⚠️ Editor '{}' exited with error", editor);
                }
            }
        }
        
        NoteCommands::Open { id } => {
            let parsed_id = Id::parse(&id)?;
            
            // Find the note file by searching for matching ID
            let files = ctx.get_vault_files();
            let mut found_file = None;
            
            for file in files {
                if let Some(filename) = file.file_name().and_then(|n| n.to_str()) {
                    if let Some(file_id) = id_manager.extract_from_filename(filename) {
                        if file_id == parsed_id {
                            found_file = Some(file);
                            break;
                        }
                    }
                }
            }
            
            if let Some(file_path) = found_file {
                let editor = Context::get_editor();
                let status = std::process::Command::new(&editor)
                    .arg(&file_path)
                    .status()?;
                
                if !status.success() {
                    eprintln!("⚠️ Editor '{}' exited with error", editor);
                }
            } else {
                eprintln!("❌ No note found with ID: {}", id);
                std::process::exit(1);
            }
        }
        
        NoteCommands::Show { id } => {
            let parsed_id = Id::parse(&id)?;
            
            // Find and display the note content
            let files = ctx.get_vault_files();
            let mut found_file = None;
            
            for file in files {
                if let Some(filename) = file.file_name().and_then(|n| n.to_str()) {
                    if let Some(file_id) = id_manager.extract_from_filename(filename) {
                        if file_id == parsed_id {
                            found_file = Some(file);
                            break;
                        }
                    }
                }
            }
            
            if let Some(file_path) = found_file {
                let content = fs::read_to_string(&file_path)?;
                println!("📄 {}", file_path.display());
                println!("{}", "─".repeat(50));
                println!("{}", content);
            } else {
                eprintln!("❌ No note found with ID: {}", id);
                std::process::exit(1);
            }
        }
    }
    
    Ok(())
}

/// List all notes in the vault with various output formats
///
/// This provides different views of the note collection:
/// - Human-readable: Shows IDs and titles
/// - Machine-readable: JSON output for scripting
/// - Detailed: Full file paths for integration
///
/// The JSON output is designed to be processed by tools like jq for
/// complex filtering and processing workflows.
fn handle_list_command(ctx: &Context, full_paths: bool, json: bool) -> anyhow::Result<()> {
    let files = ctx.get_vault_files();
    let id_manager = ctx.get_id_manager();
    
    if json {
        // Machine-readable output for scripting
        let mut notes = Vec::new();
        for file in files {
            if let Some(filename) = file.file_name().and_then(|n| n.to_str()) {
                if let Some(id) = id_manager.extract_from_filename(filename) {
                    let note_info = serde_json::json!({
                        "id": id.to_string(),
                        "filename": filename,
                        "path": file.display().to_string()
                    });
                    notes.push(note_info);
                }
            }
        }
        println!("{}", serde_json::to_string_pretty(&notes)?);
    } else {
        // Human-readable output
        for file in files {
            if let Some(filename) = file.file_name().and_then(|n| n.to_str()) {
                if let Some(id) = id_manager.extract_from_filename(filename) {
                    if full_paths {
                        println!("{} ({})", id, file.display());
                    } else {
                        // Try to extract title from filename for prettier display
                        let title = if filename.contains(" - ") {
                            filename.split(" - ")
                                .nth(1)
                                .unwrap_or("")
                                .strip_suffix(".md")
                                .unwrap_or("")
                        } else {
                            ""
                        };
                        
                        if title.is_empty() {
                            println!("{}", id);
                        } else {
                            println!("{}: {}", id, title);
                        }
                    }
                }
            }
        }
    }
    
    Ok(())
}

/// Search notes by content or filename
///
/// This implements simple text-based search across all notes in the vault.
/// It searches both filenames and file content using case-insensitive matching.
///
/// Future enhancements might include:
/// - Full-text indexing for better performance
/// - Regular expression support
/// - Search result ranking
/// - Integration with external search tools
fn handle_search_command(ctx: &Context, query: String) -> anyhow::Result<()> {
    let files = ctx.get_vault_files();
    let id_manager = ctx.get_id_manager();
    let query_lower = query.to_lowercase();
    
    println!("🔍 Searching for: {}", query);
    println!();
    
    for file in files {
        if let Some(filename) = file.file_name().and_then(|n| n.to_str()) {
            if let Some(id) = id_manager.extract_from_filename(filename) {
                // Search in filename first (faster)
                if filename.to_lowercase().contains(&query_lower) {
                    println!("📄 {} (filename match)", id);
                    continue;
                }
                
                // Search in content (slower, but more thorough)
                if let Ok(content) = fs::read_to_string(&file) {
                    if content.to_lowercase().contains(&query_lower) {
                        // Try to extract title from first line for better display
                        let title = content.lines()
                            .next()
                            .unwrap_or("")
                            .strip_prefix("# ")
                            .unwrap_or("No title");
                        
                        println!("📄 {}: {}", id, title);
                    }
                }
            }
        }
    }
    
    Ok(())
}
