use anyhow::Result;
use serde_json;
use zettel_core::id::Id;

use crate::context::Context;

/// Information about a note for listing and sorting
#[derive(Debug, Clone)]
struct NoteInfo {
    id: Id,
    filename: String,
    path: String,
    title: Option<String>,
}

/// List all notes in the vault with various output formats
///
/// This provides different views of the note collection:
/// - Human-readable: Shows IDs and titles in hierarchical order
/// - Machine-readable: JSON output for scripting
/// - Detailed: Full file paths for integration
pub fn handle(ctx: &Context, full_paths: bool, json: bool) -> Result<()> {
    let files = ctx.vault_service.get_vault_files();
    let id_manager = ctx.get_id_manager();

    // Collect all notes with their information
    let mut notes = Vec::new();
    for file in files {
        if let Some(filename) = file.file_name().and_then(|n| n.to_str()) {
            if let Some(id) = id_manager.extract_from_filename(filename) {
                // Extract title from filename for prettier display
                let title = if filename.contains(" - ") {
                    filename
                        .split(" - ")
                        .nth(1)
                        .unwrap_or("")
                        .strip_suffix(".md")
                        .unwrap_or("")
                        .to_string()
                } else {
                    String::new()
                };

                notes.push(NoteInfo {
                    id,
                    filename: filename.to_string(),
                    path: file.display().to_string(),
                    title: if title.is_empty() { None } else { Some(title) },
                });
            }
        }
    }

    // Sort notes by hierarchical order
    notes.sort_by(|a, b| a.id.cmp(&b.id));

    if json {
        // Machine-readable output for scripting
        let json_notes: Vec<_> = notes
            .iter()
            .map(|note| {
                serde_json::json!({
                    "id": note.id.to_string(),
                    "filename": note.filename,
                    "path": note.path
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json_notes)?);
    } else {
        // Human-readable output in hierarchical order
        for note in notes {
            if full_paths {
                println!("{} ({})", note.id, note.path);
            } else if let Some(title) = note.title {
                println!("{}: {}", note.id, title);
            } else {
                println!("{}", note.id);
            }
        }
    }

    Ok(())
}
