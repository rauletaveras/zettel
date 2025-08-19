// crates/zettel-cli/src/commands/mod.rs - Command Handler Modules
//
// This module organizes all command implementations into logical groups.
// Each command family (id, note, list, etc.) gets its own module for
// maintainability and clear separation of concerns.
//
// MODULE ORGANIZATION:
// - init: Vault initialization (special case, no context needed)
// - id: ID manipulation commands (pure computation)
// - note: Note management commands (file creation, editing)
// - list: Vault listing and discovery commands
// - search: Content-based search commands
//
// DESIGN BENEFITS:
// - Each module can focus on its specific domain
// - Easy to add new command families
// - Clear separation between different types of operations
// - Modules can have their own helper functions and types

pub mod id;
pub mod init;
pub mod list;
pub mod note;
pub mod search;
pub mod template;
