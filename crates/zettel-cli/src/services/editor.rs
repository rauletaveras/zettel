// crates/zettel-cli/src/services/editor.rs - Editor Integration Service
//
// This service handles text editor operations and integration with external
// editors. It provides a clean abstraction over platform-specific editor
// launching and configuration.

use anyhow::Result;
use std::env;
use std::path::Path;
use std::process::Command;

/// Handles text editor operations
///
/// This service manages editor integration - launching editors,
/// determining which editor to use, and handling platform differences.
///
/// EDITOR SELECTION HIERARCHY:
/// 1. ZETTEL_EDITOR environment variable (zettel-specific)
/// 2. EDITOR environment variable (standard Unix)
/// 3. Platform-specific default (vim on Unix, notepad on Windows)
///
/// DESIGN PHILOSOPHY:
/// - Respect user preferences through environment variables
/// - Provide sensible defaults for new users
/// - Handle platform differences transparently
/// - Support both simple and sophisticated editor configurations
///
/// FUTURE ENHANCEMENTS:
/// - Support for editor arguments (cursor positioning, etc.)
/// - Integration with specific editors (LSP, syntax highlighting)
/// - Support for GUI vs terminal editor detection
/// - Configuration through vault settings
pub struct EditorService;

impl EditorService {
    /// Get the editor command to use for opening files
    ///
    /// This implements the editor selection hierarchy and provides
    /// platform-appropriate defaults for users who haven't configured
    /// a specific editor.
    ///
    /// ENVIRONMENT VARIABLES:
    /// - ZETTEL_EDITOR: Zettel-specific editor override
    /// - EDITOR: Standard Unix editor setting
    ///
    /// PLATFORM DEFAULTS:
    /// - Unix-like systems: vim (widely available)
    /// - Windows: notepad (always available)
    ///
    /// EXAMPLES:
    /// ```bash
    /// export ZETTEL_EDITOR="helix"        # Use Helix for zettel editing
    /// export EDITOR="code --wait"         # Use VS Code for general editing
    /// ```
    ///
    /// FUTURE: Support for complex editor configurations:
    /// ```bash
    /// export ZETTEL_EDITOR="helix +{line}:{col}"  # Cursor positioning
    /// export ZETTEL_EDITOR="code --wait --goto {file}:{line}:{col}"
    /// ```
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
    ///
    /// Launches the configured editor with the given file path and waits
    /// for the editor to exit. Provides feedback if the editor exits with
    /// an error code.
    ///
    /// PROCESS HANDLING:
    /// - Spawns editor as child process
    /// - Waits for editor to complete (synchronous)
    /// - Checks exit status and reports errors
    /// - Inherits stdio (editor can interact with terminal)
    ///
    /// ERROR HANDLING:
    /// - Reports if editor command cannot be found
    /// - Reports if editor exits with non-zero status
    /// - Provides helpful context for troubleshooting
    ///
    /// FUTURE ENHANCEMENTS:
    /// - Support for non-blocking editor launch
    /// - Better error messages with editor-specific help
    /// - Support for editor-specific features (line/column positioning)
    ///
    /// EXAMPLES:
    /// ```rust
    /// EditorService::open_file(Path::new("1a.md"))?;
    /// ```
    pub fn open_file(path: &Path) -> Result<()> {
        let editor = Self::get_editor_command();

        // Launch editor and wait for completion
        let status = Command::new(&editor)
            .arg(path)
            .status()
            .map_err(|e| anyhow::anyhow!(
                "Failed to launch editor '{}': {}\n\nTips:\n- Check that '{}' is installed and in PATH\n- Set ZETTEL_EDITOR or EDITOR environment variable",
                editor, e, editor
            ))?;

        // Check if editor exited successfully
        if !status.success() {
            eprintln!(
                "⚠️ Editor '{}' exited with error code: {:?}",
                editor,
                status.code()
            );
            eprintln!("The file was created successfully, but the editor had an issue.");
        }

        Ok(())
    }

    /// Get available editors on the system (future feature)
    ///
    /// This could help users discover what editors are available
    /// and provide suggestions for configuration.
    #[allow(dead_code)]
    pub fn detect_available_editors() -> Vec<String> {
        // TODO: Implement editor detection
        // Could check PATH for common editors: vim, nvim, helix, code, etc.
        vec![]
    }
}

// EDITOR INTEGRATION PATTERNS:
//
// This service enables several common workflows:
//
// 1. IMMEDIATE EDITING:
//    zettel note create 1a "My Note" --open
//    # Creates note and immediately opens for editing
//
// 2. DEFERRED EDITING:
//    zettel note create 1a "My Note"  # Create without opening
//    zettel note open 1a              # Open later when ready to edit
//
// 3. EDITOR-SPECIFIC WORKFLOWS:
//    export ZETTEL_EDITOR="helix"
//    zettel note create 1a --open     # Always opens in Helix
//
// 4. INTEGRATION WITH EDITOR PLUGINS:
//    # From within Helix, create sibling of current note
//    :sh zettel id next-sibling $(zettel id parse %) | xargs zettel note create --open
//
// 5. BATCH OPERATIONS:
//    for title in "Introduction" "Methods" "Results" "Discussion"; do
//      id=$(zettel id next-sibling $prev_id)
//      zettel note create "$id" "$title"
//      prev_id="$id"
//    done
//    # Then open each for editing as needed
//
// FUTURE INTEGRATIONS:
//
// The service could be extended to support:
// - LSP integration for better editing experience
// - Syntax highlighting configuration
// - Custom keybindings for zettel operations
// - Integration with specific editor plugins
// - Support for collaborative editing tools
