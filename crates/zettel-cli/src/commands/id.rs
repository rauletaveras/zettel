// crates/zettel-cli/src/commands/id.rs - ID manipulation commands
use anyhow::Result;
use zettel_core::id::Id;

use crate::cli::IdCommands;
use crate::context::Context;

pub fn handle(ctx: &Context, cmd: IdCommands) -> Result<()> {
    let id_manager = ctx.get_id_manager();

    match cmd {
        IdCommands::NextSibling { id } => {
            let current_id = Id::parse(&id)?;
            let next_id = id_manager.next_available_sibling(&current_id)?;
            println!("{}", next_id);
        }

        IdCommands::NextChild { id } => {
            let parent_id = Id::parse(&id)?;
            let child_id = id_manager.next_available_child(&parent_id);
            println!("{}", child_id);
        }

        IdCommands::Parse { filename } => {
            if let Some(id) = id_manager.extract_from_filename(&filename) {
                println!("{}", id);
            } else {
                eprintln!("No valid ID found in filename: {}", filename);
                std::process::exit(1);
            }
        }

        IdCommands::Validate { id } => match Id::parse(&id) {
            Ok(parsed_id) => {
                println!("✅ Valid ID: {}", parsed_id);
                println!("   Depth: {}", parsed_id.depth());
                println!("   Root: {}", parsed_id.is_root());
                if let Ok(Some(parent)) = parsed_id.parent() {
                    println!("   Parent: {}", parent);
                }
            }
            Err(e) => {
                eprintln!("❌ Invalid ID: {}", e);
                std::process::exit(1);
            }
        },
    }

    Ok(())
}
