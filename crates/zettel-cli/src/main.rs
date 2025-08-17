// crates/zettel-cli/src/main.rs - Minimal functional CLI (FIXED)

use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use std::fs;
use std::env;
use zettel_core::id::{Id, IdManager, IdConfig};

#[derive(Parser)]
#[command(name = "zettel")]
#[command(about = "A CLI for Luhmann-style Zettelkasten management")]
#[command(version = "0.1.0")]
struct Cli {
    /// Vault directory (overrides ZETTEL_VAULT)
    #[arg(short, long, global = true)]
    vault: Option<PathBuf>,
    
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new vault
    Init {
        /// Path to create vault (defaults to current directory)
        path: Option<PathBuf>,
    },
    
    /// ID operations
    #[command(subcommand)]
    Id(IdCommands),
    
    /// Note operations
    #[command(subcommand)]
    Note(NoteCommands),
    
    /// List notes
    List {
        /// Show full paths
        #[arg(long)]
        full_paths: bool,
        
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    
    /// Simple search
    Search {
        /// Search query
        query: String,
    },
}

#[derive(Subcommand)]
enum IdCommands {
    /// Generate next sibling ID
    NextSibling {
        /// Current ID
        id: String,
    },
    
    /// Generate next child ID
    NextChild {
        /// Parent ID
        id: String,
    },
    
    /// Parse ID from filename
    Parse {
        /// Filename to parse
        filename: String,
    },
    
    /// Validate ID format
    Validate {
        /// ID to validate
        id: String,
    },
}

#[derive(Subcommand)]
enum NoteCommands {
    /// Create a new note
    Create {
        /// Note ID
        id: String,
        
        /// Note title
        title: Option<String>,
        
        /// Open in editor after creation
        #[arg(long)]
        open: bool,
    },
    
    /// Open note in editor
    Open {
        /// Note ID
        id: String,
    },
    
    /// Show note content
    Show {
        /// Note ID
        id: String,
    },
}

struct Context {
    vault_path: PathBuf,
    id_config: IdConfig,
}

impl Context {
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
    
    fn get_id_manager(&self) -> IdManager<impl Fn(&str) -> bool + '_> {
        let vault_path = &self.vault_path;
        IdManager::new(self.id_config.clone(), move |id: &str| {
            Self::id_exists_in_vault(vault_path, id)
        })
    }
    
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

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    // Handle init command specially (doesn't need existing vault)
    if let Commands::Init { path } = cli.command {
        return handle_init(path);
    }
    
    // Get vault path
    let vault_path = cli.vault
        .or_else(|| env::var("ZETTEL_VAULT").ok().map(PathBuf::from))
        .unwrap_or_else(|| env::current_dir().unwrap());
    
    let ctx = Context::new(vault_path);
    
    match cli.command {
        Commands::Id(cmd) => handle_id_command(&ctx, cmd),
        Commands::Note(cmd) => handle_note_command(&ctx, cmd),
        Commands::List { full_paths, json } => handle_list_command(&ctx, full_paths, json),
        Commands::Search { query } => handle_search_command(&ctx, query),
        Commands::Init { .. } => unreachable!(), // Handled above
    }
}

fn handle_init(path: Option<PathBuf>) -> anyhow::Result<()> {
    let vault_path = path.unwrap_or_else(|| env::current_dir().unwrap());
    
    // Create directory if it doesn't exist
    fs::create_dir_all(&vault_path)?;
    
    // Create a simple .zettel directory for metadata
    let zettel_dir = vault_path.join(".zettel");
    fs::create_dir_all(&zettel_dir)?;
    
    // Create a simple config file
    let config_content = r#"# Zettel Configuration
# ID matching: strict, separator, fuzzy
match_rule = "fuzzy"
separator = " - "

# Editor (overrides EDITOR environment variable)
# editor = "code"
"#;
    fs::write(zettel_dir.join("config.toml"), config_content)?;
    
    println!("✅ Initialized zettel vault at: {}", vault_path.display());
    println!("💡 Try: zettel note create 1 \"My First Note\"");
    
    Ok(())
}

fn handle_id_command(ctx: &Context, cmd: IdCommands) -> anyhow::Result<()> {
    let id_manager = ctx.get_id_manager();
    
    match cmd {
        IdCommands::NextSibling { id } => {
            let current_id = Id::parse(&id)?;
            let next_id = id_manager.next_available_sibling(&current_id)?;
            println!("{}", next_id);
        }
        
        IdCommands::NextChild { id } => {
            let parent_id = Id::parse(&id)?;
            let child_id = id_manager.next_available_child(&parent_id);
            println!("{}", child_id);
        }
        
        IdCommands::Parse { filename } => {
            if let Some(id) = id_manager.extract_from_filename(&filename) {
                println!("{}", id);
            } else {
                eprintln!("No valid ID found in filename: {}", filename);
                std::process::exit(1);
            }
        }
        
        IdCommands::Validate { id } => {
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

fn handle_note_command(ctx: &Context, cmd: NoteCommands) -> anyhow::Result<()> {
    let id_manager = ctx.get_id_manager();
    
    match cmd {
        NoteCommands::Create { id, title, open } => {
            let parsed_id = Id::parse(&id)?;
            
            // Check if note already exists
            if id_manager.id_exists(&parsed_id) {
                eprintln!("❌ Note with ID '{}' already exists", id);
                std::process::exit(1);
            }
            
            // Generate filename
            let filename = if let Some(ref title) = title {
                format!("{} - {}.md", id, title)
            } else {
                format!("{}.md", id)
            };
            
            let note_path = ctx.vault_path.join(&filename);
            
            // Generate content
            let content = if let Some(ref title) = title {
                format!("# {}\n\n", title)
            } else {
                format!("# Note {}\n\n", id)
            };
            
            // Create the file
            fs::write(&note_path, content)?;
            
            println!("✅ Created note: {}", note_path.display());
            
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
            
            // Find the note file
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
            
            // Find and display the note
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

fn handle_list_command(ctx: &Context, full_paths: bool, json: bool) -> anyhow::Result<()> {
    let files = ctx.get_vault_files();
    let id_manager = ctx.get_id_manager();
    
    if json {
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
        for file in files {
            if let Some(filename) = file.file_name().and_then(|n| n.to_str()) {
                if let Some(id) = id_manager.extract_from_filename(filename) {
                    if full_paths {
                        println!("{} ({})", id, file.display());
                    } else {
                        // Try to extract title from filename
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

fn handle_search_command(ctx: &Context, query: String) -> anyhow::Result<()> {
    let files = ctx.get_vault_files();
    let id_manager = ctx.get_id_manager();
    let query_lower = query.to_lowercase();
    
    println!("🔍 Searching for: {}", query);
    println!();
    
    for file in files {
        if let Some(filename) = file.file_name().and_then(|n| n.to_str()) {
            if let Some(id) = id_manager.extract_from_filename(filename) {
                // Search in filename
                if filename.to_lowercase().contains(&query_lower) {
                    println!("📄 {} (filename match)", id);
                    continue;
                }
                
                // Search in content
                if let Ok(content) = fs::read_to_string(&file) {
                    if content.to_lowercase().contains(&query_lower) {
                        // Try to extract title from first line
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
