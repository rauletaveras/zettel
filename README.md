# Zettel - CLI Zettelkasten Manager

A command-line implementation of the Luhmann-style Zettelkasten system, inspired by and designed for feature parity with the [luhman-obsidian-plugin](https://github.com/Dyldog/luhman-obsidian-plugin).

## What is a Zettelkasten?

A Zettelkasten is a note-taking system that uses hierarchical alphanumeric IDs to encode relationships between ideas. Notes are organized by IDs like `1`, `1a`, `1a2`, `2b3` where the structure itself represents conceptual connections:

- **Siblings**: Same level (`1` → `2` → `3`)
- **Children**: Deeper elaboration (`1` → `1a` → `1a1`)
- **Branching**: Related tangents (`1a` → `1b`)

## Features

### ✅ Core ID System
- Complete Luhmann ID parsing and generation
- Next sibling generation (`zettel id next-sibling 1a` → `1b`)
- Next child generation (`zettel id next-child 1a` → `1a1`)
- ID validation and extraction from filenames
- Configurable matching rules (strict/separator/fuzzy)

### ✅ Note Management
- Create notes with generated IDs (`zettel note create 1a "My Note"`)
- Open notes by ID (`zettel note open 1a`)
- Display note content (`zettel note show 1a`)
- List all notes with JSON output for scripting

### ✅ Search & Discovery
- Text-based search across titles and content
- Fuzzy filename matching
- Machine-readable output formats

### ✅ Vault Operations
- Initialize new vaults (`zettel init`)
- Configuration management
- Cross-platform file operations

### ⏳ Missing for Feature Parity

**Templates System**
- Custom template file support
- Template validation and placeholder substitution

**Bidirectional Linking**
- Automatic parent↔child link insertion
- Link format configuration

**Advanced Operations**
- Link insertion commands
- Text selection processing
- Hierarchy reorganization

## Installation

### From Source
```bash
# Clone the repository
git clone https://github.com/rauletaveras/zettel
cd zettel

# Build and install
cargo install --path crates/zettel-cli
```

### Using Cargo
```bash
cargo install zettel-cli
```

## Quick Start

```bash
# Initialize a new vault
zettel init ~/my-notes
cd ~/my-notes

# Create your first note
zettel note create 1 "My First Note"

# Create a child note
zettel note create 1a "Subtopic of First Note"

# Create a sibling
zettel note create 2 "Second Main Topic"

# List all notes
zettel list

# Search notes
zettel search "main topic"
```

## Usage Examples

### ID Operations
```bash
# Generate next sibling ID
zettel id next-sibling 1a2  # Output: 1a3

# Generate child ID  
zettel id next-child 1a     # Output: 1a1

# Parse ID from filename
zettel id parse "1a2 - My Note.md"  # Output: 1a2

# Validate ID format
zettel id validate 1a2b
```

### Note Creation
```bash
# Create note with title
zettel note create 1a "Machine Learning Basics"

# Create and open in editor
zettel note create 1b "Neural Networks" --open

# Pipe operations
next_id=$(zettel id next-sibling 1a)
zettel note create "$next_id" "Related Topic"
```

### Search and Discovery
```bash
# Human-readable list
zettel list

# Machine-readable output
zettel list --json | jq '.[] | .id'

# Search content
zettel search "machine learning"
```

## Configuration

Configuration files are stored in `.zettel/config.toml`:

```toml
[id]
match_rule = "fuzzy"        # strict|separator|fuzzy
separator = " - "
allow_unicode = false

[note]
add_title = false
add_alias = false
extension = "md"

[editor]
# command = "helix"         # Override default editor
```

### Environment Variables
```bash
export ZETTEL_VAULT="~/notes"      # Default vault location
export ZETTEL_EDITOR="helix"       # Preferred editor
export ZETTEL_MATCH_RULE="fuzzy"   # ID matching rule
```

## Integration

### Shell Scripts
```bash
# Create sibling of current note
current_id=$(zettel id parse "$CURRENT_FILE")
new_id=$(zettel id next-sibling "$current_id")
zettel note create "$new_id" "New Idea"

# Batch operations
zettel list --json | jq -r '.[].id' | \
  xargs -I {} zettel id validate {}
```

### Editor Integration
The CLI is designed for editor integration through shell commands and can be easily integrated with Helix, Vim, Emacs, or VS Code.

## Roadmap

- [ ] Template system with placeholder substitution
- [ ] Automatic bidirectional linking
- [ ] Link insertion commands
- [ ] LSP server for editor integration
- [ ] Web interface
- [ ] Import/export from other systems

## Contributing

Contributions welcome! The project uses a modular architecture with clear separation between CLI interface, business logic, and file operations.

## License

MIT License - see LICENSE file for details.

## Acknowledgments

Inspired by the excellent [luhman-obsidian-plugin](https://github.com/Dyldog/luhman-obsidian-plugin) by Dyldog. This CLI implementation aims to bring the same powerful Zettelkasten workflow to command-line users and enable broader integration possibilities.
