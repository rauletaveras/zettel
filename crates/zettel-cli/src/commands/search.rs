// crates/zettel-cli/src/commands/search.rs - Note search command
use anyhow::Result;

use crate::context::Context;

pub fn handle(ctx: &Context, query: String) -> Result<()> {
    let files = ctx.vault_service.get_vault_files();
    let id_manager = ctx.get_id_manager();
    let query_lower = query.to_lowercase();

    println!("🔍 Searching for: {}", query);
    println!();

    for file in files {
        if let Some(filename) = file.file_name().and_then(|n| n.to_str()) {
            if let Some(id) = id_manager.extract_from_filename(filename) {
                // Search in filename first (faster)
                if filename.to_lowercase().contains(&query_lower) {
                    println!("📄 {} (filename match)", id);
                    continue;
                }

                // Search in content (slower, but more thorough)
                if let Ok(content) = ctx.vault_service.read_file(&file) {
                    if content.to_lowercase().contains(&query_lower) {
                        // Try to extract title from first line for better display
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
