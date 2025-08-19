// crates/zettel-cli/src/commands/template.rs - Template System Commands
//
// These commands provide direct access to template functionality for
// development, testing, and validation workflows. They follow the Unix
// philosophy of small, composable tools.
//
// DESIGN PRINCIPLES:
// - Pipeline friendly: Support stdin/stdout for composition
// - Pure operations: Don't modify vault, just process templates
// - Clear output: Both human and machine readable formats
// - Comprehensive: Cover all template development needs

use anyhow::Result;
use serde_json;
use std::fs;
use std::io::{self, Read};
use zettel_core::template::TemplateService;

use crate::cli::TemplateCommands;
use crate::context::Context;

/// Handle all template-related commands
///
/// This is the main entry point for template operations. Each command is
/// designed to be useful both interactively and in scripts/pipelines.
pub fn handle(ctx: &Context, cmd: TemplateCommands) -> Result<()> {
    match cmd {
        TemplateCommands::Validate { file } => handle_validate(ctx, file),
        TemplateCommands::Test { title, link, file } => handle_test(ctx, title, link, file),
        TemplateCommands::Placeholders { file, json } => handle_placeholders(ctx, file, json),
        TemplateCommands::Example {
            template_type,
            output,
        } => handle_example(template_type, output),
    }
}

/// Validate template file format and requirements
///
/// This command helps users ensure their templates will work correctly
/// before using them in note creation. It checks both syntax and
/// configuration requirements.
fn handle_validate(ctx: &Context, file: Option<String>) -> Result<()> {
    // Read template content from file or stdin
    let template_content = read_template_input(ctx, file.as_deref())?;

    // Validate against current configuration
    let config = &ctx.config().template;
    let validation_result = TemplateService::validate_template(&template_content, config);

    if validation_result.valid {
        println!("✅ Template validation passed");

        if !validation_result.found_placeholders.is_empty() {
            println!(
                "📋 Found placeholders: {}",
                validation_result.found_placeholders.join(", ")
            );
        }

        // Additional helpful information
        if !config.require_title
            && !validation_result
                .found_placeholders
                .contains(&"title".to_string())
        {
            println!("ℹ️  Note: Template has no {{title}} placeholder (allowed by config)");
        }

        if !config.require_link
            && !validation_result
                .found_placeholders
                .contains(&"link".to_string())
        {
            println!("ℹ️  Note: Template has no {{link}} placeholder (allowed by config)");
        }

        Ok(())
    } else {
        eprintln!("❌ Template validation failed");
        eprintln!(
            "   {}",
            validation_result
                .message
                .unwrap_or_else(|| "Unknown error".to_string())
        );

        if !validation_result.missing_placeholders.is_empty() {
            eprintln!(
                "   Missing required: {{{{{}}}}}",
                validation_result.missing_placeholders.join("}}, {{")
            );
        }

        if !validation_result.found_placeholders.is_empty() {
            eprintln!(
                "   Found placeholders: {}",
                validation_result.found_placeholders.join(", ")
            );
        }

        // Helpful suggestions
        eprintln!();
        eprintln!("💡 Tips:");
        eprintln!("   - Add missing placeholders to your template");
        eprintln!("   - Or disable requirements in vault configuration");
        eprintln!("   - Check placeholder spelling: {{{{title}}}} not {{{{Title}}}}");

        std::process::exit(1);
    }
}

/// Test template output with sample data
///
/// This is like a "dry run" of note creation - shows what content would
/// be generated without actually creating a file.
fn handle_test(ctx: &Context, title: String, link: String, file: Option<String>) -> Result<()> {
    // Read template content from file or stdin
    let template_content = read_template_input(ctx, file.as_deref())?;

    // Validate template first
    let config = &ctx.config().template;
    let validation_result = TemplateService::validate_template(&template_content, config);

    if !validation_result.valid {
        eprintln!(
            "❌ Template validation failed: {}",
            validation_result
                .message
                .unwrap_or_else(|| "Unknown error".to_string())
        );
        eprintln!("   Use 'zettel template validate' for more details");
        std::process::exit(1);
    }

    // Generate content with provided values
    let generated_content =
        TemplateService::generate_content(Some(&template_content), &title, &link);

    // Output with clear separation
    println!("📄 Template test output:");
    println!("{}", "─".repeat(50));
    println!("{}", generated_content);
    println!("{}", "─".repeat(50));
    println!("✅ Template processed successfully");

    Ok(())
}

/// Extract placeholder information from template
///
/// Analyzes template and reports all placeholders found. Useful for
/// documentation and understanding template requirements.
fn handle_placeholders(ctx: &Context, file: Option<String>, json: bool) -> Result<()> {
    // Read template content from file or stdin
    let template_content = read_template_input(ctx, file.as_deref())?;

    // Validate to get placeholder information
    let config = &ctx.config().template;
    let validation_result = TemplateService::validate_template(&template_content, config);

    if json {
        // Machine-readable output
        let output = serde_json::json!({
            "valid": validation_result.valid,
            "found_placeholders": validation_result.found_placeholders,
            "missing_placeholders": validation_result.missing_placeholders,
            "validation_message": validation_result.message,
            "requirements": {
                "title_required": config.require_title,
                "link_required": config.require_link
            }
        });

        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        // Human-readable output
        println!("📋 Template placeholder analysis:");
        println!();

        if validation_result.found_placeholders.is_empty() {
            println!("   No placeholders found");
        } else {
            println!("   Found placeholders:");
            for placeholder in &validation_result.found_placeholders {
                let description = match placeholder.as_str() {
                    "title" => "Note title",
                    "link" => "Backlink to parent note",
                    _ => "Unknown placeholder",
                };
                println!("     • {{{{{}}}}} - {}", placeholder, description);
            }
        }

        println!();
        println!("   Requirements (from vault config):");
        println!("     • {{{{title}}}} required: {}", config.require_title);
        println!("     • {{{{link}}}} required: {}", config.require_link);

        if !validation_result.valid {
            println!();
            println!("   ❌ Validation status: FAILED");
            if let Some(message) = &validation_result.message {
                println!("      {}", message);
            }
        } else {
            println!();
            println!("   ✅ Validation status: PASSED");
        }
    }

    Ok(())
}

/// Generate example template files
///
/// Creates sample templates to help users get started with customization.
/// Covers common use cases and demonstrates placeholder usage.
fn handle_example(template_type: String, output: Option<String>) -> Result<()> {
    let template_content = match template_type.as_str() {
        "basic" => generate_basic_template(),
        "academic" => generate_academic_template(),
        "meeting" => generate_meeting_template(),
        "daily" => generate_daily_template(),
        _ => {
            eprintln!("❌ Unknown template type: {}", template_type);
            eprintln!("   Available types: basic, academic, meeting, daily");
            std::process::exit(1);
        }
    };

    match output {
        Some(file_path) => {
            // Write to specified file
            fs::write(&file_path, template_content)?;
            println!("✅ Example template written to: {}", file_path);
            println!("💡 Edit the file to customize, then set in vault config");
        }
        None => {
            // Write to stdout for pipeline use
            print!("{}", template_content);
        }
    }

    Ok(())
}

/// Read template content from file or stdin
///
/// Handles the common pattern of reading template content either from
/// a specified file or from stdin for pipeline use.
fn read_template_input(ctx: &Context, file_path: Option<&str>) -> Result<String> {
    match file_path {
        Some(path) => {
            // Try reading as vault-relative path first, then as absolute path
            let vault_relative_result = ctx.vault_service.read_template_file(path);
            match vault_relative_result {
                Ok(content) => Ok(content),
                Err(_) => {
                    // Try as absolute/current-directory path
                    fs::read_to_string(path).map_err(|e| {
                        anyhow::anyhow!(
                            "Failed to read template file '{}': {}\n\nTried both vault-relative and absolute paths.",
                            path, e
                        )
                    })
                }
            }
        }
        None => {
            // Read from stdin
            let mut buffer = String::new();
            io::stdin().read_to_string(&mut buffer)?;

            if buffer.trim().is_empty() {
                return Err(anyhow::anyhow!(
                    "No template content provided. Either specify a file or provide content via stdin."
                ));
            }

            Ok(buffer)
        }
    }
}

/// Generate basic example template
///
/// Simple template with title heading and backlink placement.
/// Good starting point for most users.
fn generate_basic_template() -> String {
    r#"# {{title}}

{{link}}

## Notes

"#
    .to_string()
}

/// Generate academic research template
///
/// Template structured for research notes with common academic sections.
/// Demonstrates more complex template organization.
fn generate_academic_template() -> String {
    r#"# {{title}}

**Source:** {{link}}

## Summary

## Key Points

- 

## Questions

- 

## Related Ideas

- 

## References

"#
    .to_string()
}

/// Generate meeting notes template
///
/// Template for capturing meeting information with agenda and action items.
/// Shows how templates can structure recurring note types.
fn generate_meeting_template() -> String {
    r#"# Meeting: {{title}}

**Previous Meeting:** {{link}}

**Date:** 
**Attendees:** 

## Agenda

- 

## Discussion Notes

## Decisions Made

- 

## Action Items

- [ ] 

## Next Meeting

**Date:** 
**Topics:** 

"#
    .to_string()
}

/// Generate daily note template
///
/// Template for daily journaling or planning notes.
/// Demonstrates date-based note structure.
fn generate_daily_template() -> String {
    r#"# {{title}}

{{link}}

## Today's Focus

- 

## Notes

## Completed

- [ ] 

## Tomorrow

- 

"#
    .to_string()
}

// TEMPLATE COMMAND BENEFITS:
//
// 1. DEVELOPMENT WORKFLOW:
//    Users can iterate on templates without creating test notes
//    Validation catches errors before they affect note creation
//
// 2. PIPELINE INTEGRATION:
//    Commands support stdin/stdout for use in scripts and CI/CD
//    JSON output enables tooling integration
//
// 3. LEARNING AID:
//    Example templates demonstrate best practices
//    Placeholder analysis helps understand template requirements
//
// 4. DEBUGGING:
//    Test command shows exact output before note creation
//    Validation provides detailed error messages
//
// UNIX COMPOSABILITY EXAMPLES:
//
// ```bash
// # Validate all templates in a directory
// find templates/ -name "*.md" -exec zettel template validate {} \;
//
// # Test template with different values
// for title in "Research" "Meeting" "Daily"; do
//   echo "Testing with title: $title"
//   zettel template test "$title" "" < my-template.md
//   echo "---"
// done
//
// # Generate and immediately validate template
// zettel template example academic | zettel template validate
//
// # Extract placeholders for documentation
// zettel template placeholders --json templates/*.md | jq '.found_placeholders[]'
//
// # Create custom template from basic example
// zettel template example basic > my-template.md
// $EDITOR my-template.md
// zettel template validate my-template.md
// ```
