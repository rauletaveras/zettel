// crates/zettel-cli/src/services/vault.rs - File system operations service
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use zettel_core::id::IdManager;

/// Handles all file system operations and vault interactions
///
/// This is the "data access layer" - it knows HOW to interact with files
/// but doesn't know WHY (that's business logic).
pub struct VaultService {
    vault_path: PathBuf,
}

impl VaultService {
    pub fn new(vault_path: PathBuf) -> Self {
        Self { vault_path }
    }

    /// Check if a note with the given ID exists in the vault
    pub fn id_exists(&self, id: &str) -> bool {
        if !self.vault_path.exists() {
            return false;
        }

        if let Ok(entries) = fs::read_dir(&self.vault_path) {
            for entry in entries.flatten() {
                if let Some(filename) = entry.file_name().to_str() {
                    if filename.ends_with(".md") {
                        let stem = filename.strip_suffix(".md").unwrap_or(filename);
                        if stem.starts_with(id) {
                            // Simple check: ID followed by non-alphanumeric or end of string
                            if stem == id
                                || (stem.len() > id.len()
                                    && !stem.chars().nth(id.len()).unwrap().is_alphanumeric())
                            {
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
    pub fn get_vault_files(&self) -> Vec<PathBuf> {
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

    /// Create a new file with given content
    pub fn create_file(&self, relative_path: &str, content: &str) -> Result<PathBuf> {
        let full_path = self.vault_path.join(relative_path);

        // Create directory if needed
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&full_path, content)?;
        Ok(full_path)
    }

    /// Read file content
    pub fn read_file(&self, path: &Path) -> Result<String> {
        Ok(fs::read_to_string(path)?)
    }

    /// Get the directory where new files should be created
    pub fn get_new_file_directory(&self, _current_file_path: &str) -> &Path {
        // For now, just use vault root - could be more sophisticated
        &self.vault_path
    }

    /// Initialize vault directory structure
    pub fn init_vault(&self) -> Result<()> {
        // Create directory if it doesn't exist
        fs::create_dir_all(&self.vault_path)?;

        // Create the .zettel metadata directory
        let zettel_dir = self.vault_path.join(".zettel");
        fs::create_dir_all(&zettel_dir)?;

        // Create a simple configuration file
        let config_content = r#"# Zettel Configuration
# ID matching: strict, separator, fuzzy
match_rule = "fuzzy"
separator = " - "

# Editor (overrides EDITOR environment variable)
# editor = "code"
"#;
        fs::write(zettel_dir.join("config.toml"), config_content)?;

        Ok(())
    }
}
