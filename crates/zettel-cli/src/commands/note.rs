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

            // Determine parent relationships for bidirectional linking
            let parent_info = determine_parent_info(&parsed_id, ctx)?;

            // Generate note content with parent link (if enabled)
            let content = generate_note_content(&title, &parent_info, ctx)?;

            // Create the file
            let note_path = ctx.vault_service.create_file(&filename, &content)?;
            println!("✅ Created note: {}", note_path.display());

            // Insert child link into parent file (if enabled and parent exists)
            if let Some(ref parent) = parent_info {
                insert_child_link_in_parent(&id_str, &title, parent, ctx)?;
            }

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
fn generate_note_content(
    title: &Option<String>,
    parent_info: &Option<ParentInfo>,
    ctx: &Context,
) -> Result<String> {
    let config = ctx.config();
    let title_str = title.as_deref().unwrap_or("");

    let backlink_content = if config.linking.insert_in_child {
        if let Some(parent) = parent_info {
            generate_link_text(&parent.id, &parent.filename, parent.title.as_deref(), ctx)
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    if TemplateService::should_use_template(&config.template) {
        generate_template_content(title_str, &backlink_content, ctx)
    } else {
        Ok(TemplateService::generate_content(
            None,
            title_str,
            &backlink_content,
        ))
    }
}

/// Information about the parent note for bidirectional linking
#[derive(Debug, Clone)]
struct ParentInfo {
    id: String,
    filename: String,
    path: String,
    title: Option<String>,
}

/// Determine parent information for bidirectional linking
fn determine_parent_info(new_id: &Id, ctx: &Context) -> Result<Option<ParentInfo>> {
    if let Ok(Some(parent_id)) = new_id.parent() {
        let files = ctx.vault_service.get_vault_files();
        let id_manager = ctx.get_id_manager();

        for file in files {
            if let Some(filename) = file.file_name().and_then(|n| n.to_str()) {
                if let Some(file_id) = id_manager.extract_from_filename(filename) {
                    if file_id == parent_id {
                        let title = extract_title_from_file(&file, ctx).ok();
                        return Ok(Some(ParentInfo {
                            id: parent_id.to_string(),
                            filename: file
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or(filename)
                                .to_string(),
                            path: file.to_string_lossy().to_string(),
                            title,
                        }));
                    }
                }
            }
        }
    }
    Ok(None)
}

/// Extract title from a note file
fn extract_title_from_file(file: &std::path::Path, ctx: &Context) -> Result<String> {
    let content = ctx.vault_service.read_file(file)?;

    for line in content.lines().take(5) {
        if let Some(title) = line.strip_prefix("# ") {
            return Ok(title.trim().to_string());
        }
    }

    let filename = file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Untitled");

    let id_manager = ctx.get_id_manager();
    let id = id_manager
        .extract_from_filename(filename)
        .map(|id| id.to_string())
        .unwrap_or_default();
    let title = filename.strip_prefix(&id.to_string()).unwrap_or(filename);

    Ok(title
        .trim_start_matches(|c: char| !c.is_alphanumeric())
        .to_string())
}

/// Insert child link into parent file
fn insert_child_link_in_parent(
    child_id: &str,
    child_title: &Option<String>,
    parent: &ParentInfo,
    ctx: &Context,
) -> Result<()> {
    let config = &ctx.config().linking;

    if !config.insert_in_parent {
        return Ok(());
    }

    let child_filename = generate_filename(child_id, child_title, ctx);
    let child_filename_base = child_filename
        .strip_suffix(".md")
        .unwrap_or(&child_filename);
    let link_text = generate_link_text(child_id, child_filename_base, child_title.as_deref(), ctx);

    ctx.vault_service.insert_content_into_file(
        &parent.path,
        &link_text,
        &config.insertion_point,
        config.create_links_section,
    )?;

    println!(
        "🔗 Added link to child {} in parent {}",
        child_id, parent.id
    );
    Ok(())
}

/// Generate link text based on configuration
fn generate_link_text(id: &str, filename: &str, title: Option<&str>, ctx: &Context) -> String {
    let config = &ctx.config().linking;

    if let Some(format_template) = &config.format {
        format_template
            .replace("{id}", id)
            .replace("{filename}", filename)
            .replace("{title}", title.unwrap_or(filename))
    } else if config.use_title_alias && title.is_some() {
        format!("[[{}|{}]]", filename, title.unwrap())
    } else {
        format!("[[{}]]", filename)
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
