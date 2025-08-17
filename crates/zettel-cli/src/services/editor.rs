// crates/zettel-cli/src/services/editor.rs - Editor integration service
use anyhow::Result;
use std::env;
use std::path::Path;
use std::process::Command;

/// Handles text editor operations
///
/// This service manages editor integration - launching editors,
/// determining which editor to use, etc.
pub struct EditorService;

impl EditorService {
    /// Get the editor command to use for opening files
    pub fn get_editor_command() -> String {
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

    /// Open a file in the configured editor
    pub fn open_file(path: &Path) -> Result<()> {
        let editor = Self::get_editor_command();
        let status = Command::new(&editor).arg(path).status()?;

        if !status.success() {
            eprintln!("⚠️ Editor '{}' exited with error", editor);
        }

        Ok(())
    }
}
