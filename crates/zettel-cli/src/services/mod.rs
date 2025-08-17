// crates/zettel-cli/src/services/mod.rs - Service Layer Modules
//
// This module organizes infrastructure services that handle external concerns
// like file I/O, editor integration, and other system interactions.
//
// SERVICE LAYER BENEFITS:
// - Separation of concerns: Business logic doesn't handle infrastructure
// - Testability: Services can be mocked for unit testing
// - Maintainability: Changes to external systems are isolated
// - Reusability: Services can be shared across different commands

pub mod editor;
pub mod vault;

pub use editor::EditorService;
pub use vault::VaultService;
