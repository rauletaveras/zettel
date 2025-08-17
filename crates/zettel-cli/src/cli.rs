use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Main CLI structure
#[derive(Parser)]
#[command(name = "zettel")]
#[command(about = "A CLI for Luhmann-style Zettelkasten management")]
#[command(version = "0.1.0")]
pub struct Cli {
    /// Vault directory (overrides ZETTEL_VAULT environment variable)
    #[arg(short, long, global = true)]
    pub vault: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Commands,
}

/// Top-level command categories
#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new vault
    Init {
        /// Path to create vault (defaults to current directory)
        path: Option<PathBuf>,
    },

    /// ID operations (generation, validation, parsing)
    #[command(subcommand)]
    Id(IdCommands),

    /// Note operations (create, open, show)
    #[command(subcommand)]
    Note(NoteCommands),

    /// List notes in the vault
    List {
        /// Show full file paths instead of just IDs
        #[arg(long)]
        full_paths: bool,

        /// Output as JSON for machine processing
        #[arg(long)]
        json: bool,
    },

    /// Search notes by content or title
    Search {
        /// Search query string
        query: String,
    },
}

/// ID-specific subcommands
#[derive(Subcommand)]
pub enum IdCommands {
    /// Generate next sibling ID
    NextSibling {
        /// Current ID to increment from
        id: String,
    },

    /// Generate next child ID
    NextChild {
        /// Parent ID to create child under
        id: String,
    },

    /// Parse ID from filename
    Parse {
        /// Filename to extract ID from
        filename: String,
    },

    /// Validate ID format
    Validate {
        /// ID string to validate
        id: String,
    },
}

/// Note management subcommands
#[derive(Subcommand)]
pub enum NoteCommands {
    /// Create a new note with given ID and title
    Create {
        /// Note ID (must be valid Luhmann format)
        id: String,

        /// Note title (optional)
        title: Option<String>,

        /// Open note in editor after creation
        #[arg(long)]
        open: bool,
    },

    /// Open existing note in editor
    Open {
        /// ID of note to open
        id: String,
    },

    /// Display note content to stdout
    Show {
        /// ID of note to display
        id: String,
    },
}
