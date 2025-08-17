// crates/zettel-core/src/config.rs - Configuration System
//
// This module provides the complete configuration schema and loading mechanism
// for the zettel CLI tool. It handles the hierarchy of configuration sources
// and provides a clean interface for all configurable behavior.
//
// CONFIGURATION HIERARCHY (highest to lowest priority):
// 1. Command-line arguments (--vault, etc.)
// 2. Environment variables (ZETTEL_VAULT, ZETTEL_EDITOR, etc.)
// 3. Vault-specific config file (.zettel/config.toml)
// 4. Global config file (~/.config/zettel/config.toml)
// 5. Built-in defaults
//
// DESIGN PRINCIPLES:
// - Comprehensive: Cover all behavior that could reasonably vary between users
// - Hierarchical: Allow both global and vault-specific overrides
// - Extensible: Easy to add new settings without breaking existing configs
// - Validated: Catch configuration errors early with helpful messages
// - Self-documenting: Generated config files include explanatory comments

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors that can occur during configuration loading and validation
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Configuration file not found: {0}")]
    FileNotFound(String),

    #[error("Invalid TOML syntax in {file}: {error}")]
    ParseError { file: String, error: String },

    #[error("Invalid configuration value: {0}")]
    ValidationError(String),

    #[error("I/O error reading config: {0}")]
    IoError(#[from] std::io::Error),
}

/// Result type for configuration operations
pub type ConfigResult<T> = Result<T, ConfigError>;

/// Complete configuration schema for the zettel system
///
/// This encompasses all user-configurable behavior, organized into logical sections.
/// Each section corresponds to a major area of functionality.
///
/// SERIALIZATION NOTES:
/// - Uses serde for TOML serialization/deserialization
/// - Provides defaults for all fields to handle partial config files
/// - Field names match TOML keys for clear mapping
///
/// RUST PATTERNS:
/// - All fields are owned (String, not &str) for easier manipulation
/// - Uses Option<T> for truly optional settings
/// - Provides Default trait for sensible fallbacks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZettelConfig {
    /// Vault-specific settings
    #[serde(default)]
    pub vault: VaultConfig,

    /// ID parsing and generation rules
    #[serde(default)]
    pub id: IdConfig,

    /// Note creation and content settings
    #[serde(default)]
    pub note: NoteConfig,

    /// Template system configuration
    #[serde(default)]
    pub template: TemplateConfig,

    /// Linking behavior settings
    #[serde(default)]
    pub linking: LinkingConfig,

    /// Editor integration settings
    #[serde(default)]
    pub editor: EditorConfig,

    /// Output formatting options
    #[serde(default)]
    pub output: OutputConfig,

    /// Performance and caching settings
    #[serde(default)]
    pub performance: PerformanceConfig,
}

/// Vault-level configuration
///
/// Settings that control how the vault operates as a whole, including
/// file organization and backup behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultConfig {
    /// Default vault path if not specified via CLI or environment
    pub default_path: Option<String>,

    /// Whether to automatically rebuild search index on file changes
    #[serde(default = "default_true")]
    pub auto_index: bool,

    /// Whether to create backup files before destructive operations
    #[serde(default = "default_false")]
    pub backup_on_change: bool,

    /// Directories to exclude from zettel operations (relative to vault root)
    #[serde(default)]
    pub exclude_dirs: Vec<String>,

    /// File patterns to exclude from zettel operations
    #[serde(default)]
    pub exclude_patterns: Vec<String>,
}

/// ID parsing and generation configuration
///
/// Controls how IDs are recognized in filenames and how new IDs are generated.
/// This is the core of the zettelkasten system's organizational structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdConfig {
    /// ID matching rule: "strict", "separator", or "fuzzy"
    ///
    /// - strict: Filename must be exactly the ID (e.g., "1a2.md")
    /// - separator: ID followed by separator then title (e.g., "1a2 - My Note.md")
    /// - fuzzy: ID at start, anything after first non-alphanumeric (e.g., "1a2_note.md")
    #[serde(default = "default_match_rule")]
    pub match_rule: String,

    /// Separator between ID and title in filenames
    ///
    /// Only used when match_rule is "separator" or when creating files with titles.
    /// Can include whitespace for prettier filenames.
    #[serde(default = "default_separator")]
    pub separator: String,

    /// Whether to allow Unicode characters in IDs
    ///
    /// When false, IDs are restricted to ASCII alphanumeric characters.
    /// When true, allows international characters but may cause filesystem issues.
    #[serde(default = "default_false")]
    pub allow_unicode: bool,

    /// Maximum depth for ID hierarchy
    ///
    /// Prevents runaway nesting that could cause performance issues.
    /// Set to 0 for unlimited depth (not recommended).
    #[serde(default = "default_max_depth")]
    pub max_depth: u32,
}

/// Note creation and file naming configuration
///
/// Controls how new notes are created, named, and initially populated with content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteConfig {
    /// Whether to include note title in the filename
    ///
    /// When true: "1a2 - My Note Title.md"
    /// When false: "1a2.md"
    #[serde(default = "default_false")]
    pub add_title: bool,

    /// Whether to add note title as an alias in frontmatter
    ///
    /// Enables title-based search even with ID-only filenames.
    #[serde(default = "default_false")]
    pub add_alias: bool,

    /// File extension for new notes
    #[serde(default = "default_extension")]
    pub extension: String,

    /// Default directory for new notes (relative to vault root)
    ///
    /// If empty, notes are created in the same directory as the current file.
    #[serde(default)]
    pub default_directory: String,

    /// Whether to create notes in date-based subdirectories
    #[serde(default = "default_false")]
    pub use_date_directories: bool,

    /// Date format for directory names (when use_date_directories is true)
    #[serde(default = "default_date_format")]
    pub date_format: String,
}

/// Template system configuration
///
/// Controls whether and how custom templates are used for note creation.
/// Templates allow advanced users to customize note structure beyond basic formatting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateConfig {
    /// Whether to use custom templates instead of built-in formatting
    #[serde(default = "default_false")]
    pub enabled: bool,

    /// Path to template file (relative to vault root)
    ///
    /// Template supports {{title}} and {{link}} placeholders.
    #[serde(default)]
    pub file: String,

    /// Directory containing multiple template files
    ///
    /// If specified, users can choose from multiple templates.
    #[serde(default)]
    pub directory: String,

    /// Default template name when directory is used
    #[serde(default = "default_template_name")]
    pub default_template: String,

    /// Whether template validation requires {{title}} placeholder
    #[serde(default = "default_true")]
    pub require_title: bool,

    /// Whether template validation requires {{link}} placeholder
    #[serde(default = "default_true")]
    pub require_link: bool,
}

/// Bidirectional linking configuration
///
/// Controls the core zettelkasten feature of automatic linking between
/// parent and child notes when the hierarchy is modified.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkingConfig {
    /// Whether to insert link to child in parent when creating children
    #[serde(default = "default_true")]
    pub insert_in_parent: bool,

    /// Whether to insert link to parent in child when creating children
    #[serde(default = "default_true")]
    pub insert_in_child: bool,

    /// Whether to use title as display text in generated links
    ///
    /// When true: [[1a2|My Note Title]] (prettier but can break)
    /// When false: [[1a2]] (always works)
    #[serde(default = "default_false")]
    pub use_title_alias: bool,

    /// Link format template
    ///
    /// Supports placeholders: {id}, {title}, {filename}
    /// Default: "[[{filename}]]" or "[[{filename}|{title}]]" based on use_title_alias
    #[serde(default)]
    pub format: Option<String>,
}

/// Editor integration configuration
///
/// Controls how the CLI integrates with text editors for note editing and creation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorConfig {
    /// Editor command to use (overrides ZETTEL_EDITOR and EDITOR env vars)
    #[serde(default)]
    pub command: Option<String>,

    /// Arguments to pass to editor
    ///
    /// Supports placeholders: {file}, {line}, {col}
    /// Example: ["+{line}:{col}", "{file}"] for vim-style cursor positioning
    #[serde(default)]
    pub args: Vec<String>,

    /// Whether to wait for editor to exit before continuing
    #[serde(default = "default_true")]
    pub wait: bool,

    /// Working directory for editor (relative to vault root)
    #[serde(default)]
    pub working_directory: Option<String>,
}

/// Output formatting configuration
///
/// Controls how command output is formatted for both human and machine consumption.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    /// Default output format: "human", "json", "csv", "xml"
    #[serde(default = "default_output_format")]
    pub default_format: String,

    /// Color output: "auto", "always", "never"
    #[serde(default = "default_color")]
    pub color: String,

    /// Whether to use a pager for long output
    #[serde(default = "default_pager")]
    pub pager: String,

    /// Date format for human-readable output
    #[serde(default = "default_date_format")]
    pub date_format: String,

    /// Whether to show relative dates ("2 days ago") vs absolute dates
    #[serde(default = "default_true")]
    pub relative_dates: bool,
}

/// Performance and caching configuration
///
/// Controls optimizations for large vaults and resource usage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Whether to enable file system caching
    #[serde(default = "default_true")]
    pub cache_enabled: bool,

    /// Maximum cache age in seconds
    #[serde(default = "default_cache_max_age")]
    pub cache_max_age: u64,

    /// Maximum cache size in MB
    #[serde(default = "default_cache_max_size")]
    pub cache_max_size: u64,

    /// Whether to use parallel processing for file operations
    #[serde(default = "default_true")]
    pub parallel_processing: bool,

    /// Maximum number of threads for parallel operations
    #[serde(default)]
    pub max_threads: Option<usize>,
}

/// Configuration loading and management
///
/// Handles the complex logic of loading configuration from multiple sources
/// and merging them according to the priority hierarchy.
pub struct ConfigManager;

impl ConfigManager {
    /// Load complete configuration from all sources
    ///
    /// This implements the configuration hierarchy by loading from multiple
    /// sources and merging them in priority order.
    ///
    /// LOADING STRATEGY:
    /// 1. Start with built-in defaults
    /// 2. Override with global config file (if exists)
    /// 3. Override with vault-specific config (if exists)
    /// 4. Override with environment variables
    /// 5. Override with command-line arguments (handled by clap)
    ///
    /// ERROR HANDLING:
    /// - Missing config files are not errors (use defaults)
    /// - Invalid TOML syntax is an error with helpful context
    /// - Validation errors include suggestions for fixes
    pub fn load_config(vault_path: Option<&Path>) -> ConfigResult<ZettelConfig> {
        // Start with sensible defaults
        let mut config = ZettelConfig::default();

        // Try to load global config file
        if let Some(global_config) = Self::try_load_global_config()? {
            config = Self::merge_configs(config, global_config);
        }

        // Try to load vault-specific config file
        if let Some(vault_path) = vault_path {
            if let Some(vault_config) = Self::try_load_vault_config(vault_path)? {
                config = Self::merge_configs(config, vault_config);
            }
        }

        // Apply environment variable overrides
        Self::apply_env_overrides(&mut config);

        // Validate the final configuration
        Self::validate_config(&config)?;

        Ok(config)
    }

    /// Generate a default configuration file with comments
    ///
    /// Creates a well-documented config file that users can customize.
    /// Includes explanations of each setting and examples of common configurations.
    pub fn generate_default_config() -> String {
        // This would generate a comprehensive TOML file with comments
        // explaining each section and setting. For brevity, showing structure:
        r#"# Zettel Configuration File
#
# This file controls how the zettel CLI tool behaves.
# Lines starting with # are comments and are ignored.

[vault]
# Default vault path if not specified via --vault or ZETTEL_VAULT
# default_path = "~/notes"

# Automatically rebuild search index when files change
auto_index = true

# Create backup files before destructive operations
backup_on_change = false

[id]
# ID matching rule: "strict", "separator", or "fuzzy"
match_rule = "fuzzy"

# Separator between ID and title in filenames
separator = " - "

# Allow Unicode characters in IDs (may cause filesystem issues)
allow_unicode = false

[note]
# Include note title in filename
add_title = false

# Add note title as frontmatter alias
add_alias = false

# File extension for new notes
extension = "md"

[template]
# Use custom template files
enabled = false

# Path to template file (relative to vault root)
# file = "templates/note.md"

[linking]
# Insert link to child in parent when creating children
insert_in_parent = true

# Insert link to parent in child when creating children
insert_in_child = true

# Use title as display text in links
use_title_alias = false

[editor]
# Editor command (overrides ZETTEL_EDITOR and EDITOR env vars)
# command = "helix"

# Arguments to pass to editor (supports {file}, {line}, {col} placeholders)
# args = ["+{line}:{col}"]

[output]
# Default output format: "human", "json", "csv"
default_format = "human"

# Color output: "auto", "always", "never"
color = "auto"

# Use pager for long output: "auto", "always", "never"
pager = "auto"

[performance]
# Enable file system caching
cache_enabled = true

# Maximum cache age in seconds
cache_max_age = 3600

# Use parallel processing for file operations
parallel_processing = true
"#
        .to_string()
    }

    /// Try to load global configuration file
    ///
    /// Looks for config in standard locations following XDG Base Directory spec:
    /// - Linux: ~/.config/zettel/config.toml
    /// - macOS: ~/Library/Application Support/zettel/config.toml
    /// - Windows: %APPDATA%\zettel\config.toml
    fn try_load_global_config() -> ConfigResult<Option<ZettelConfig>> {
        // Implementation would use dirs crate to find config directory
        // and attempt to load config.toml from there
        Ok(None) // Placeholder - returns None if no global config found
    }

    /// Try to load vault-specific configuration file
    ///
    /// Looks for .zettel/config.toml in the vault directory.
    /// This allows per-vault customization of behavior.
    fn try_load_vault_config(vault_path: &Path) -> ConfigResult<Option<ZettelConfig>> {
        let config_path = vault_path.join(".zettel").join("config.toml");

        if !config_path.exists() {
            return Ok(None);
        }

        let config_content =
            std::fs::read_to_string(&config_path).map_err(|e| ConfigError::IoError(e))?;

        let config: ZettelConfig =
            toml::from_str(&config_content).map_err(|e| ConfigError::ParseError {
                file: config_path.display().to_string(),
                error: e.to_string(),
            })?;

        Ok(Some(config))
    }

    /// Merge two configurations, with the second taking priority
    ///
    /// This implements the override behavior where later configs
    /// take precedence over earlier ones.
    fn merge_configs(base: ZettelConfig, override_config: ZettelConfig) -> ZettelConfig {
        // Implementation would merge each field, with override_config taking precedence
        // For now, just return override_config as placeholder
        override_config
    }

    /// Apply environment variable overrides
    ///
    /// Certain settings can be overridden by environment variables:
    /// - ZETTEL_VAULT -> vault.default_path
    /// - ZETTEL_EDITOR -> editor.command
    /// - ZETTEL_MATCH_RULE -> id.match_rule
    /// - etc.
    fn apply_env_overrides(config: &mut ZettelConfig) {
        use std::env;

        if let Ok(vault) = env::var("ZETTEL_VAULT") {
            config.vault.default_path = Some(vault);
        }

        if let Ok(editor) = env::var("ZETTEL_EDITOR") {
            config.editor.command = Some(editor);
        }

        if let Ok(match_rule) = env::var("ZETTEL_MATCH_RULE") {
            config.id.match_rule = match_rule;
        }

        // Add more environment variable mappings as needed
    }

    /// Validate the final configuration for consistency and correctness
    ///
    /// Catches configuration errors that would cause runtime failures
    /// and provides helpful error messages with suggestions for fixes.
    fn validate_config(config: &ZettelConfig) -> ConfigResult<()> {
        // Validate match rule
        match config.id.match_rule.as_str() {
            "strict" | "separator" | "fuzzy" => {}
            _ => {
                return Err(ConfigError::ValidationError(format!(
                    "Invalid match_rule '{}'. Must be one of: strict, separator, fuzzy",
                    config.id.match_rule
                )))
            }
        }

        // Validate separator is not empty when required
        if config.id.match_rule == "separator" && config.id.separator.is_empty() {
            return Err(ConfigError::ValidationError(
                "Separator cannot be empty when match_rule is 'separator'".to_string(),
            ));
        }

        // Validate template configuration
        if config.template.enabled {
            if config.template.file.is_empty() && config.template.directory.is_empty() {
                return Err(ConfigError::ValidationError(
                    "Template file or directory must be specified when templates are enabled"
                        .to_string(),
                ));
            }
        }

        // Validate output format
        match config.output.default_format.as_str() {
            "human" | "json" | "csv" | "xml" => {}
            _ => {
                return Err(ConfigError::ValidationError(format!(
                    "Invalid output format '{}'. Must be one of: human, json, csv, xml",
                    config.output.default_format
                )))
            }
        }

        // Add more validation rules as needed

        Ok(())
    }
}

/// Default value implementations for serde
///
/// These functions provide the default values used when config fields
/// are missing from TOML files. They're separate functions so they can
/// be used both for serde defaults and for documentation.

fn default_true() -> bool {
    true
}
fn default_false() -> bool {
    false
}

fn default_match_rule() -> String {
    "fuzzy".to_string()
}
fn default_separator() -> String {
    " - ".to_string()
}
fn default_extension() -> String {
    "md".to_string()
}
fn default_template_name() -> String {
    "default".to_string()
}
fn default_date_format() -> String {
    "%Y-%m-%d".to_string()
}
fn default_output_format() -> String {
    "human".to_string()
}
fn default_color() -> String {
    "auto".to_string()
}
fn default_pager() -> String {
    "auto".to_string()
}

fn default_max_depth() -> u32 {
    10
}
fn default_cache_max_age() -> u64 {
    3600
}
fn default_cache_max_size() -> u64 {
    100
}

/// Provide sensible defaults for the entire configuration
impl Default for ZettelConfig {
    fn default() -> Self {
        Self {
            vault: VaultConfig::default(),
            id: IdConfig::default(),
            note: NoteConfig::default(),
            template: TemplateConfig::default(),
            linking: LinkingConfig::default(),
            editor: EditorConfig::default(),
            output: OutputConfig::default(),
            performance: PerformanceConfig::default(),
        }
    }
}

// Default implementations for each config section
impl Default for VaultConfig {
    fn default() -> Self {
        Self {
            default_path: None,
            auto_index: true,
            backup_on_change: false,
            exclude_dirs: vec![
                "_layouts".to_string(),
                "templates".to_string(),
                "scripts".to_string(),
            ],
            exclude_patterns: vec![],
        }
    }
}

impl Default for IdConfig {
    fn default() -> Self {
        Self {
            match_rule: default_match_rule(),
            separator: default_separator(),
            allow_unicode: false,
            max_depth: default_max_depth(),
        }
    }
}

impl Default for NoteConfig {
    fn default() -> Self {
        Self {
            add_title: false,
            add_alias: false,
            extension: default_extension(),
            default_directory: String::new(),
            use_date_directories: false,
            date_format: default_date_format(),
        }
    }
}

impl Default for TemplateConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            file: String::new(),
            directory: String::new(),
            default_template: default_template_name(),
            require_title: true,
            require_link: true,
        }
    }
}

impl Default for LinkingConfig {
    fn default() -> Self {
        Self {
            insert_in_parent: true,
            insert_in_child: true,
            use_title_alias: false,
            format: None,
        }
    }
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            command: None,
            args: vec![],
            wait: true,
            working_directory: None,
        }
    }
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            default_format: default_output_format(),
            color: default_color(),
            pager: default_pager(),
            date_format: default_date_format(),
            relative_dates: true,
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            cache_enabled: true,
            cache_max_age: default_cache_max_age(),
            cache_max_size: default_cache_max_size(),
            parallel_processing: true,
            max_threads: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_is_valid() {
        let config = ZettelConfig::default();
        assert!(ConfigManager::validate_config(&config).is_ok());
    }

    #[test]
    fn test_config_serialization() {
        let config = ZettelConfig::default();
        let toml = toml::to_string_pretty(&config).unwrap();
        let parsed: ZettelConfig = toml::from_str(&toml).unwrap();
        // Compare some key fields to ensure round-trip works
        assert_eq!(config.id.match_rule, parsed.id.match_rule);
        assert_eq!(
            config.linking.insert_in_parent,
            parsed.linking.insert_in_parent
        );
    }

    #[test]
    fn test_invalid_match_rule_validation() {
        let mut config = ZettelConfig::default();
        config.id.match_rule = "invalid".to_string();
        assert!(ConfigManager::validate_config(&config).is_err());
    }

    #[test]
    fn test_empty_separator_with_separator_rule() {
        let mut config = ZettelConfig::default();
        config.id.match_rule = "separator".to_string();
        config.id.separator = "".to_string();
        assert!(ConfigManager::validate_config(&config).is_err());
    }
}
