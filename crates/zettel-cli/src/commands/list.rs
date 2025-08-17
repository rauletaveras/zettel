// crates/zettel-cli/src/commands/list.rs - Note Listing Command
//
// This command provides different views of the note collection with support
// for both human-readable and machine-readable output formats.

use anyhow::Result;
use serde_json;

use crate::context::Context;

/// List all notes in the vault with various output formats
///
/// This provides different views of the note collection:
/// - Human-readable: Shows IDs and titles
/// - Machine-readable: JSON output for scripting
/// - Detailed: Full file paths for integration
pub fn handle(ctx: &Context, full_paths: bool, json: bool) -> Result<()> {
    let files = ctx.vault_service.get_vault_files();
    let id_manager = ctx.get_id_manager();

    if json {
        // Machine-readable output for scripting
        let mut notes = Vec::new();
        for file in files {
            if let Some(filename) = file.file_name().and_then(|n| n.to_str()) {
                if let Some(id) = id_manager.extract_from_filename(filename) {
                    let note_info = serde_json::json!({
                        "id": id.to_string(),
                        "filename": filename,
                        "path": file.display().to_string()
                    });
                    notes.push(note_info);
                }
            }
        }
        println!("{}", serde_json::to_string_pretty(&notes)?);
    } else {
        // Human-readable output
        for file in files {
            if let Some(filename) = file.file_name().and_then(|n| n.to_str()) {
                if let Some(id) = id_manager.extract_from_filename(filename) {
                    if full_paths {
                        println!("{} ({})", id, file.display());
                    } else {
                        // Try to extract title from filename for prettier display
                        let title = if filename.contains(" - ") {
                            filename
                                .split(" - ")
                                .nth(1)
                                .unwrap_or("")
                                .strip_suffix(".md")
                                .unwrap_or("")
                        } else {
                            ""
                        };

                        if title.is_empty() {
                            println!("{}", id);
                        } else {
                            println!("{}: {}", id, title);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
