// crates/zettel-cli/src/commands/init.rs - Vault initialization command
use anyhow::Result;
use std::env;
use std::path::PathBuf;

use crate::services::VaultService;

pub fn handle(path: Option<PathBuf>) -> Result<()> {
    let vault_path = path.unwrap_or_else(|| env::current_dir().unwrap());
    let vault_service = VaultService::new(vault_path.clone());

    vault_service.init_vault()?;

    println!("✅ Initialized zettel vault at: {}", vault_path.display());
    println!("💡 Try: zettel note create 1 \"My First Note\"");

    Ok(())
}
