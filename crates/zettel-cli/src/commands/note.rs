// crates/zettel-cli/src/commands/note.rs - Note Management Commands
//
// These commands handle the high-level workflow of creating and managing notes.
// They coordinate between ID generation, file creation, and editor integration.

use anyhow::Result;
use zettel_core::id::Id;
use zettel_core::template::TemplateService;

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

            // Generate filename based on configuration
            let filename = generate_filename(&id_str, &title, ctx);

            // Generate note content using template system
            let content = generate_note_content(&title, ctx)?;

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

/// Generate filename based on configuration and optional title
///
/// Business rules for filename generation:
/// - Always starts with ID
/// - Includes title if configured and provided
/// - Uses configured separator between ID and title
/// - Always ends with .md extension
fn generate_filename(id: &str, title: &Option<String>, ctx: &Context) -> String {
    let config = ctx.config();

    match title {
        Some(t) if config.note.add_title => {
            format!("{}{}{}.md", id, config.id.separator, t)
        }
        _ => format!("{}.md", id),
    }
}

/// Generate note content using template system or built-in format
///
/// This implements the core content generation logic:
/// 1. Check if templates are enabled and configured
/// 2. If using templates: read, validate, and process template
/// 3. If not using templates: use built-in format
/// 4. Handle all error cases with helpful messages
fn generate_note_content(title: &Option<String>, ctx: &Context) -> Result<String> {
    let config = ctx.config();
    let title_str = title.as_deref().unwrap_or("");

    // For now, we don't have parent linking in the simple create command
    // This would be added when implementing bidirectional linking
    let backlink_content = "";

    if TemplateService::should_use_template(&config.template) {
        // Template mode: read and process template file
        generate_template_content(title_str, backlink_content, ctx)
    } else {
        // Built-in mode: use standard format
        Ok(TemplateService::generate_content(
            None,
            title_str,
            backlink_content,
        ))
    }
}

/// Generate content using template file
///
/// Handles the complete template workflow:
/// 1. Resolve template file path from configuration
/// 2. Read template content from disk
/// 3. Validate template against requirements
/// 4. Generate final content with placeholder substitution
fn generate_template_content(title: &str, backlink: &str, ctx: &Context) -> Result<String> {
    let config = &ctx.config().template;

    // Step 1: Resolve template file path
    let template_path = TemplateService::resolve_template_path(config)
        .map_err(|e| anyhow::anyhow!("Template configuration error: {}", e))?;

    // Step 2: Read template content from disk
    let template_content = ctx
        .vault_service
        .read_template_file(&template_path)
        .map_err(|e| anyhow::anyhow!("Failed to read template: {}", e))?;

    // Step 3: Validate template content
    let validation_result = TemplateService::validate_template(&template_content, config);
    if !validation_result.valid {
        return Err(anyhow::anyhow!(
            "Template validation failed: {}",
            validation_result
                .message
                .unwrap_or_else(|| "Unknown validation error".to_string())
        ));
    }

    // Step 4: Generate final content
    Ok(TemplateService::generate_content(
        Some(&template_content),
        title,
        backlink,
    ))
}
