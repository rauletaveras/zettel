use anyhow::Result;
use clap::Parser;

mod cli;
mod commands;
mod context;
mod services;

use cli::Cli;
use context::Context;

/// Main entry point - minimal and focused
///
/// This now just handles:
/// 1. Parse CLI arguments
/// 2. Set up application context
/// 3. Dispatch to command handlers
/// 4. Handle top-level errors
fn main() -> Result<()> {
    let cli = Cli::parse();

    // Special case: init doesn't need existing vault
    if let cli::Commands::Init { path } = &cli.command {
        return commands::init::handle(path.clone());
    }

    // Set up context for all other commands
    let ctx = Context::new(cli.vault)?;

    // Dispatch to appropriate command handler
    match cli.command {
        cli::Commands::Id(cmd) => commands::id::handle(&ctx, cmd),
        cli::Commands::Note(cmd) => commands::note::handle(&ctx, cmd),
        cli::Commands::List { full_paths, json } => commands::list::handle(&ctx, full_paths, json),
        cli::Commands::Search { query } => commands::search::handle(&ctx, query),
        cli::Commands::Init { .. } => unreachable!(), // Handled above
    }
}
