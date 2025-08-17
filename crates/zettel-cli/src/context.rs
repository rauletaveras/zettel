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
use zettel_core::id::{IdConfig, IdManager};

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
    id_config: IdConfig,
}

impl Context {
    /// Create new context with vault path and configuration
    ///
    /// This is the main entry point for setting up the application context.
    /// It handles the configuration resolution hierarchy and validates that
    /// the vault exists and is accessible.
    ///
    /// CONFIGURATION RESOLUTION ORDER:
    /// 1. CLI argument (--vault /path)
    /// 2. Environment variable (ZETTEL_VAULT=/path)
    /// 3. Current working directory (fallback)
    ///
    /// ERROR HANDLING:
    /// - Returns error if vault path doesn't exist
    /// - Returns error if vault isn't readable
    /// - Provides helpful context in error messages
    ///
    /// FUTURE ENHANCEMENTS:
    /// - Load configuration from .zettel/config.toml
    /// - Validate vault structure (.zettel directory exists)
    /// - Support vault format migrations
    ///
    /// EXAMPLES:
    /// ```rust
    /// let ctx = Context::new(None)?;                          // Use environment/cwd
    /// let ctx = Context::new(Some("/home/user/notes".into()))?; // Explicit path
    /// ```
    pub fn new(vault_path: Option<PathBuf>) -> Result<Self> {
        // Determine vault path using configuration hierarchy
        // This follows Unix tool conventions: explicit args > env vars > defaults
        let vault_path = vault_path
            .or_else(|| {
                // Try environment variable, converting to PathBuf if present
                env::var("ZETTEL_VAULT").ok().map(PathBuf::from)
            })
            .unwrap_or_else(|| {
                // Fallback to current directory - safe because we validate below
                env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
            });

        // Validate that the vault path exists and is accessible
        // This provides early error detection rather than failing later
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

        // Load ID configuration - currently uses defaults
        // TODO: Load from .zettel/config.toml when that file exists
        let id_config = Self::load_id_config(&vault_path).with_context(|| {
            format!("Failed to load configuration from {}", vault_path.display())
        })?;

        // Initialize vault service with validated path
        let vault_service = VaultService::new(vault_path.clone());

        Ok(Self {
            vault_service,
            vault_path,
            id_config,
        })
    }

    /// Load ID configuration from vault or use defaults
    ///
    /// This function implements the configuration loading strategy.
    /// Currently returns sensible defaults, but is designed to be extended
    /// to read from .zettel/config.toml in the future.
    ///
    /// CONFIGURATION STRATEGY:
    /// - Start with conservative defaults that work for most users
    /// - Override with vault-specific settings if they exist
    /// - Validate configuration for consistency
    ///
    /// DEFAULT CONFIGURATION PHILOSOPHY:
    /// - "fuzzy" matching: Most permissive for filename patterns
    /// - " - " separator: Readable and common in note-taking
    /// - ASCII-only: Maximum compatibility across systems
    ///
    /// FUTURE CONFIG FILE FORMAT:
    /// ```toml
    /// [id]
    /// match_rule = "fuzzy"
    /// separator = " - "
    /// allow_unicode = false
    ///
    /// [editor]
    /// command = "helix"
    /// args = ["+{line}:{col}"]
    /// ```
    fn load_id_config(_vault_path: &Path) -> Result<IdConfig> {
        // TODO: Read from .zettel/config.toml if it exists
        // For now, return sensible defaults that work for most users

        Ok(IdConfig {
            // Fuzzy matching is most permissive - works with various filename patterns
            // "1a2.md", "1a2-title.md", "1a2 - My Note.md" all match
            match_rule: "fuzzy".to_string(),

            // Common separator that's readable and doesn't conflict with shell
            separator: " - ".to_string(),

            // ASCII-only for maximum compatibility across filesystems
            allow_unicode: false,
        })
    }

    /// Create an ID manager with vault-specific existence checking
    ///
    /// The ID manager needs to know which IDs already exist to avoid conflicts
    /// when generating new IDs. This method creates an IdManager configured
    /// with a closure that checks the actual filesystem.
    ///
    /// DESIGN PATTERN: Dependency Injection with Closures
    /// Rather than having IdManager directly depend on file system operations,
    /// we inject a function that can check ID existence. This makes IdManager
    /// testable and keeps it pure (no side effects).
    ///
    /// CLOSURE EXPLANATION:
    /// The `move |id: &str|` creates a closure that captures `self` by reference.
    /// This closure implements the `Fn(&str) -> bool` trait that IdManager expects.
    /// The closure "closes over" the vault_service, giving IdManager access to
    /// file operations without directly coupling them.
    ///
    /// LIFETIME CONSIDERATIONS:
    /// The returned IdManager borrows from self (hence the lifetime parameter).
    /// This ensures the Context lives as long as any IdManager created from it.
    ///
    /// USAGE PATTERN:
    /// ```rust
    /// let id_manager = ctx.get_id_manager();
    /// let next_id = id_manager.next_available_sibling(&current_id)?;
    /// ```
    pub fn get_id_manager(&self) -> IdManager<impl Fn(&str) -> bool + '_> {
        IdManager::new(
            self.id_config.clone(),
            // Closure that captures vault_service and checks ID existence
            // The `move` keyword isn't needed here because we're borrowing &self
            |id: &str| self.vault_service.id_exists(id),
        )
    }

    /// Get vault path for commands that need filesystem operations
    ///
    /// Provides read-only access to the vault path. Commands might need this
    /// for operations like determining where to create new files or for
    /// displaying path information to users.
    ///
    /// DESIGN: Encapsulation
    /// Rather than making vault_path public, we provide controlled access.
    /// This prevents commands from accidentally modifying the path and
    /// makes it clear what data commands are accessing.
    pub fn vault_path(&self) -> &Path {
        &self.vault_path
    }

    /// Get ID configuration for commands that need parsing rules
    ///
    /// Some commands might need direct access to ID configuration,
    /// for example to display current settings or to validate user input
    /// against the configured rules.
    ///
    /// USAGE EXAMPLE:
    /// ```rust
    /// println!("ID matching rule: {}", ctx.id_config().match_rule);
    /// ```
    pub fn id_config(&self) -> &IdConfig {
        &self.id_config
    }

    /// Validate that this looks like a zettelkasten vault
    ///
    /// Future enhancement: Check for .zettel directory and valid configuration.
    /// This would provide better error messages when users accidentally run
    /// commands in the wrong directory.
    ///
    /// VALIDATION CHECKS (future):
    /// - .zettel directory exists
    /// - config.toml is valid
    /// - No conflicting note formats
    /// - Reasonable number of markdown files
    #[allow(dead_code)]
    pub fn validate_vault(&self) -> Result<()> {
        // TODO: Implement vault structure validation
        // For now, just check that it's a directory (already done in new())
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
