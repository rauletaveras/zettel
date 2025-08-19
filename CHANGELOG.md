# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
#### Template system
- Template validation: Check a file complies with zettel template requirements
  - `zettel template validate <file>`
- Generate basic default templates: simple, academic, etc
  - `zettel template example <template-type>` (possible values: basic, academic, meeting, daily)
- If config options set with default template, `zettel note create` will create the note following the given template.

## [0.1.0] - 2025-08-19

### Added

#### Core ID System
- **ID Generation**: Generate next sibling and child IDs following Luhmann's alternating number/letter pattern
  - `zettel id next-sibling <id>` - Generate next available sibling ID
  - `zettel id next-child <id>` - Generate next available child ID
- **ID Validation**: Comprehensive ID format validation with detailed error messages
  - `zettel id validate <id>` - Validate single ID with structural information
  - `zettel id validate-batch` - Validate multiple IDs from stdin
- **ID Parsing**: Extract IDs from filenames with configurable matching rules
  - `zettel id parse <filename>` - Extract ID from filename
  - `zettel id extract-ids` - Extract IDs from multiple files
- **Conflict Detection**: Automatic checking for existing IDs when generating new ones

#### Note Management
- **Note Creation**: Create notes with automatic file naming and initial content
  - `zettel note create <id> [title]` - Create new note with optional title
  - `--open` flag to immediately open in editor after creation
- **Note Operations**: Basic note manipulation commands
  - `zettel note open <id>` - Open existing note in configured editor
  - `zettel note show <id>` - Display note content to stdout
- **Filename Handling**: Automatic filename generation with ID and optional title

#### Vault Management
- **Vault Initialization**: Set up new zettelkasten vault structure
  - `zettel init [path]` - Initialize vault with .zettel directory and config
- **Note Discovery**: List and search existing notes
  - `zettel list` - Human-readable note listing with titles
  - `zettel list --json` - Machine-readable JSON output for scripting
  - `zettel list --full-paths` - Show complete file paths
  - `zettel search <query>` - Basic text search in filenames and content

#### Configuration System
- **Flexible ID Matching**: Three configurable matching strategies
  - `strict`: Filename exactly matches ID (e.g., "1a2.md")
  - `separator`: ID followed by separator and title (e.g., "1a2 - Title.md")
  - `fuzzy`: ID at start with any separator (e.g., "1a2_note.md")
- **Vault Configuration**: Per-vault settings via `.zettel/config.toml`
- **Global Configuration**: User-wide settings via `~/.config/zettel/config.toml`
- **Environment Variables**: Override settings via `ZETTEL_VAULT`, `ZETTEL_EDITOR`

#### Editor Integration
- **Multi-Editor Support**: Automatic editor detection and launching
  - Respects `ZETTEL_EDITOR`, `EDITOR` environment variables
  - Platform-appropriate defaults (vim on Unix, notepad on Windows)
- **Seamless Workflow**: Create and immediately edit notes

#### Unix Philosophy Design
- **Composable Commands**: Each command does one thing well
- **Pipeable Output**: Commands support stdin/stdout for scripting
- **Exit Codes**: Proper error codes for script integration

### Known Limitations
- No bidirectional linking between parent and child notes
- No custom template system for note creation
- Basic text search only (no full-text indexing)

### Dependencies
- **clap**: Command-line argument parsing
- **serde**: Configuration serialization
- **regex**: ID pattern matching
- **anyhow**: Error handling
- **thiserror**: Custom error types

[0.1.0]: https://github.com/rauletaveras/zettel/releases/tag/v0.1.0
