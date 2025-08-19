// crates/zettel-cli/src/main.rs - CLI Application Entry Point
//
// This is the main entry point for the zettel CLI tool. It implements a Unix-style command structure
// where each subcommand does one thing well and can be composed with other tools.
//
// ARCHITECTURE OVERVIEW:
// ┌─────────────────┐    ┌──────────────────┐    ┌─────────────────────┐
// │   User Input    │───▶│   CLI Parser     │───▶│  Command Handlers   │
// │ (clap commands) │    │ (main function)  │    │ (commands/*.rs)     │
// └─────────────────┘    └──────────────────┘    └─────────────────────┘
//                                 │                        │
//                                 ▼                        ▼
//                        ┌──────────────────┐    ┌─────────────────────┐
//                        │     Context      │    │     Services        │
//                        │ (vault + config) │    │ (file I/O, editor)  │
//                        └──────────────────┘    └─────────────────────┘
//
// DESIGN PRINCIPLES:
// - Each command has a single responsibility (Unix philosophy)
// - Commands output machine-readable formats (JSON) for scripting
// - Context is passed explicitly (no global state)
// - Error handling follows Rust patterns (Result<T, E>)
// - Commands can be chained with pipes for complex workflows
//
// RUST PATTERNS USED:
// - Module system for clean separation of concerns
// - Result<T, E> for error propagation with anyhow for user-friendly messages
// - Dependency injection through Context struct
// - Command pattern with enum dispatch
//
// EXAMPLE USAGE:
// ```bash
// zettel init                                    # Initialize new vault
// zettel note create 1 "My First Note"          # Create root note
// zettel id next-child 1                        # Get next child ID (outputs: 1a)
// zettel list --json | jq '.[] | .id'           # Pipe output to other tools
// current_id=$(zettel id parse "current.md")    # Use in shell scripts
// ```

use anyhow::Result;
use clap::Parser;

// Module declarations - each module handles a specific concern
mod cli; // Command-line interface definitions (pure data structures)
mod commands; // Command implementations (business logic)
mod context; // Application context and dependency injection
mod services; // Infrastructure services (file I/O, editor integration)
mod stdin; // Stdin functions for command inputs

use cli::Cli;
use context::Context;

/// Main entry point - minimal and focused on orchestration
///
/// This function implements the typical CLI application pattern:
/// 1. Parse command-line arguments using clap
/// 2. Handle special cases (like init that doesn't need existing vault)
/// 3. Set up application context with services and configuration
/// 4. Dispatch to appropriate command handler
/// 5. Handle and report errors in user-friendly way
fn main() -> Result<()> {
    // Parse command-line arguments using clap's derive API
    // This is declarative - the CLI structure is defined in cli.rs
    let cli = Cli::parse();

    // Special case: init command doesn't need an existing vault
    // We handle it separately before setting up context to avoid circular dependency
    // (can't create context for non-existent vault, but init creates the vault)
    if let cli::Commands::Init { path } = &cli.command {
        return commands::init::handle(path.clone());
    }

    // Set up application context with vault and configuration
    // Context implements dependency injection - it provides services to commands
    // This ensures commands don't directly depend on file system or configuration
    let ctx = Context::new(cli.vault)?;

    // Dispatch to appropriate command handler using pattern matching
    // Each command family is implemented in its own module for maintainability
    // This is the Command pattern - each variant maps to a specific handler
    match cli.command {
        cli::Commands::Id(cmd) => commands::id::handle(&ctx, cmd),
        cli::Commands::Note(cmd) => commands::note::handle(&ctx, cmd),
        cli::Commands::Template(cmd) => commands::template::handle(&ctx, cmd),
        cli::Commands::List { full_paths, json } => commands::list::handle(&ctx, full_paths, json),
        cli::Commands::Search { query } => commands::search::handle(&ctx, query),
        cli::Commands::Init { .. } => unreachable!(), // Already handled above
    }
}

// ARCHITECTURE BENEFITS:
//
// 1. SEPARATION OF CONCERNS:
//    - main.rs: Orchestration only (parse, dispatch, handle errors)
//    - cli.rs: Command definitions (what commands exist)
//    - commands/*.rs: Business logic (what commands do)
//    - services/*.rs: Infrastructure (how to do file I/O, etc.)
//    - context.rs: Configuration and dependency injection
//
// 2. TESTABILITY:
//    - Each command can be tested independently by providing mock context
//    - Services can be tested with temporary directories
//    - Business logic is separated from I/O concerns
//
// 3. MAINTAINABILITY:
//    - Adding new commands means adding one file in commands/
//    - Changing file operations only affects services/vault.rs
//    - CLI definitions are centralized in cli.rs
//
// 4. UNIX PHILOSOPHY:
//    - Each command does one thing well
//    - Commands can be composed with pipes and shell scripts
//    - Output formats support both human and machine consumption
//
// 5. RUST SAFETY:
//    - Compile-time guarantees about error handling
//    - No null pointer dereferences or memory leaks
//    - Pattern matching ensures all cases are handled
