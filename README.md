# Zettelkasten CLI Architecture

## Philosophy: Unix Tools That Compose

Following GNU/Unix principles:
- **Do one thing well**: Each command has a single responsibility
- **Compose through pipes**: Tools work together via stdin/stdout
- **Text-based interfaces**: Machine-readable output for scripting
- **Configuration via environment**: Standard Unix configuration patterns
- **Extensible through plugins**: Simple hook system for customization

## Core Architecture

```
zettel-core/                    # Core library (no CLI)
├── src/
│   ├── lib.rs                 # Public API
│   ├── vault/                 # Vault operations
│   ├── id/                    # ID manipulation
│   ├── note/                  # Note creation/parsing
│   ├── search/                # Search and indexing
│   └── config/                # Configuration management
├── tests/                     # Integration tests
└── Cargo.toml

zettel-cli/                     # CLI frontend
├── src/
│   ├── main.rs               # Command dispatcher
│   ├── commands/             # Individual commands
│   ├── output/               # Output formatters
│   └── completions/          # Shell completions
├── tests/
└── Cargo.toml

zettel-lsp/                     # LSP server (optional)
├── src/
│   ├── main.rs
│   └── handlers/
└── Cargo.toml

zettel-web/                     # Web interface (optional)
└── ...

scripts/                        # Helper scripts and integrations
├── editor-integrations/
│   ├── helix/
│   ├── vim/
│   ├── emacs/
│   └── vscode/
├── shell-completions/
└── installation/
```

## Command Structure (Unix-style)

### Core Commands

```bash
# Vault operations
zettel init [path]              # Initialize vault
zettel config <key> [value]     # Get/set configuration
zettel status                   # Vault health check

# Note creation (minimal, composable)
zettel id next-sibling <id>     # Generate next sibling ID
zettel id next-child <id>       # Generate next child ID  
zettel id validate <id>         # Validate ID format
zettel id parse <filename>      # Extract ID from filename

# Note operations
zettel note create <id> [title] # Create note with ID
zettel note template <id>       # Apply template to note
zettel note link <from> <to>    # Create bidirectional link

# Search and discovery
zettel list [pattern]           # List notes (JSON output)
zettel search <query>           # Search by content/title
zettel tree [root-id]           # Show hierarchy tree
zettel graph [format]           # Output relationship graph

# Metadata operations  
zettel meta get <file> <key>    # Get metadata field
zettel meta set <file> <key> <value>  # Set metadata field
zettel meta index               # Rebuild search index
```

### Composition Examples

```bash
# Create sibling with interactive title
current_id=$(zettel id parse "$file")
new_id=$(zettel id next-sibling "$current_id")
title=$(echo "Enter title:" | dmenu -p "Sibling:")
zettel note create "$new_id" "$title"

# Fuzzy search and open
zettel search --format=json "$query" | \
  jq -r '.[] | "\(.title) (\(.id))"' | \
  fzf | \
  grep -o '([^)]*)'  | \
  tr -d '()' | \
  xargs zettel note open

# Batch operations
zettel list --orphans | \
  xargs -I {} zettel note link {} "index"

# Export to different formats
zettel graph --format=dot | dot -Tsvg > vault-graph.svg
zettel tree | tree --fromfile
```

## Configuration System

### Environment Variables
```bash
export ZETTEL_VAULT="$HOME/notes"          # Vault root
export ZETTEL_EDITOR="helix"               # Default editor  
export ZETTEL_TEMPLATE_DIR="$VAULT/templates"
export ZETTEL_CONFIG="$HOME/.config/zettel/config.toml"
```

### Configuration File (`~/.config/zettel/config.toml`)
```toml
[vault]
default_path = "~/notes"
auto_index = true
backup_on_change = false

[id]
match_rule = "strict"           # strict|separator|fuzzy
separator = " - "
allow_unicode = false

[note]
default_template = "default"
auto_link_parent = true
auto_link_child = true
add_aliases = true

[search]
index_content = true
index_titles = true
fuzzy_threshold = 0.6

[output]
default_format = "human"       # human|json|csv|xml
color = "auto"                 # auto|always|never
pager = "auto"

[hooks]
pre_create = []                # Scripts to run before note creation
post_create = []               # Scripts to run after note creation
pre_link = []
post_link = []
```

## Plugin/Hook System

### Hook Scripts
```bash
# ~/.config/zettel/hooks/post-create.sh
#!/bin/bash
# Called after note creation: post-create.sh <note-path> <note-id> <title>
note_path="$1"
note_id="$2" 
title="$3"

# Example: Add creation timestamp
echo "Created: $(date -Iseconds)" >> "$note_path"

# Example: Update index file
echo "- [[$note_id]] $title" >> "$ZETTEL_VAULT/index.md"

# Example: Git commit
cd "$ZETTEL_VAULT" && git add "$note_path" && git commit -m "Add note: $note_id"
```

### Plugin Development
```rust
// Plugin trait for extending functionality
pub trait ZettelPlugin {
    fn name(&self) -> &str;
    fn commands(&self) -> Vec<Command>;
    fn hooks(&self) -> Vec<Hook>;
}

// Example plugin
pub struct GitPlugin;
impl ZettelPlugin for GitPlugin {
    fn name(&self) -> &str { "git" }
    
    fn commands(&self) -> Vec<Command> {
        vec![
            Command::new("git-status").about("Show vault git status"),
            Command::new("git-sync").about("Sync vault with remote"),
        ]
    }
}
```

## Output Formats (Machine Readable)

### JSON Output
```json
{
  "notes": [
    {
      "id": "1a2",
      "title": "My Note",
      "path": "/vault/1a2.md",
      "created": "2024-01-01T12:00:00Z",
      "modified": "2024-01-02T10:30:00Z",
      "parent": "1a",
      "children": ["1a2a", "1a2b"],
      "links": ["index", "2b3"],
      "tags": ["philosophy", "notes"]
    }
  ],
  "meta": {
    "vault_path": "/vault",
    "total_notes": 150,
    "query_time_ms": 23
  }
}
```

### CSV Output  
```csv
id,title,path,parent,children_count,link_count
1a2,"My Note",/vault/1a2.md,1a,2,5
```

## Error Handling & Reliability

### Error Codes
```rust
pub enum ZettelError {
    VaultNotFound = 1,
    InvalidId = 2,
    NoteExists = 3,
    TemplateError = 4,
    PermissionDenied = 5,
    ConfigError = 6,
    // ... standard Unix exit codes
}
```

### Validation & Safety
```bash
# Atomic operations with rollback
zettel note create "$id" --dry-run          # Preview without changes
zettel note create "$id" --backup           # Create backup first
zettel validate vault                       # Check vault integrity
```

## Performance & Scalability

### Indexing Strategy
```rust
// Fast index for large vaults
pub struct VaultIndex {
    notes: HashMap<String, NoteMetadata>,
    title_index: tantivy::Index,        // Full-text search
    link_graph: petgraph::Graph,        // Relationship graph
    file_watcher: notify::Watcher,      // Auto-update on changes
}
```

### Caching
```bash
# Cache locations following XDG spec
~/.cache/zettel/
├── index.db           # SQLite index for fast queries
├── search.idx         # Full-text search index  
└── vault-hash         # Hash of vault state for cache invalidation
```

## Installation & Distribution

### Package Structure
```bash
# Release artifacts
zettel-v1.0.0/
├── bin/
│   ├── zettel                 # Main CLI
│   ├── zettel-lsp            # LSP server
│   └── zettel-web            # Web server
├── share/
│   ├── man/man1/zettel.1     # Man pages
│   ├── completions/          # Shell completions
│   │   ├── zettel.bash
│   │   ├── zettel.zsh
│   │   └── zettel.fish
│   ├── templates/            # Default templates
│   └── examples/             # Example configurations
├── doc/
│   ├── README.md
│   ├── TUTORIAL.md
│   └── API.md
└── install.sh                # Installation script
```

### Distribution Methods
```bash
# Cargo
cargo install zettel-cli

# Package managers
brew install zettel-cli        # Homebrew
yay -S zettel-cli             # Arch AUR
nix-env -i zettel-cli         # Nix

# Docker
docker run --rm -v $PWD:/vault zettel/cli list

# Standalone binary
curl -L github.com/user/zettel/releases/latest/download/zettel-linux -o zettel
chmod +x zettel
```

## Editor Integrations

### Helix Integration
```toml
# ~/.config/helix/languages.toml
[[language]]
name = "markdown"
language-servers = ["marksman", "zettel-lsp"]

[language-server.zettel-lsp]
command = "zettel-lsp"
args = ["--vault", "$ZETTEL_VAULT"]
```

### Shell Completions
```bash
# Generated completions for all commands
zettel <TAB>                   # Shows: list, search, create, etc.
zettel note create 1a<TAB>     # Shows: 1a1, 1a2, 1aa (next available)
zettel search <TAB>            # Shows recent searches or note titles
```

## Testing Strategy

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_id_increment() {
        assert_eq!(increment_id("1"), "2");
        assert_eq!(increment_id("1a"), "1b");
        assert_eq!(increment_id("1z"), "1aa");
    }
    
    #[test]
    fn test_vault_operations() {
        let vault = TestVault::new();
        vault.create_note("1", "Test Note").unwrap();
        assert!(vault.note_exists("1"));
        assert_eq!(vault.get_title("1").unwrap(), "Test Note");
    }
}
```

### Integration Tests
```bash
#!/bin/bash
# tests/integration/basic_workflow.sh

set -e

# Setup test vault
VAULT=$(mktemp -d)
export ZETTEL_VAULT="$VAULT"

# Test basic workflow
zettel init
zettel note create "1" "First Note"
zettel note create "1a" "Child Note" --parent "1"
zettel note create "2" "Second Note"

# Verify structure
[[ $(zettel list --count) -eq 3 ]]
[[ $(zettel tree "1" | wc -l) -eq 2 ]]

# Cleanup
rm -rf "$VAULT"
echo "✓ Basic workflow test passed"
```

## Documentation Strategy

### Man Pages
```bash
man zettel                    # Overview and common commands
man zettel-note              # Note operations
man zettel-search            # Search functionality  
man zettel-config            # Configuration reference
```

### Interactive Help
```bash
zettel help                   # Command overview
zettel note --help           # Detailed command help
zettel --help search         # Alternative help syntax
zettel examples              # Show common usage patterns
```

## Migration & Compatibility

### Import from Other Systems
```bash
zettel import obsidian /path/to/vault     # Import Obsidian vault
zettel import zettlr /path/to/notes       # Import Zettlr notes
zettel import fs /path/to/markdown        # Import plain markdown files
```

### Export Options
```bash
zettel export obsidian /output/path       # Export for Obsidian
zettel export hugo /output/path           # Export for Hugo static site
zettel export json /output/file.json     # Machine-readable export
```

This architecture provides:
- **Composability**: Each command does one thing well
- **Extensibility**: Plugin system and hooks for customization  
- **Performance**: Efficient indexing and caching
- **Reliability**: Atomic operations and validation
- **Portability**: Works across Unix-like systems
- **Maintainability**: Clear separation of concerns and testing

# Zettelkasten CLI Project Structure

## Repository Layout

```
zettel/
├── README.md
├── LICENSE
├── CHANGELOG.md
├── Makefile                    # Main build system
├── justfile                    # Modern alternative to Makefile
├── .github/
│   └── workflows/
│       ├── ci.yml             # Continuous integration
│       ├── release.yml        # Automated releases
│       └── docs.yml           # Documentation builds
│
├── crates/                     # Rust workspace
│   ├── zettel-core/           # Core library
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs         # Public API
│   │   │   ├── vault.rs       # Vault operations
│   │   │   ├── id.rs          # ID manipulation
│   │   │   ├── note.rs        # Note management
│   │   │   ├── search.rs      # Search engine
│   │   │   ├── template.rs    # Template system
│   │   │   ├── config.rs      # Configuration
│   │   │   ├── hooks.rs       # Plugin hooks
│   │   │   └── error.rs       # Error types
│   │   ├── tests/
│   │   │   ├── integration.rs
│   │   │   └── fixtures/
│   │   └── benches/           # Performance benchmarks
│   │
│   ├── zettel-cli/            # Command-line interface
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── main.rs        # CLI entry point
│   │   │   ├── commands/      # Command implementations
│   │   │   ├── output/        # Output formatters
│   │   │   └── completions/   # Shell completions
│   │   └── tests/
│   │
│   ├── zettel-lsp/            # Language Server Protocol
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── handlers/      # LSP message handlers
│   │   │   └── capabilities.rs
│   │   └── tests/
│   │
│   └── zettel-web/            # Web interface (optional)
│       ├── Cargo.toml
│       ├── src/
│       └── static/
│
├── scripts/                   # Helper scripts
│   ├── install.sh            # Unix installation script
│   ├── install.ps1           # Windows installation script
│   ├── completions/          # Shell completion generators
│   │   ├── generate-bash.sh
│   │   ├── generate-zsh.sh
│   │   ├── generate-fish.sh
│   │   └── generate-powershell.ps1
│   ├── editor-integrations/  # Editor plugins
│   │   ├── helix/
│   │   │   ├── zettel.scm     # Tree-sitter queries
│   │   │   └── commands.toml  # Helix commands
│   │   ├── vim/
│   │   │   └── zettel.vim
│   │   ├── emacs/
│   │   │   └── zettel.el
│   │   └── vscode/
│   │       ├── package.json
│   │       └── src/
│   └── packaging/            # Package building
│       ├── homebrew/
│       ├── debian/
│       ├── arch/
│       └── nix/
│
├── docs/                     # Documentation
│   ├── book/                 # mdBook documentation
│   │   ├── book.toml
│   │   └── src/
│   │       ├── SUMMARY.md
│   │       ├── introduction.md
│   │       ├── installation.md
│   │       ├── tutorial.md
│   │       ├── commands/
│   │       ├── configuration.md
│   │       ├── plugins.md
│   │       └── api.md
│   ├── man/                  # Man pages
│   │   ├── zettel.1.md
│   │   ├── zettel-note.1.md
│   │   └── zettel-search.1.md
│   └── examples/             # Example configurations
│       ├── basic-config.toml
│       ├── advanced-config.toml
│       └── workflows/
│
├── templates/                # Default note templates
│   ├── default.md
│   ├── academic.md
│   ├── journal.md
│   └── project.md
│
├── tests/                    # Integration tests
│   ├── cli/                  # CLI integration tests
│   ├── fixtures/             # Test data
│   └── performance/          # Performance benchmarks
│
└── tools/                    # Development tools
    ├── test-vault/           # Sample vault for testing
    ├── benchmark.sh          # Performance testing
    └── coverage.sh           # Coverage reporting
```

This comprehensive build system provides:

1. **Multiple build tools** - Both traditional Makefile and modern justfile
2. **Comprehensive testing** - Unit, integration, and performance tests
3. **Quality assurance** - Linting, formatting, and security audits  
4. **Documentation generation** - Man pages, API docs, and user guide
5. **Multi-platform packaging** - Debian, RPM, Homebrew, Arch packages
6. **Shell integrations** - Completions and convenient aliases
7. **Automated releases** - GitHub Actions for CI/CD
8. **Easy installation** - Single script installation with dependency checking

The structure follows Rust best practices while providing Unix-style composability. Each component can be built, tested, and deployed independently, making it maintainable for both you and future contributors.
