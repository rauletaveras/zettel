// crates/zettel-cli/src/context.rs - Application Context and Dependency Injection
//
// This module implements the application context pattern, which is Rust's approach
// to dependency injection. Instead of having global state or services that create
// their own dependencies, we bundle related services and configuration into a
// context object that gets passed to command handlers.
//
// DESIGN PATTERNS:
// - Dependency Injection: Services are provided to commands, not created by them
// - Service Locator: Context acts as a registry of available services
// - Configuration Management: Centralized configuration loading and validation
// - Resource Management: Handles vault paths, file access, etc.
//
// BENEFITS OF THIS APPROACH:
// - Testability: Mock services can be injected for testing
// - Maintainability: Changes to services don't require changing all commands
// - Configuration: Single place to handle all app-wide settings
// - Resource safety: Ensures vault exists before commands try to use it

use anyhow::{Context as AnyhowContext, Result};
use std::env;
use std::path::{Path, PathBuf};
use zettel_core::config::{ConfigManager, ZettelConfig};
use zettel_core::id::IdManager;

use crate::services::VaultService;

/// Application context that gets passed to command handlers
///
/// This struct acts as a "dependency injection container" - it provides
/// all the services and configuration that commands need to do their work.
/// This pattern is common in larger applications to avoid tight coupling
/// between components.
///
/// ARCHITECTURE BENEFITS:
/// - Commands don't need to know how to create services
/// - Services can be mocked for testing
/// - Configuration is centralized and consistent
/// - Resource initialization happens once, early in the application lifecycle
///
/// RUST PATTERNS:
/// - Composition over inheritance: Context contains services rather than extending them
/// - Explicit dependencies: Commands declare what they need via function parameters
/// - Lifetime management: Context owns services and lends them to commands
///
/// EXAMPLE USAGE:
/// ```rust
/// let ctx = Context::new(Some("/path/to/vault".into()))?;
/// let id_manager = ctx.get_id_manager();
/// let files = ctx.vault_service.get_vault_files();
/// ```
pub struct Context {
    /// File system operations service
    ///
    /// Handles all vault file operations - reading, writing, listing files.
    /// Separated into a service to make it easy to mock for testing and
    /// to centralize file I/O error handling.
    pub vault_service: VaultService,

    /// Path to the vault root directory
    ///
    /// This is the "working directory" for all zettelkasten operations.
    /// All note file paths are relative to this directory.
    /// Private because we provide accessor method for controlled access.
    vault_path: PathBuf,

    /// ID configuration rules
    ///
    /// Determines how IDs are parsed from filenames and how new IDs are generated.
    /// Currently uses defaults, but in future will be loaded from vault config.
    /// Private because it's used internally to create IdManager instances.
    config: ZettelConfig,
}

impl Context {
    /// Create new context with vault path and configuration
    ///
    /// Now loads configuration from the full hierarchy:
    /// defaults -> global config -> vault config -> env vars
    pub fn new(vault_path: Option<PathBuf>) -> Result<Self> {
        // Determine vault path using configuration hierarchy
        let vault_path = vault_path
            .or_else(|| env::var("ZETTEL_VAULT").ok().map(PathBuf::from))
            .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        // Validate that the vault path exists and is accessible
        if !vault_path.exists() {
            return Err(anyhow::anyhow!(
                "Vault directory does not exist: {}\n\nTry:\n  zettel init {}",
                vault_path.display(),
                vault_path.display()
            ));
        }

        if !vault_path.is_dir() {
            return Err(anyhow::anyhow!(
                "Vault path is not a directory: {}",
                vault_path.display()
            ));
        }

        // Load complete configuration from all sources
        let config = ConfigManager::load_config(Some(&vault_path)).with_context(|| {
            format!("Failed to load configuration from {}", vault_path.display())
        })?;

        // Initialize vault service
        let vault_service = VaultService::new(vault_path.clone());

        Ok(Self {
            vault_service,
            vault_path,
            config,
        })
    }

    /// Create an ID manager with vault-specific existence checking
    ///
    /// Now uses the comprehensive ID configuration instead of hardcoded rules.
    pub fn get_id_manager(&self) -> IdManager<impl Fn(&str) -> bool + '_> {
        IdManager::new(self.config.id.clone(), |id: &str| {
            self.vault_service.id_exists(id)
        })
    }

    /// Get vault path for commands that need filesystem operations
    pub fn vault_path(&self) -> &Path {
        &self.vault_path
    }

    /// Get ID configuration for commands that need parsing rules
    pub fn id_config(&self) -> &zettel_core::config::IdConfig {
        &self.config.id
    }

    /// Get complete configuration for commands that need access to all settings
    pub fn config(&self) -> &ZettelConfig {
        &self.config
    }

    /// Validate that this looks like a zettelkasten vault
    #[allow(dead_code)]
    pub fn validate_vault(&self) -> Result<()> {
        // TODO: Implement vault structure validation
        Ok(())
    }
}
// CONTEXT PATTERN BENEFITS EXPLAINED:
//
// 1. DEPENDENCY INJECTION:
//    Commands receive what they need rather than creating it themselves.
//    This makes commands testable and reduces coupling.
//
// 2. CONFIGURATION MANAGEMENT:
//    Single place to handle environment variables, config files, defaults.
//    Changes to config loading don't affect command implementations.
//
// 3. RESOURCE MANAGEMENT:
//    Context ensures vault exists and is valid before commands run.
//    Prevents commands from having to handle missing vault errors.
//
// 4. SERVICE DISCOVERY:
//    Commands can get the services they need through well-defined methods.
//    New services can be added without changing existing command signatures.
//
// 5. ERROR HANDLING:
//    Configuration and setup errors are caught early, with helpful messages.
//    Commands can focus on their core logic rather than environment validation.
//
// 6. TESTABILITY:
//    Easy to create test contexts with mock services or temporary directories.
//    Commands can be tested in isolation without setting up real file systems.
//
// RUST-SPECIFIC BENEFITS:
//
// 1. LIFETIME SAFETY:
//    Compiler ensures Context lives as long as any services borrowed from it.
//    No risk of dangling pointers or use-after-free.
//
// 2. ZERO-COST ABSTRACTIONS:
//    Service access through Context compiles to direct field access.
//    No runtime overhead compared to global variables.
//
// 3. THREAD SAFETY:
//    Context can be made thread-safe by using Arc<Context> for sharing.
//    Services can use interior mutability patterns if needed.
//
// COMPARISON TO OTHER LANGUAGES:
//
// - Java Spring: Similar dependency injection, but runtime configuration
// - C# DI Container: Similar service locator pattern
// - Python: Similar to passing config objects, but with compile-time guarantees
// - Go: Similar to context.Context pattern for request-scoped data
