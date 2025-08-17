// crates/zettel-cli/src/cli.rs - Command Line Interface Definitions
//
// This module contains pure data structures that define the CLI interface.
// It uses clap's derive API to declaratively specify commands and arguments.
//
// DESIGN PHILOSOPHY:
// - Declarative over imperative: We describe WHAT the interface looks like,
//   not HOW to parse it (clap handles the how)
// - No business logic: This module only defines the shape of commands
// - Documentation as code: Help text is embedded in the struct definitions
// - Type safety: Invalid command combinations are prevented at compile time
//
// RUST LEARNING NOTES:
// - `#[derive(Parser)]` is a procedural macro that generates parsing code
// - `#[command(...)]` attributes configure clap's behavior
// - `#[arg(...)]` attributes configure individual argument parsing
//
// CLAP PATTERNS:
// - Subcommands are represented as enum variants
// - Global options (like --vault) are defined on the main struct
// - Help text comes from doc comments and attribute descriptions
// - Argument types are inferred from struct field types

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Main CLI structure defining global options and subcommands
///
/// This uses clap's derive API for clean, declarative command definition.
/// The structure mirrors typical Unix tools where you have:
/// - Global options (like --vault) that apply to all commands
/// - Subcommands that implement specific functionality
///
/// EXAMPLE USAGE:
/// ```bash
/// zettel --vault ~/notes init                    # Global option before command
/// zettel note create 1 "My First Note"           # Simple command with args
/// zettel list --json | jq '.[] | .id'            # Command option with piping
/// ZETTEL_VAULT=~/work zettel id next-sibling 1   # Environment variable override
/// ```
///
/// DESIGN DECISIONS:
/// - Global --vault option allows working with multiple vaults
/// - Commands are grouped by functionality (id, note, etc.)
/// - Both short (-v) and long (--vault) options for usability
/// - Help text explains the Luhmann ID concept for new users
#[derive(Parser)]
#[command(name = "zettel")]
#[command(about = "A CLI for Luhmann-style Zettelkasten management")]
#[command(version = "0.1.0")]
#[command(long_about = "
A command-line tool for managing a Zettelkasten using Luhmann's ID system.

Luhmann IDs are hierarchical: 1, 1a, 1a1, 1a2, 1b, 2, etc.
Each ID encodes the note's position in a branching structure.

Examples:
  zettel init                           Initialize new vault
  zettel note create 1 \"First Note\"    Create root note
  zettel id next-child 1                Get child ID (1a)
  zettel list --json                    Machine-readable output
")]
pub struct Cli {
    /// Vault directory (overrides ZETTEL_VAULT environment variable)
    ///
    /// This allows users to work with multiple vaults or override the default.
    /// Following Unix conventions: CLI args > environment > defaults
    ///
    /// EXAMPLES:
    /// --vault ~/work-notes    Use specific vault
    /// -v /tmp/test           Short form
    ///
    /// If not specified, checks ZETTEL_VAULT environment variable,
    /// then falls back to current directory.
    #[arg(short, long, global = true)]
    #[arg(help = "Vault directory (overrides ZETTEL_VAULT env var)")]
    pub vault: Option<PathBuf>,

    /// Main command dispatch - each variant corresponds to a major area of functionality
    ///
    /// DESIGN: Start with few commands, add more as needed. Better to have
    /// a small, well-designed CLI than a large, confusing one.
    #[command(subcommand)]
    pub command: Commands,
}

/// Top-level command categories
///
/// This groups related functionality and makes the CLI feel organized.
/// Each category can have its own subcommands for granular control.
///
/// COMMAND PHILOSOPHY:
/// - init: One-time setup (like `git init`)
/// - id: Core ID manipulation (the heart of the system)
/// - note: High-level note operations (user-facing workflow)
/// - list: Discovery and navigation
/// - search: Content-based finding
///
/// Note how we use #[command(subcommand)] for commands that have their own
/// subcommands (id, note) vs direct fields for simple commands (list, search).
#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new vault
    ///
    /// Creates the directory structure and configuration files needed
    /// for a new zettelkasten vault. This is typically the first command
    /// users run when setting up a new knowledge base.
    ///
    /// BEHAVIOR:
    /// - Creates vault directory if it doesn't exist
    /// - Creates .zettel/ subdirectory for metadata
    /// - Writes default configuration file
    /// - Safe: won't overwrite existing files
    ///
    /// EXAMPLES:
    /// zettel init                Initialize in current directory
    /// zettel init ~/notes        Initialize in specific location
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
    ///
    /// ID CONCEPT EXPLANATION:
    /// Luhmann IDs encode hierarchy: 1 -> 1a -> 1a1
    /// - Numbers and letters alternate
    /// - Each level represents conceptual depth
    /// - Siblings share the same prefix (1a, 1b, 1c)
    /// - Children extend the parent ID (1a -> 1a1, 1a2)
    #[command(subcommand)]
    Id(IdCommands),

    /// Note operations (create, open, show)
    ///
    /// High-level note management. These commands coordinate between
    /// ID generation, file creation, and content management.
    ///
    /// WORKFLOW INTEGRATION:
    /// These commands are designed for editor integration and daily use.
    /// They handle the user-facing workflow of creating and managing notes.
    #[command(subcommand)]
    Note(NoteCommands),

    /// List notes in the vault
    ///
    /// Provides different views of the note collection. Supports both
    /// human-readable and machine-readable output formats.
    ///
    /// OUTPUT FORMATS:
    /// - Default: ID and title for human reading
    /// - --json: Structured data for scripting
    /// - --full-paths: Complete file paths for integration
    ///
    /// EXAMPLES:
    /// zettel list                    Human-readable format
    /// zettel list --json | jq       Process with jq
    /// zettel list --full-paths       Show complete paths
    List {
        /// Show full file paths instead of just IDs
        ///
        /// Useful for scripting or when you need to know exact file locations.
        /// Displays complete filesystem paths rather than just note IDs.
        #[arg(long)]
        #[arg(help = "Show complete file paths instead of just IDs")]
        full_paths: bool,

        /// Output as JSON for machine processing
        ///
        /// Enables integration with tools like jq, fzf, or custom scripts.
        /// JSON output includes metadata like creation time, parent relationships, etc.
        ///
        /// EXAMPLE OUTPUT:
        /// [{"id": "1a", "filename": "1a - My Note.md", "path": "/vault/1a - My Note.md"}]
        #[arg(long)]
        #[arg(help = "Output as JSON for machine processing")]
        json: bool,
    },

    /// Search notes by content or title
    ///
    /// Simple text-based search. Future versions might add more sophisticated
    /// search capabilities (full-text indexing, regex, etc.).
    ///
    /// SEARCH STRATEGY:
    /// 1. Search filenames first (fast)
    /// 2. Search file content (thorough but slower)
    /// 3. Case-insensitive substring matching
    ///
    /// EXAMPLES:
    /// zettel search "machine learning"    Find notes about ML
    /// zettel search "TODO"                Find notes with tasks
    Search {
        /// Search query string
        ///
        /// Currently does simple case-insensitive substring matching.
        /// Searches both filenames and file content.
        ///
        /// TIP: Use quotes for multi-word queries: "machine learning"
        query: String,
    },
}

/// ID-specific subcommands
///
/// These implement the core ID manipulation logic that powers the zettelkasten system.
/// Each command does one specific ID operation and outputs the result.
///
/// DESIGN RATIONALE:
/// Fine-grained commands allow for maximum composability. You can build complex
/// workflows by chaining these simple operations. Each command follows the Unix
/// philosophy: do one thing and do it well.
///
/// ID SYSTEM PRIMER:
/// - Luhmann IDs: 1, 1a, 1a1, 1a2, 1b, 1b1, 2, 2a...
/// - Siblings: Same level (1a, 1b, 1c)
/// - Children: One level deeper (1 -> 1a, 1a -> 1a1)
/// - Parent: One level up (1a1 -> 1a -> 1)
#[derive(Subcommand)]
pub enum IdCommands {
    /// Generate next sibling ID
    ///
    /// Sibling = same level in hierarchy. Examples:
    /// - 1 -> 2 (next root note)
    /// - 1a -> 1b (next child of note 1)
    /// - 1a2 -> 1a3 (next grandchild of note 1a)
    ///
    /// This finds the next available ID at the same hierarchical level,
    /// skipping any IDs that already exist in the vault.
    ///
    /// USAGE PATTERN:
    /// ```bash
    /// current_id=$(zettel id parse "current_note.md")
    /// sibling_id=$(zettel id next-sibling "$current_id")
    /// zettel note create "$sibling_id" "Related Note"
    /// ```
    #[command(name = "next-sibling")]
    NextSibling {
        /// Current ID to increment from
        ///
        /// Must be a valid Luhmann-style ID (1, 1a, 1a2, etc.)
        /// The command will generate the next available sibling ID.
        ///
        /// EXAMPLES:
        /// zettel id next-sibling 1      # Output: 2 (or first available)
        /// zettel id next-sibling 1a     # Output: 1b (or first available)
        id: String,
    },

    /// Generate next child ID
    ///
    /// Child = one level deeper in hierarchy. Examples:
    /// - 1 -> 1a (first child of root note)
    /// - 1a -> 1a1 (first grandchild)
    /// - 2b -> 2b1 (child follows alternating pattern)
    ///
    /// This creates the next available child under the given parent,
    /// following Luhmann's alternating number/letter pattern.
    ///
    /// BRANCHING CONCEPT:
    /// Children represent conceptual branching or elaboration.
    /// Use children when the new note is a sub-topic or development
    /// of the parent note's ideas.
    #[command(name = "next-child")]
    NextChild {
        /// Parent ID to create child under
        ///
        /// The child will follow Luhmann's alternating number/letter pattern:
        /// - After number comes letter: 1 -> 1a
        /// - After letter comes number: 1a -> 1a1
        ///
        /// EXAMPLES:
        /// zettel id next-child 1        # Output: 1a (or first available)
        /// zettel id next-child 1a       # Output: 1a1 (or first available)
        id: String,
    },

    /// Parse ID from filename
    ///
    /// Extracts the ID portion from a filename based on the vault's matching rules.
    /// Useful for getting the ID of the currently open file in your editor.
    ///
    /// MATCHING RULES:
    /// - Strict: "1a2.md" -> "1a2"
    /// - Separator: "1a2 - Title.md" -> "1a2"
    /// - Fuzzy: "1a2_anything.md" -> "1a2"
    ///
    /// EDITOR INTEGRATION:
    /// ```bash
    /// # In your editor, get current file's ID
    /// current_id=$(zettel id parse "$CURRENT_FILE")
    /// next_id=$(zettel id next-sibling "$current_id")
    /// ```
    Parse {
        /// Filename to extract ID from
        ///
        /// Can be just the basename (1a2-title.md) or full path.
        /// The command will extract the ID portion according to
        /// the vault's configured matching rules.
        ///
        /// EXAMPLES:
        /// zettel id parse "1a2.md"              # Output: 1a2
        /// zettel id parse "1a2 - My Note.md"    # Output: 1a2
        /// zettel id parse "/path/to/1a2_note.md" # Output: 1a2
        filename: String,
    },

    /// Validate ID format
    ///
    /// Checks if a string is a valid Luhmann-style ID and shows its structure.
    /// Useful for debugging ID generation or validating user input.
    ///
    /// VALIDATION RULES:
    /// - Must start with a number
    /// - Numbers and letters must alternate
    /// - Only lowercase letters allowed
    /// - No special characters or spaces
    ///
    /// OUTPUT INCLUDES:
    /// - Whether ID is valid
    /// - Hierarchical depth
    /// - Parent ID (if any)
    /// - Whether it's a root note
    Validate {
        /// ID string to validate
        ///
        /// Will show detailed information about the ID structure if valid,
        /// or explain what's wrong if invalid.
        ///
        /// EXAMPLES:
        /// zettel id validate 1a2        # Shows: Valid, depth 3, parent 1a
        /// zettel id validate 1A2        # Shows: Invalid, uppercase not allowed
        /// zettel id validate abc        # Shows: Invalid, must start with number
        id: String,
    },
}

/// Note management subcommands
///
/// These operate at a higher level than ID commands - they create actual files,
/// manage content, and handle the user-facing aspects of note management.
///
/// WORKFLOW FOCUS:
/// These commands are designed for daily note-taking workflows. They handle
/// the common operations users perform when building their zettelkasten.
///
/// INTEGRATION DESIGN:
/// Commands support both interactive use and editor integration through
/// options like --open and structured output formats.
#[derive(Subcommand)]
pub enum NoteCommands {
    /// Create a new note with given ID and title
    ///
    /// This is the main note creation command. It handles:
    /// - File creation with proper naming
    /// - Initial content generation
    /// - Template processing (future feature)
    /// - Optional editor launching
    ///
    /// NAMING STRATEGY:
    /// - With title: "1a2 - My Note Title.md"
    /// - Without title: "1a2.md"
    /// - Follows vault's separator configuration
    ///
    /// CONTENT GENERATION:
    /// - Creates markdown heading from title
    /// - Adds blank line for writing
    /// - Future: template system support
    ///
    /// EXAMPLES:
    /// zettel note create 1 "First Note"         # Create and stay in terminal
    /// zettel note create 1a --open             # Create and open in editor
    /// zettel note create 2 "Second" --open     # Create with title and open
    Create {
        /// Note ID (must be valid Luhmann format)
        ///
        /// Should typically be generated using the `id` subcommands to ensure
        /// it fits properly in the hierarchy and doesn't conflict with existing notes.
        ///
        /// SAFETY: Command will refuse to overwrite existing notes.
        ///
        /// EXAMPLES:
        /// 1        Root note
        /// 1a       Child of note 1
        /// 1a2      Grandchild of note 1
        id: String,

        /// Note title (optional)
        ///
        /// If provided, gets added to filename and used as the note's heading.
        /// If omitted, note gets generic title based on ID.
        ///
        /// TITLE PROCESSING:
        /// - Used in filename: "1a - My Title.md"
        /// - Used as markdown heading: "# My Title"
        /// - Future: template variable substitution
        title: Option<String>,

        /// Open note in editor after creation
        ///
        /// Uses $ZETTEL_EDITOR, $EDITOR, or platform default.
        /// Useful for immediate editing after creation.
        ///
        /// EDITOR SELECTION:
        /// 1. ZETTEL_EDITOR environment variable
        /// 2. EDITOR environment variable  
        /// 3. Platform default (vim on Unix, notepad on Windows)
        #[arg(long)]
        #[arg(help = "Open note in editor after creation")]
        open: bool,
    },

    /// Open existing note in editor
    ///
    /// Finds the note file with the given ID and opens it in the configured editor.
    /// Handles the ID-to-filename mapping based on vault configuration.
    ///
    /// SEARCH STRATEGY:
    /// - Scans all .md files in vault
    /// - Extracts ID from each filename
    /// - Opens first matching file
    ///
    /// ERROR HANDLING:
    /// - Clear message if note not found
    /// - Suggests checking vault configuration if extraction fails
    Open {
        /// ID of note to open
        ///
        /// Must match an existing note in the vault.
        /// Uses the same ID extraction rules as other commands.
        ///
        /// EXAMPLES:
        /// zettel note open 1        # Open root note
        /// zettel note open 1a2      # Open specific note in hierarchy
        id: String,
    },

    /// Display note content to stdout
    ///
    /// Prints the note content without opening an editor. Useful for scripting
    /// or quick content inspection.
    ///
    /// OUTPUT FORMAT:
    /// - Shows filename header
    /// - Separator line
    /// - Complete file content
    ///
    /// SCRIPTING USE:
    /// zettel note show 1 | grep "TODO"          # Find tasks
    /// zettel note show 1a | wc -w               # Count words
    Show {
        /// ID of note to display
        ///
        /// Must match an existing note in the vault.
        /// Outputs the complete content to stdout for processing.
        id: String,
    },
}

// CLI DESIGN PRINCIPLES EXPLAINED:
//
// 1. DISCOVERABILITY:
//    Rich help text explains concepts for newcomers.
//    Examples show real usage patterns.
//
// 2. COMPOSABILITY:
//    Commands output simple text that can be piped.
//    Each command does one thing well.
//
// 3. FLEXIBILITY:
//    Global options work with all commands.
//    Multiple output formats (human, JSON).
//
// 4. SAFETY:
//    Commands validate input and refuse destructive operations.
//    Clear error messages guide users to solutions.
//
// 5. CONVENTION:
//    Follows Unix command patterns users expect.
//    Environment variables for configuration.
//
// 6. EXTENSIBILITY:
//    Easy to add new commands or options.
//    Subcommand structure allows logical grouping.
