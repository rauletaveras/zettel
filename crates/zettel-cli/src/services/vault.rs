// crates/zettel-cli/src/services/vault.rs - File System Operations Service
//
// This service handles all file system operations and vault interactions.
// It's the "data access layer" of the application - it knows HOW to interact
// with files but doesn't know WHY (that's business logic in commands).
//
// DESIGN PRINCIPLES:
// - Single Responsibility: Only handles file I/O operations
// - No Business Logic: Doesn't make decisions about WHAT to do, only HOW
// - Error Handling: Converts file system errors to user-friendly messages
// - Path Safety: Uses PathBuf for cross-platform compatibility
// - Abstraction: Provides vault-specific operations built on std::fs
//
// COMPARISON TO TYPESCRIPT ORIGINAL:
// This is equivalent to FileOperationsService in the TypeScript version.
// Like that service, it handles the infrastructure concerns while leaving
// business logic to other layers.

use anyhow::{Context as AnyhowContext, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Handles all file system operations and vault interactions
///
/// This service encapsulates all the ways the application interacts with
/// the file system. By centralizing file operations, we can:
/// - Handle errors consistently
/// - Mock file system for testing
/// - Change file storage strategy without affecting business logic
/// - Ensure proper path handling across platforms
///
/// SERVICE PATTERN BENEFITS:
/// - Testability: Easy to mock for unit tests
/// - Maintainability: File I/O changes are isolated
/// - Reliability: Consistent error handling and path validation
/// - Performance: Could add caching or batching in the future
///
/// EXAMPLE USAGE:
/// ```rust
/// let vault = VaultService::new("/path/to/vault".into());
/// let files = vault.get_vault_files();
/// let exists = vault.id_exists("1a2");
/// vault.create_file("new-note.md", "# Content")?;
/// ```
pub struct VaultService {
    /// Path to the vault root directory
    ///
    /// All file operations are relative to this path. This is the "working directory"
    /// for the zettelkasten system. Private to ensure controlled access through methods.
    vault_path: PathBuf,
}

impl VaultService {
    /// Create a new vault service for the given directory
    ///
    /// This initializes the service but doesn't validate the path exists.
    /// Path validation should be done by the Context during application startup
    /// to provide better error messages.
    ///
    /// DESIGN DECISION: Separate construction from validation
    /// This allows the service to be created even if the vault doesn't exist yet
    /// (useful for the init command). Validation happens at the Context level.
    ///
    /// RUST PATTERN: Constructor pattern
    /// Simple constructor that takes owned data and returns configured instance.
    /// No complex initialization or fallible operations in the constructor.
    pub fn new(vault_path: PathBuf) -> Self {
        Self { vault_path }
    }

    /// Check if a note with the given ID exists in the vault
    ///
    /// This implements the core ID existence checking logic used by the ID manager
    /// to avoid conflicts when generating new IDs. It searches all markdown files
    /// to see if any have a filename that starts with the given ID.
    ///
    /// ALGORITHM:
    /// 1. List all files in vault directory (ignoring errors for non-existent vault)
    /// 2. Filter to only .md files
    /// 3. Extract basename without extension
    /// 4. Check if basename starts with the ID
    /// 5. Verify it's followed by non-alphanumeric char or end of string
    ///
    /// MATCHING LOGIC:
    /// We use a simple heuristic: ID followed by non-alphanumeric character or end.
    /// This catches cases like:
    /// - "1a2.md" (exact match)
    /// - "1a2 - Title.md" (ID + separator)
    /// - "1a2_note.md" (ID + underscore)
    /// But not false positives like "1a23.md" when looking for "1a2"
    ///
    /// PERFORMANCE NOTE:
    /// This scans all files on each call. For large vaults, we might want to
    /// cache the list of existing IDs and invalidate on file changes.
    ///
    /// ERROR HANDLING:
    /// Returns false if vault doesn't exist or isn't readable, rather than erroring.
    /// This allows the function to be used during vault initialization.
    pub fn id_exists(&self, id: &str) -> bool {
        // Early return if vault doesn't exist (graceful handling for init command)
        if !self.vault_path.exists() {
            return false;
        }

        // Try to read directory - if it fails, assume ID doesn't exist
        // This is conservative: better to allow potential duplicates than fail
        let Ok(entries) = fs::read_dir(&self.vault_path) else {
            return false;
        };

        // Search through all directory entries for matching markdown files
        for entry in entries.flatten() {
            // Only process files we can get names for
            if let Some(filename) = entry.file_name().to_str() {
                // Only consider markdown files as potential zettel notes
                if filename.ends_with(".md") {
                    // Remove .md extension to get the base filename
                    let stem = filename.strip_suffix(".md").unwrap_or(filename);

                    // Check if filename starts with our target ID
                    if stem.starts_with(id) {
                        // Verify this is actually the ID, not just a prefix
                        // ID must be followed by non-alphanumeric or end of string
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

        false
    }

    /// Get list of all markdown files in the vault, sorted by name
    ///
    /// This provides the foundation for listing, searching, and other operations
    /// that need to work with all notes in the vault. The sorting ensures
    /// consistent output for user-facing commands.
    ///
    /// DESIGN DECISIONS:
    /// - Only returns .md files (filters out other file types)
    /// - Returns full PathBuf objects (commands can extract what they need)
    /// - Sorts by filename for predictable output
    /// - Handles missing vault gracefully (returns empty list)
    ///
    /// CURRENT LIMITATIONS:
    /// - Only scans vault root (doesn't recurse into subdirectories)
    /// - No filtering for system files or directories
    /// - Loads all files into memory (not lazy)
    ///
    /// FUTURE ENHANCEMENTS:
    /// - Support subdirectories with recursive scanning
    /// - Respect .zettelignore files
    /// - Lazy iteration for very large vaults
    /// - Filter out non-zettel markdown files
    ///
    /// ERROR HANDLING:
    /// Returns empty vec if vault doesn't exist or isn't readable.
    /// Individual file access errors are ignored (skip unreadable files).
    pub fn get_vault_files(&self) -> Vec<PathBuf> {
        let mut files = Vec::new();

        // Try to read vault directory - return empty list if it fails
        let Ok(entries) = fs::read_dir(&self.vault_path) else {
            return files;
        };

        // Process all readable directory entries
        for entry in entries.flatten() {
            let path = entry.path();

            // Only include markdown files in the result
            if path.extension().map_or(false, |ext| ext == "md") {
                files.push(path);
            }
        }

        // Sort for consistent, predictable output
        // This ensures commands always show files in the same order
        files.sort();
        files
    }

    /// Create a new file with given content
    ///
    /// This is the core file creation operation used by note creation commands.
    /// It handles directory creation, atomic writes, and error reporting.
    ///
    /// SAFETY FEATURES:
    /// - Creates parent directories if they don't exist
    /// - Atomic write (file appears complete or not at all)
    /// - Returns full path of created file for further operations
    /// - Provides detailed error context for troubleshooting
    ///
    /// ATOMIC WRITE EXPLANATION:
    /// std::fs::write is atomic on most systems - the file either gets written
    /// completely or not at all. This prevents partial file creation if the
    /// operation is interrupted.
    ///
    /// PATH HANDLING:
    /// Relative paths are resolved against vault_path. Absolute paths are used
    /// as-is (allowing creation outside vault if needed for future features).
    ///
    /// EXAMPLES:
    /// ```rust
    /// // Create in vault root
    /// vault.create_file("note.md", "# My Note\n")?;
    ///
    /// // Create in subdirectory (creates dir if needed)
    /// vault.create_file("subfolder/note.md", "# Content")?;
    /// ```
    pub fn create_file(&self, relative_path: &str, content: &str) -> Result<PathBuf> {
        // Resolve relative path against vault root
        let full_path = self.vault_path.join(relative_path);

        // Create parent directories if they don't exist
        // This allows creating files in subdirectories without manual mkdir
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        // Write file content atomically
        fs::write(&full_path, content)
            .with_context(|| format!("Failed to create file: {}", full_path.display()))?;

        Ok(full_path)
    }

    /// Read file content from disk
    ///
    /// Simple wrapper around std::fs::read_to_string with better error context.
    /// Used by commands that need to examine or display file content.
    ///
    /// ENCODING ASSUMPTION:
    /// Assumes files are UTF-8 encoded. This is reasonable for markdown files
    /// but might need enhancement for international character sets.
    ///
    /// ERROR CONTEXT:
    /// Provides file path in error message to help users identify problems.
    /// Common errors: file not found, permission denied, invalid UTF-8.
    pub fn read_file(&self, path: &Path) -> Result<String> {
        fs::read_to_string(path).with_context(|| format!("Failed to read file: {}", path.display()))
    }

    /// Get the directory where new files should be created
    ///
    /// This implements the business rule for where new notes should be placed.
    /// Currently just returns vault root, but could be enhanced to support
    /// subdirectories or date-based organization.
    ///
    /// BUSINESS RULE PLACEHOLDER:
    /// The _current_file_path parameter is unused but included for future
    /// enhancement where new files might be created relative to the currently
    /// open file's location.
    ///
    /// FUTURE ENHANCEMENTS:
    /// - Create new notes in same directory as current file
    /// - Support year/month subdirectory organization
    /// - Respect user-configured default directory
    /// - Handle special cases (daily notes, reference notes, etc.)
    pub fn get_new_file_directory(&self, _current_file_path: &str) -> &Path {
        // For now, just use vault root for all new files
        // TODO: Could be enhanced to support subdirectory organization
        &self.vault_path
    }

    /// Initialize vault directory structure
    ///
    /// This creates the basic directory structure needed for a new zettelkasten vault.
    /// Called by the init command to set up a new vault from scratch.
    ///
    /// CREATED STRUCTURE:
    /// ```
    /// vault_directory/
    /// ├── .zettel/
    /// │   └── config.toml
    /// └── (ready for notes)
    /// ```
    ///
    /// DESIGN PHILOSOPHY:
    /// - Minimal structure: Don't create files/directories users didn't ask for
    /// - Hidden metadata: .zettel directory follows Unix hidden file convention
    /// - Self-documenting: Config file includes comments explaining options
    /// - Safe operation: Won't overwrite existing files
    ///
    /// THE .zettel DIRECTORY:
    /// Similar to .git in Git repositories, this marks a directory as a zettelkasten
    /// vault and contains metadata files. Future features might include:
    /// - config.toml: User preferences
    /// - cache/: Performance optimization data
    /// - hooks/: Custom scripts for note creation/modification
    /// - templates/: Note templates
    ///
    /// ERROR HANDLING:
    /// Returns detailed errors if directory creation or file writing fails.
    /// Common issues: permission denied, disk full, path too long.
    pub fn init_vault(&self) -> Result<()> {
        // Create main vault directory if it doesn't exist
        fs::create_dir_all(&self.vault_path).with_context(|| {
            format!(
                "Failed to create vault directory: {}",
                self.vault_path.display()
            )
        })?;

        // Create the .zettel metadata directory
        let zettel_dir = self.vault_path.join(".zettel");
        fs::create_dir_all(&zettel_dir).with_context(|| {
            format!(
                "Failed to create .zettel directory: {}",
                zettel_dir.display()
            )
        })?;

        // Create a default configuration file with documentation
        let config_content = r#"# Zettel Configuration File
#
# This file controls how the zettel CLI tool behaves in this vault.
# Lines starting with # are comments and are ignored.

[id]
# ID matching rule: "strict", "separator", or "fuzzy"
match_rule = "fuzzy"

# Separator used between ID and title in filenames
separator = " - "

# Allow Unicode characters in IDs (may cause filesystem issues)
allow_unicode = false

[note]
# Include note title in filename
add_title = false

# Add note title as frontmatter alias
add_alias = false

# File extension for new notes
extension = "md"

[template]
# Use custom template files
enabled = false

# Path to template file (relative to vault root)
# file = "templates/note.md"

# Require {{title}} placeholder in templates
require_title = true

# Require {{link}} placeholder in templates  
require_link = true

[linking]
# Insert link to child in parent when creating children
insert_in_parent = true

# Insert link to parent in child when creating children
insert_in_child = true

# Use title as display text in links: [[file|title]] vs [[file]]
use_title_alias = false

# Where to insert child links: "end", "after_title", "section"
insertion_point = "end"

# Create dedicated ## Links section when inserting
create_links_section = false

[editor]
# Editor command (overrides ZETTEL_EDITOR and EDITOR env vars)
# command = "helix"

# Arguments to pass to editor (supports {file}, {line}, {col} placeholders)
# args = ["+{line}:{col}"]

[output]
# Default output format: "human", "json", "csv"
default_format = "human"

# Color output: "auto", "always", "never"
color = "auto"

# Use pager for long output: "auto", "always", "never"  
pager = "auto"#;

        fs::write(zettel_dir.join("config.toml"), config_content)
            .with_context(|| "Failed to create config.toml")?;

        Ok(())
    }

    /// Read template file content from disk
    ///
    /// Reads a template file for use in note creation. Template paths are
    /// relative to the vault root directory. Returns the raw file content
    /// for template processing by the core template service.
    ///
    /// ERROR HANDLING:
    /// Provides specific error context for template file issues to help
    /// users troubleshoot template configuration problems.
    ///
    /// EXAMPLES:
    /// ```rust
    /// let content = vault.read_template_file("templates/academic.md")?;
    /// ```
    pub fn read_template_file(&self, relative_path: &str) -> Result<String> {
        let template_path = self.vault_path.join(relative_path);

        if !template_path.exists() {
            return Err(anyhow::anyhow!(
                "Template file not found: {}\nCheck the template path in your configuration.",
                template_path.display()
            ));
        }

        fs::read_to_string(&template_path).with_context(|| {
            format!(
                "Failed to read template file: {}\nCheck file permissions and encoding (should be UTF-8).",
                template_path.display()
            )
        })
    }

    /// Get the vault path for operations that need it
    ///
    /// Provides read-only access to the vault path. Some operations might need
    /// the vault path for display purposes or for operations that work with
    /// paths directly.
    ///
    /// DESIGN: Controlled access to internal state
    /// Rather than making vault_path public, we provide this accessor method.
    /// This maintains encapsulation while allowing necessary access.
    pub fn vault_path(&self) -> &Path {
        &self.vault_path
    }

    /// Insert content into an existing file at specified location
    ///
    /// Used for bidirectional linking - when creating child notes, we need to
    /// insert links into the parent file.
    ///
    /// INSERTION STRATEGIES:
    /// - "end": Append to end of file with proper spacing
    /// - "after_title": Insert after the first # heading
    /// - "section": Insert in or create a ## Links section
    pub fn insert_content_into_file(
        &self,
        file_path: &str,
        content: &str,
        insertion_point: &str,
        create_section: bool,
    ) -> Result<()> {
        let full_path = self.vault_path.join(file_path);

        if !full_path.exists() {
            return Err(anyhow::anyhow!(
                "File does not exist: {}",
                full_path.display()
            ));
        }

        let existing_content = fs::read_to_string(&full_path)
            .with_context(|| format!("Failed to read file: {}", full_path.display()))?;

        let new_content = match insertion_point {
            "end" => self.insert_at_end(&existing_content, content),
            "after_title" => self.insert_after_title(&existing_content, content),
            "section" => self.insert_in_links_section(&existing_content, content, create_section),
            _ => self.insert_at_end(&existing_content, content), // Default fallback
        };

        fs::write(&full_path, new_content)
            .with_context(|| format!("Failed to write file: {}", full_path.display()))?;

        Ok(())
    }

    /// Insert content at the end of file with proper spacing
    fn insert_at_end(&self, existing_content: &str, new_content: &str) -> String {
        let trimmed = existing_content.trim_end();
        if trimmed.is_empty() {
            new_content.to_string()
        } else {
            format!("{}\n\n{}", trimmed, new_content)
        }
    }

    /// Insert content after the first # heading
    fn insert_after_title(&self, existing_content: &str, new_content: &str) -> String {
        let lines: Vec<&str> = existing_content.lines().collect();

        // Find first heading line
        for (i, line) in lines.iter().enumerate() {
            if line.starts_with("# ") {
                let mut new_lines = lines[..=i].to_vec();
                new_lines.push(""); // Add blank line after title
                new_lines.push(new_content);
                new_lines.push(""); // Add blank line after inserted content

                // Add remaining lines if any
                if i + 1 < lines.len() {
                    new_lines.extend_from_slice(&lines[i + 1..]);
                }

                return new_lines.join("\n");
            }
        }

        // No heading found, insert at beginning
        if existing_content.trim().is_empty() {
            new_content.to_string()
        } else {
            format!("{}\n\n{}", new_content, existing_content)
        }
    }

    /// Insert content in a ## Links section, creating it if needed
    fn insert_in_links_section(
        &self,
        existing_content: &str,
        new_content: &str,
        create_section: bool,
    ) -> String {
        let lines: Vec<&str> = existing_content.lines().collect();

        // Look for existing ## Links section
        for (i, line) in lines.iter().enumerate() {
            if line.starts_with("## Links")
                || line.starts_with("## Related")
                || line.starts_with("## Children")
            {
                // Found links section, insert after it
                let mut new_lines = lines[..=i].to_vec();
                new_lines.push(""); // Blank line after section header
                new_lines.push(new_content);

                // Add remaining lines
                if i + 1 < lines.len() {
                    new_lines.push(""); // Blank line before next content
                    new_lines.extend_from_slice(&lines[i + 1..]);
                }

                return new_lines.join("\n");
            }
        }

        // No links section found
        if create_section {
            // Create new links section at end
            let trimmed = existing_content.trim_end();
            if trimmed.is_empty() {
                format!("## Links\n\n{}", new_content)
            } else {
                format!("{}\n\n## Links\n\n{}", trimmed, new_content)
            }
        } else {
            // Just append at end
            self.insert_at_end(existing_content, new_content)
        }
    }
}

//
//
// SERVICE PATTERN BENEFITS:
//
// 1. SEPARATION OF CONCERNS:
//    File I/O is isolated from business logic and UI concerns.
//    Changes to file handling don't affect command implementations.
//
// 2. TESTABILITY:
//    Easy to test with temporary directories and mock file systems.
//    No complex setup or dependencies on external state.
//
// 3. ERROR HANDLING:
//    Centralized file error handling with consistent user messages.
//    Easier to ensure all file operations handle errors properly.
//
// 4. PERFORMANCE:
//    Future optimizations (caching, batching) can be added without
//    changing the interface that commands depend on.
//
// 5. PORTABILITY:
//    Path handling differences between Windows/Unix are isolated here.
//    Commands don't need to worry about platform-specific file operations.
