// crates/zettel-cli/src/commands/search.rs - Note Search Command
//
// This command implements simple text-based search across all notes in the vault.
// It searches both filenames and file content using case-insensitive matching.

use anyhow::Result;

use crate::context::Context;

/// Search notes by content or filename
///
/// This implements simple text-based search across all notes in the vault.
/// It searches both filenames and file content using case-insensitive matching.
pub fn handle(ctx: &Context, query: Option<String>) -> Result<()> {
    // Get query from argument or stdin
    let query_str = crate::stdin::read_input_or_stdin(query.as_deref())?;

    let files = ctx.vault_service.get_vault_files();
    let id_manager = ctx.get_id_manager();
    let query_lower = query_str.to_lowercase();

    println!("🔍 Searching for: {}", query_str);
    println!();

    for file in files {
        if let Some(filename) = file.file_name().and_then(|n| n.to_str()) {
            if let Some(id) = id_manager.extract_from_filename(filename) {
                // Search in filename first
                if filename.to_lowercase().contains(&query_lower) {
                    println!("📄 {} (filename match)", id);
                    continue;
                }

                // Search in content
                if let Ok(content) = ctx.vault_service.read_file(&file) {
                    if content.to_lowercase().contains(&query_lower) {
                        let title = content
                            .lines()
                            .next()
                            .unwrap_or("")
                            .strip_prefix("# ")
                            .unwrap_or("No title");

                        println!("📄 {}: {}", id, title);
                    }
                }
            }
        }
    }

    Ok(())
}
