// crates/zettel-cli/src/commands/note.rs - Note Management Commands
//
// These commands handle the high-level workflow of creating and managing notes.
// They coordinate between ID generation, file creation, and editor integration.

use anyhow::Result;
use zettel_core::id::Id;

use crate::cli::NoteCommands;
use crate::context::Context;
use crate::services::EditorService;

/// Handle note management commands
///
/// These commands operate at a higher level than ID commands - they create actual files,
/// manage content, and handle the user-facing aspects of note management.
pub fn handle(ctx: &Context, cmd: NoteCommands) -> Result<()> {
    let id_manager = ctx.get_id_manager();

    match cmd {
        NoteCommands::Create { id, title, open } => {
            let parsed_id = Id::parse(&id)?;

            // Safety check: don't overwrite existing notes
            if id_manager.id_exists(&parsed_id) {
                eprintln!("❌ Note with ID '{}' already exists", id);
                std::process::exit(1);
            }

            // Generate filename based on title
            let filename = if let Some(ref title) = title {
                format!("{} - {}.md", id, title)
            } else {
                format!("{}.md", id)
            };

            // Generate initial content
            let content = if let Some(ref title) = title {
                format!("# {}\n\n", title)
            } else {
                format!("# Note {}\n\n", id)
            };

            // Create the file
            let note_path = ctx.vault_service.create_file(&filename, &content)?;

            println!("✅ Created note: {}", note_path.display());

            // Optionally open in editor
            if open {
                EditorService::open_file(&note_path, Some(&ctx.config().editor))?;
            }
        }

        NoteCommands::Open { id } => {
            let parsed_id = Id::parse(&id)?;

            // Find the note file by searching for matching ID
            let files = ctx.vault_service.get_vault_files();
            let mut found_file = None;

            for file in files {
                if let Some(filename) = file.file_name().and_then(|n| n.to_str()) {
                    if let Some(file_id) = id_manager.extract_from_filename(filename) {
                        if file_id == parsed_id {
                            found_file = Some(file);
                            break;
                        }
                    }
                }
            }

            if let Some(file_path) = found_file {
                EditorService::open_file(&file_path, Some(&ctx.config().editor))?;
            } else {
                eprintln!("❌ No note found with ID: {}", id);
                std::process::exit(1);
            }
        }

        NoteCommands::Show { id } => {
            let parsed_id = Id::parse(&id)?;

            // Find and display the note content
            let files = ctx.vault_service.get_vault_files();
            let mut found_file = None;

            for file in files {
                if let Some(filename) = file.file_name().and_then(|n| n.to_str()) {
                    if let Some(file_id) = id_manager.extract_from_filename(filename) {
                        if file_id == parsed_id {
                            found_file = Some(file);
                            break;
                        }
                    }
                }
            }

            if let Some(file_path) = found_file {
                let content = ctx.vault_service.read_file(&file_path)?;
                println!("📄 {}", file_path.display());
                println!("{}", "─".repeat(50));
                println!("{}", content);
            } else {
                eprintln!("❌ No note found with ID: {}", id);
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
