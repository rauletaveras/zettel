use anyhow::Result;
use std::env;
use std::path::{Path, PathBuf};
use zettel_core::id::{IdConfig, IdManager};

use crate::services::VaultService;

/// Application context that gets passed to command handlers
///
/// This encapsulates vault state and provides high-level services
/// to command handlers. It's the "dependency injection container"
/// for the CLI application.
pub struct Context {
    pub vault_service: VaultService,
    vault_path: PathBuf,
    id_config: IdConfig,
}

impl Context {
    /// Create new context with vault path and configuration
    pub fn new(vault_path: Option<PathBuf>) -> Result<Self> {
        // Determine vault path using precedence: CLI arg > environment > current directory
        let vault_path = vault_path
            .or_else(|| env::var("ZETTEL_VAULT").ok().map(PathBuf::from))
            .unwrap_or_else(|| env::current_dir().unwrap());

        // Load configuration (for now, use defaults - later read from .zettel/config.toml)
        let id_config = IdConfig {
            match_rule: "fuzzy".to_string(),
            separator: " - ".to_string(),
            allow_unicode: false,
        };

        let vault_service = VaultService::new(vault_path.clone());

        Ok(Self {
            vault_service,
            vault_path,
            id_config,
        })
    }

    /// Create an ID manager with vault-specific existence checking
    pub fn get_id_manager(&self) -> IdManager<impl Fn(&str) -> bool + '_> {
        IdManager::new(self.id_config.clone(), |id: &str| {
            self.vault_service.id_exists(id)
        })
    }

    /// Get vault path
    pub fn vault_path(&self) -> &Path {
        &self.vault_path
    }

    /// Get ID configuration
    pub fn id_config(&self) -> &IdConfig {
        &self.id_config
    }
}
