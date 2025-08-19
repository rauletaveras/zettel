// crates/zettel-core/src/template.rs - Template System Core Logic
//
// This module implements the template system that allows users to customize
// note creation beyond the built-in "# Title\n\nBacklink" format.
//
// DESIGN PRINCIPLES:
// - Pure functions: No I/O, only string processing
// - Composable: Functions can be used independently
// - Fail-fast: Template validation catches errors early
// - Flexible: Support both single templates and template directories
//
// TEMPLATE FORMAT:
// Templates are markdown files with special placeholders:
// - {{title}} - Replaced with user-provided note title
// - {{link}} - Replaced with backlink to parent note
//
// EXAMPLE TEMPLATE:
// ```markdown
// # {{title}}
//
// Created: {{date}}
// Parent: {{link}}
//
// ## Notes
//
// ## References
// ```

use regex::Regex;
use std::collections::HashMap;
use thiserror::Error;

use crate::config::TemplateConfig;

/// Errors that can occur during template operations
#[derive(Error, Debug, Clone, PartialEq)]
pub enum TemplateError {
    #[error("Template validation failed: {0}")]
    ValidationError(String),

    #[error("Missing required placeholder: {0}")]
    MissingPlaceholder(String),

    #[error("Invalid template configuration: {0}")]
    ConfigError(String),

    #[error("Template processing error: {0}")]
    ProcessingError(String),
}

/// Result type for template operations
pub type TemplateResult<T> = Result<T, TemplateError>;

/// Represents validation result for template content
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationResult {
    /// Whether the template passed validation
    pub valid: bool,
    /// Error message if validation failed
    pub message: Option<String>,
    /// List of missing required placeholders
    pub missing_placeholders: Vec<String>,
    /// List of found placeholders
    pub found_placeholders: Vec<String>,
}

impl ValidationResult {
    /// Create a successful validation result
    pub fn success(found_placeholders: Vec<String>) -> Self {
        Self {
            valid: true,
            message: None,
            missing_placeholders: Vec::new(),
            found_placeholders,
        }
    }

    /// Create a failed validation result
    pub fn failure(message: String, missing: Vec<String>, found: Vec<String>) -> Self {
        Self {
            valid: false,
            message: Some(message),
            missing_placeholders: missing,
            found_placeholders: found,
        }
    }
}

/// Core template processing service
///
/// This handles all template-related business logic:
/// - Validation of template content against requirements
/// - Placeholder substitution with actual values
/// - Content generation for both template and built-in modes
///
/// PURE FUNCTIONS DESIGN:
/// All methods are pure functions that take input and produce output
/// without side effects. This makes testing easy and behavior predictable.
pub struct TemplateService;

impl TemplateService {
    /// Determines if templates should be used based on configuration
    ///
    /// Business rule: Templates are used when:
    /// 1. Template system is enabled in config
    /// 2. Template file path is specified and non-empty
    ///
    /// This is a pure function that only examines configuration.
    pub fn should_use_template(config: &TemplateConfig) -> bool {
        config.enabled && !config.file.trim().is_empty()
    }

    /// Validates template content against configuration requirements
    ///
    /// Checks that required placeholders are present. This catches configuration
    /// errors early, before note creation fails.
    ///
    /// VALIDATION RULES:
    /// - If require_title is true, {{title}} must be present
    /// - If require_link is true, {{link}} must be present
    /// - Unknown placeholders are allowed (forward compatibility)
    ///
    /// EXAMPLES:
    /// ```rust
    /// let config = TemplateConfig { require_title: true, require_link: true, .. };
    /// let content = "# {{title}}\n\nParent: {{link}}";
    /// let result = TemplateService::validate_template(content, &config);
    /// assert!(result.valid);
    /// ```
    pub fn validate_template(content: &str, config: &TemplateConfig) -> ValidationResult {
        // Extract all placeholders from template content
        let found_placeholders = Self::extract_placeholders(content);
        let mut missing_placeholders = Vec::new();

        // Check for required title placeholder
        if config.require_title && !found_placeholders.contains(&"title".to_string()) {
            missing_placeholders.push("title".to_string());
        }

        // Check for required link placeholder
        if config.require_link && !found_placeholders.contains(&"link".to_string()) {
            missing_placeholders.push("link".to_string());
        }

        // Generate validation result
        if missing_placeholders.is_empty() {
            ValidationResult::success(found_placeholders)
        } else {
            let message = format!(
                "Template missing required placeholder(s): {{{{{}}}}}",
                missing_placeholders.join("}}, {{")
            );
            ValidationResult::failure(message, missing_placeholders, found_placeholders)
        }
    }

    /// Extracts all placeholder names from template content
    ///
    /// Finds all instances of {{placeholder_name}} and returns the names.
    /// This is used for validation and debugging.
    ///
    /// REGEX PATTERN: {{(\w+)}}
    /// - {{ and }} are literal braces
    /// - (\w+) captures word characters (letters, numbers, underscore)
    ///
    /// EXAMPLES:
    /// - "# {{title}}" -> ["title"]
    /// - "{{title}} and {{link}}" -> ["title", "link"]
    /// - "{{title}} {{title}}" -> ["title"] (deduplicated)
    fn extract_placeholders(content: &str) -> Vec<String> {
        let placeholder_regex = Regex::new(r"\{\{(\w+)\}\}").unwrap();
        let mut placeholders = Vec::new();

        for capture in placeholder_regex.captures_iter(content) {
            if let Some(placeholder) = capture.get(1) {
                let name = placeholder.as_str().to_string();
                if !placeholders.contains(&name) {
                    placeholders.push(name);
                }
            }
        }

        placeholders
    }

    /// Generates final note content using template or built-in format
    ///
    /// This is the core content generation function. It handles both template
    /// mode (with placeholder substitution) and built-in mode (standard format).
    ///
    /// TEMPLATE MODE:
    /// Replaces all placeholders with provided values. Unknown placeholders
    /// are left unchanged for forward compatibility.
    ///
    /// BUILT-IN MODE:
    /// Creates standard "# Title\n\nBacklink" format when no template provided.
    ///
    /// BUSINESS RULES:
    /// - Empty title is handled gracefully
    /// - Empty backlink is handled gracefully
    /// - Whitespace is preserved from template
    /// - Multiple occurrences of same placeholder are all replaced
    ///
    /// EXAMPLES:
    /// ```rust
    /// // Template mode
    /// let template = "# {{title}}\n\nParent: {{link}}";
    /// let content = TemplateService::generate_content(
    ///     Some(template), "My Note", "[[parent]]"
    /// );
    /// // Result: "# My Note\n\nParent: [[parent]]"
    ///
    /// // Built-in mode
    /// let content = TemplateService::generate_content(
    ///     None, "My Note", "[[parent]]"
    /// );
    /// // Result: "# My Note\n\n[[parent]]"
    /// ```
    pub fn generate_content(
        template_content: Option<&str>,
        title: &str,
        backlink_content: &str,
    ) -> String {
        match template_content {
            Some(template) => {
                // Template mode: substitute placeholders
                Self::substitute_placeholders(template, title, backlink_content)
            }
            None => {
                // Built-in mode: standard markdown format
                Self::generate_builtin_content(title, backlink_content)
            }
        }
    }

    /// Substitutes placeholders in template with actual values
    ///
    /// Replaces known placeholders and leaves unknown ones unchanged.
    /// This enables forward compatibility with future placeholder types.
    ///
    /// CURRENT PLACEHOLDERS:
    /// - {{title}} -> note title
    /// - {{link}} -> backlink content
    ///
    /// FUTURE PLACEHOLDERS (examples):
    /// - {{date}} -> current date
    /// - {{author}} -> note author
    /// - {{id}} -> note ID
    /// - {{parent_id}} -> parent note ID
    fn substitute_placeholders(template: &str, title: &str, backlink: &str) -> String {
        template
            .replace("{{title}}", title)
            .replace("{{link}}", backlink)
        // Note: Unknown placeholders like {{date}} are left unchanged
        // This allows templates to include future features
    }

    /// Generates built-in content format when no template is used
    ///
    /// Creates the standard zettelkasten note format:
    /// - Heading with note title
    /// - Blank line for separation
    /// - Backlink to parent (if provided)
    ///
    /// FORMATTING RULES:
    /// - Title becomes "# Title" heading
    /// - Leading whitespace is trimmed from title
    /// - Backlink gets separated by blank line
    /// - Empty title or backlink are handled gracefully
    fn generate_builtin_content(title: &str, backlink: &str) -> String {
        let mut content = String::new();

        // Add title heading if provided
        if !title.trim().is_empty() {
            content.push_str(&format!("# {}", title.trim_start()));
        }

        // Add backlink if provided
        if !backlink.trim().is_empty() {
            if !content.is_empty() {
                content.push_str("\n\n");
            }
            content.push_str(backlink);
        }

        content
    }

    /// Resolves template file path based on configuration
    ///
    /// Handles both single template file and template directory modes.
    /// Returns the final template file path to read.
    ///
    /// RESOLUTION LOGIC:
    /// 1. If specific file is configured, use that
    /// 2. If directory is configured, use default template within directory
    /// 3. Validate configuration makes sense
    ///
    /// This is pure logic - actual file reading is handled by CLI layer.
    pub fn resolve_template_path(config: &TemplateConfig) -> TemplateResult<String> {
        if !config.file.trim().is_empty() {
            // Direct file path specified
            Ok(config.file.trim().to_string())
        } else if !config.directory.trim().is_empty() {
            // Template directory specified - use default template
            if config.default_template.trim().is_empty() {
                return Err(TemplateError::ConfigError(
                    "Template directory specified but no default template name provided"
                        .to_string(),
                ));
            }

            let directory = config.directory.trim();
            let template_name = config.default_template.trim();
            Ok(format!("{}/{}", directory, template_name))
        } else {
            Err(TemplateError::ConfigError(
                "No template file or directory specified".to_string(),
            ))
        }
    }

    /// Creates a context map for advanced template processing (future feature)
    ///
    /// This prepares for more sophisticated template systems that might use
    /// templating engines like Handlebars or Tera. Currently returns basic
    /// key-value pairs for the placeholders we support.
    ///
    /// FUTURE ENHANCEMENT:
    /// Could support conditional logic, loops, includes, etc.
    ///
    /// ```handlebars
    /// # {{title}}
    /// {{#if parent}}
    /// Parent: {{parent}}
    /// {{/if}}
    /// ```
    #[allow(dead_code)]
    pub fn create_template_context(title: &str, backlink: &str) -> HashMap<String, String> {
        let mut context = HashMap::new();
        context.insert("title".to_string(), title.to_string());
        context.insert("link".to_string(), backlink.to_string());

        // Future: Add more context variables
        // context.insert("date".to_string(), chrono::Utc::now().format("%Y-%m-%d").to_string());
        // context.insert("author".to_string(), get_author_from_config());
        // context.insert("id".to_string(), note_id.to_string());

        context
    }
}

/// Template manager for handling multiple templates (future feature)
///
/// This would enable template selection during note creation:
/// - Academic paper template
/// - Daily note template
/// - Meeting notes template
/// - Reference note template
///
/// For now, it's a placeholder for future expansion.
#[allow(dead_code)]
pub struct TemplateManager {
    templates: HashMap<String, String>,
}

#[allow(dead_code)]
impl TemplateManager {
    /// Create new template manager
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
        }
    }

    /// Register a template with a name
    pub fn register_template(&mut self, name: String, content: String) {
        self.templates.insert(name, content);
    }

    /// Get template by name
    pub fn get_template(&self, name: &str) -> Option<&String> {
        self.templates.get(name)
    }

    /// List available template names
    pub fn list_templates(&self) -> Vec<&String> {
        self.templates.keys().collect()
    }
}

// TEMPLATE SYSTEM BENEFITS:
//
// 1. PURE FUNCTIONS:
//    All template logic is pure - no side effects, easy to test
//    Functions take input and produce output predictably
//
// 2. COMPOSABILITY:
//    Template validation, substitution, and generation are separate
//    Can be used independently or composed together
//
// 3. EXTENSIBILITY:
//    Easy to add new placeholders or template features
//    Forward compatible - unknown placeholders are preserved
//
// 4. ERROR HANDLING:
//    Template validation catches configuration errors early
//    Clear error messages help users fix template issues
//
// 5. CONFIGURATION DRIVEN:
//    All behavior controlled by configuration
//    No hardcoded assumptions about template format
//
// EXAMPLES OF TEMPLATE USAGE:
//
// Basic template:
// ```markdown
// # {{title}}
//
// {{link}}
//
// ## Notes
//
// ## References
// ```
//
// Academic template:
// ```markdown
// # {{title}}
//
// **Source:** {{link}}
//
// ## Summary
//
// ## Key Points
//
// ## Questions
//
// ## Related Ideas
// ```
//
// Meeting notes template:
// ```markdown
// # Meeting: {{title}}
//
// **Previous:** {{link}}
//
// ## Agenda
//
// ## Notes
//
// ## Action Items
//
// ## Next Steps
// ```
//
// UNIX COMPOSABILITY:
//
// The template system maintains Unix philosophy:
//
// ```bash
// # Generate template content
// echo "# {{title}}\n\n{{link}}" | zettel template apply "My Note" "[[parent]]"
//
// # Validate template
// zettel template validate < my-template.md
//
// # List available placeholders
// zettel template placeholders < my-template.md
//
// # Create note with template
// zettel note create 1a "My Note" --template academic.md
// ```
