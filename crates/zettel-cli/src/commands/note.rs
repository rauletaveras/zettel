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
            // Get ID from argument or stdin
            let id_str = crate::stdin::read_input_or_stdin(id.as_deref())?;
            let parsed_id = Id::parse(&id_str)?;

            // Safety check
            if id_manager.id_exists(&parsed_id) {
                eprintln!("❌ Note with ID '{}' already exists", id_str);
                std::process::exit(1);
            }

            // Generate filename based on title
            let filename = match &title {
                Some(t) => format!("{} - {}.md", id_str, t),
                None => format!("{}.md", id_str),
            };

            // Generate initial content
            let content = match &title {
                Some(t) => format!("# {}\n\n", t),
                None => format!("# Note {}\n\n", id_str),
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
            // Get ID from argument or stdin
            let id_str = crate::stdin::read_input_or_stdin(id.as_deref())?;
            let parsed_id = Id::parse(&id_str)?;

            // Find and open the note
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
                eprintln!("❌ No note found with ID: {}", id_str);
                std::process::exit(1);
            }
        }

        NoteCommands::Show { id } => {
            // Get ID from argument or stdin
            let id_str = crate::stdin::read_input_or_stdin(id.as_deref())?;
            let parsed_id = Id::parse(&id_str)?;

            // Find and display the note
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
                eprintln!("❌ No note found with ID: {}", id_str);
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
