// crates/zettel-cli/src/commands/init.rs - Vault Initialization Command
//
// The init command is special because it doesn't require an existing vault.
// It creates the vault structure from scratch, so it can't use the normal
// Context-based approach that assumes a vault already exists.

use anyhow::Result;
use std::env;
use std::path::PathBuf;

use crate::services::VaultService;

/// Initialize a new zettelkasten vault
///
/// This command creates the directory structure and configuration files needed
/// for a new vault. It's designed to be safe (won't overwrite existing files)
/// and provide helpful feedback about what was created.
///
/// CREATED STRUCTURE:
/// ```
/// vault_directory/
/// ├── .zettel/
/// │   └── config.toml
/// └── (ready for notes)
/// ```
///
/// SAFETY FEATURES:
/// - Creates directories only if they don't exist
/// - Won't overwrite existing configuration files
/// - Provides clear feedback about what was created
/// - Suggests next steps for the user
///
/// DESIGN PHILOSOPHY:
/// - Minimal setup: Only create what's absolutely necessary
/// - Self-documenting: Config file includes explanatory comments
/// - User guidance: Output suggests logical next steps
///
/// EXAMPLES:
/// ```bash
/// zettel init                    # Initialize in current directory
/// zettel init ~/my-notes         # Initialize in specific location
/// mkdir new-vault && cd new-vault && zettel init  # Common pattern
/// ```
pub fn handle(path: Option<PathBuf>) -> Result<()> {
    // Determine target directory: explicit path or current directory
    let vault_path = path.unwrap_or_else(|| env::current_dir().unwrap());

    // Create vault service and initialize directory structure
    let vault_service = VaultService::new(vault_path.clone());
    vault_service.init_vault()?;

    // Provide user feedback and guidance for next steps
    println!("✅ Initialized zettel vault at: {}", vault_path.display());
    println!("📁 Created .zettel/ directory with configuration");
    println!();
    println!("💡 Next steps:");
    println!("   zettel note create 1 \"My First Note\"");
    println!("   zettel list");
    println!("   zettel --help");

    Ok(())
}
