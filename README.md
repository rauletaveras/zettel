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

## Build System (Makefile)

```makefile
# Main Makefile for Zettelkasten CLI
SHELL := /bin/bash
.PHONY: help build test install clean docs release

# Configuration
CARGO := cargo
TARGET_DIR := target
INSTALL_PREFIX := /usr/local
VERSION := $(shell grep '^version' crates/zettel-cli/Cargo.toml | cut -d'"' -f2)

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

# Development
build: ## Build all crates
	$(CARGO) build --workspace

build-release: ## Build optimized release binaries
	$(CARGO) build --workspace --release

test: ## Run all tests
	$(CARGO) test --workspace

test-integration: ## Run integration tests
	./tests/run-integration-tests.sh

bench: ## Run performance benchmarks
	$(CARGO) bench --workspace

# Quality assurance
check: ## Run clippy and formatting checks
	$(CARGO) clippy --workspace -- -D warnings
	$(CARGO) fmt --check

fmt: ## Format code
	$(CARGO) fmt

audit: ## Security audit
	$(CARGO) audit

# Documentation
docs: ## Build documentation
	$(CARGO) doc --workspace --no-deps
	mdbook build docs/book

docs-serve: ## Serve documentation locally
	mdbook serve docs/book

man-pages: ## Generate man pages from markdown
	for page in docs/man/*.1.md; do \
		pandoc -s -t man "$$page" -o "$${page%.md}"; \
	done

# Installation
install: build-release ## Install to system
	install -Dm755 $(TARGET_DIR)/release/zettel $(INSTALL_PREFIX)/bin/zettel
	install -Dm755 $(TARGET_DIR)/release/zettel-lsp $(INSTALL_PREFIX)/bin/zettel-lsp
	install -Dm644 docs/man/zettel.1 $(INSTALL_PREFIX)/share/man/man1/zettel.1
	install -Dm644 scripts/completions/zettel.bash $(INSTALL_PREFIX)/share/bash-completion/completions/zettel
	install -Dm644 scripts/completions/zettel.zsh $(INSTALL_PREFIX)/share/zsh/site-functions/_zettel
	install -Dm644 scripts/completions/zettel.fish $(INSTALL_PREFIX)/share/fish/vendor_completions.d/zettel.fish

install-dev: build ## Install development version
	$(CARGO) install --path crates/zettel-cli --force
	$(CARGO) install --path crates/zettel-lsp --force

uninstall: ## Uninstall from system
	rm -f $(INSTALL_PREFIX)/bin/zettel
	rm -f $(INSTALL_PREFIX)/bin/zettel-lsp
	rm -f $(INSTALL_PREFIX)/share/man/man1/zettel.1
	rm -f $(INSTALL_PREFIX)/share/bash-completion/completions/zettel
	rm -f $(INSTALL_PREFIX)/share/zsh/site-functions/_zettel
	rm -f $(INSTALL_PREFIX)/share/fish/vendor_completions.d/zettel.fish

# Shell completions
completions: ## Generate shell completions
	mkdir -p scripts/completions
	$(TARGET_DIR)/release/zettel completions bash > scripts/completions/zettel.bash
	$(TARGET_DIR)/release/zettel completions zsh > scripts/completions/zettel.zsh
	$(TARGET_DIR)/release/zettel completions fish > scripts/completions/zettel.fish

# Packaging
package-deb: build-release ## Build Debian package
	./scripts/packaging/debian/build-deb.sh $(VERSION)

package-rpm: build-release ## Build RPM package
	./scripts/packaging/rpm/build-rpm.sh $(VERSION)

package-homebrew: ## Update Homebrew formula
	./scripts/packaging/homebrew/update-formula.sh $(VERSION)

package-arch: ## Build Arch Linux package
	./scripts/packaging/arch/build-pkg.sh $(VERSION)

# Release
release: test build-release docs completions ## Prepare release
	git tag -a v$(VERSION) -m "Release v$(VERSION)"
	./scripts/create-release-artifacts.sh $(VERSION)

release-upload: ## Upload release to GitHub
	gh release create v$(VERSION) \
		--title "v$(VERSION)" \
		--notes-file CHANGELOG.md \
		target/release/zettel-*

# Cleanup
clean: ## Clean build artifacts
	$(CARGO) clean
	rm -rf docs/book/book
	rm -f docs/man/*.1

clean-all: clean ## Clean everything including caches
	rm -rf ~/.cargo/registry/cache/
```

## Modern Build System (justfile)

```justfile
# Justfile for Zettelkasten CLI - modern alternative to Makefile
# Usage: just <command>

# Configuration
cargo := "cargo"
target_dir := "target"
version := `grep '^version' crates/zettel-cli/Cargo.toml | cut -d'"' -f2`

# Default recipe
default:
    @just --list

# Development commands
build:
    {{cargo}} build --workspace

build-release:
    {{cargo}} build --workspace --release

test:
    {{cargo}} test --workspace

test-integration:
    ./tests/run-integration-tests.sh

watch:
    {{cargo}} watch -x "build --workspace"

# Code quality
check:
    {{cargo}} clippy --workspace -- -D warnings
    {{cargo}} fmt --check

fmt:
    {{cargo}} fmt

audit:
    {{cargo}} audit

# Benchmarks and performance
bench:
    {{cargo}} bench --workspace

profile binary="zettel" args="":
    {{cargo}} build --release
    perf record --call-graph=dwarf {{target_dir}}/release/{{binary}} {{args}}
    perf report

# Documentation
docs:
    {{cargo}} doc --workspace --no-deps --open

docs-book:
    mdbook build docs/book

docs-serve:
    mdbook serve docs/book

# Installation
install: build-release
    {{cargo}} install --path crates/zettel-cli --force
    {{cargo}} install --path crates/zettel-lsp --force

# Shell completions
completions: build-release
    mkdir -p scripts/completions
    {{target_dir}}/release/zettel completions bash > scripts/completions/zettel.bash
    {{target_dir}}/release/zettel completions zsh > scripts/completions/zettel.zsh
    {{target_dir}}/release/zettel completions fish > scripts/completions/zettel.fish

# Testing with different configurations
test-minimal:
    ZETTEL_CONFIG=tests/configs/minimal.toml {{cargo}} test

test-advanced:
    ZETTEL_CONFIG=tests/configs/advanced.toml {{cargo}} test

test-large-vault:
    ./tools/benchmark.sh large-vault

# Packaging
package-all: build-release completions
    ./scripts/create-packages.sh {{version}}

# Release workflow
pre-release: test check build-release docs-book completions
    echo "Ready for release {{version}}"

release tag=version: pre-release
    git tag -a v{{tag}} -m "Release v{{tag}}"
    git push origin v{{tag}}

# Development setup
setup:
    rustup component add clippy rustfmt
    cargo install mdbook
    cargo install cargo-watch
    cargo install cargo-audit

# Clean up
clean:
    {{cargo}} clean

clean-all: clean
    rm -rf docs/book/book
    rm -f scripts/completions/*
```

## Cargo Workspace Configuration

```toml
# Cargo.toml (workspace root)
[workspace]
members = [
    "crates/zettel-core",
    "crates/zettel-cli", 
    "crates/zettel-lsp",
    "crates/zettel-web",
]
exclude = []

[workspace.package]
version = "0.1.0"
authors = ["Your Name <email@example.com>"]
license = "MIT OR Apache-2.0"
repository = "https://github.com/username/zettel"
homepage = "https://zettel.dev"
documentation = "https://docs.rs/zettel-core"
keywords = ["zettelkasten", "notes", "cli", "knowledge-management"]
categories = ["command-line-utilities", "text-processing"]
edition = "2021"
rust-version = "1.70"

[workspace.dependencies]
# Shared dependencies across all crates
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"
tokio = { version = "1.0", features = ["full"] }
anyhow = "1.0"
thiserror = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
clap = { version = "4.0", features = ["derive", "env"] }
regex = "1.0"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.0", features = ["v4", "serde"] }
walkdir = "2.0"
tempfile = "3.0"
indexmap = "2.0"
dashmap = "5.0"
rayon = "1.0"
fuzzy-matcher = "0.3"
tantivy = "0.21"
notify = "6.0"
crossbeam-channel = "0.5"

# CLI-specific
atty = "0.2"
console = "0.15"
indicatif = "0.17"

# LSP-specific
tower-lsp = "0.20"
lsp-types = "0.94"

# Optional features
git2 = { version = "0.18", optional = true }
syntect = { version = "5.0", optional = true }

[workspace.metadata.docs.rs]
all-features = true
rustdoc-args = ["--cfg", "docsrs"]
```

## Core Crate Configuration

```toml
# crates/zettel-core/Cargo.toml
[package]
name = "zettel-core"
description = "Core library for Luhmann-style Zettelkasten management"
readme = "README.md"
version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
homepage.workspace = true
documentation.workspace = true
keywords.workspace = true
categories.workspace = true
edition.workspace = true
rust-version.workspace = true

[features]
default = ["search", "templates"]
search = ["tantivy", "fuzzy-matcher"]
templates = ["handlebars"]
git = ["git2"]
syntax-highlighting = ["syntect"]
performance = ["rayon", "dashmap"]

[dependencies]
# Core dependencies
serde.workspace = true
serde_json.workspace = true
serde_yaml.workspace = true
anyhow.workspace = true
thiserror.workspace = true
regex.workspace = true
chrono.workspace = true
uuid.workspace = true
walkdir.workspace = true
indexmap.workspace = true
crossbeam-channel.workspace = true
notify.workspace = true
tracing.workspace = true

# Optional dependencies
tantivy = { workspace = true, optional = true }
fuzzy-matcher = { workspace = true, optional = true }
handlebars = { version = "4.0", optional = true }
git2 = { workspace = true, optional = true }
syntect = { workspace = true, optional = true }
rayon = { workspace = true, optional = true }
dashmap = { workspace = true, optional = true }

[dev-dependencies]
tempfile.workspace = true
tokio = { workspace = true, features = ["test-util", "macros"] }
criterion = "0.5"
proptest = "1.0"

[[bench]]
name = "id_generation"
harness = false

[[bench]]
name = "search_performance"
harness = false
required-features = ["search"]
```

## CLI Crate Configuration

```toml
# crates/zettel-cli/Cargo.toml
[package]
name = "zettel-cli"
description = "Command-line interface for Zettelkasten management"
readme = "README.md"
version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
homepage.workspace = true
keywords.workspace = true
categories.workspace = true
edition.workspace = true
rust-version.workspace = true

[[bin]]
name = "zettel"
path = "src/main.rs"

[features]
default = ["full"]
full = ["search", "templates", "git", "completions"]
search = ["zettel-core/search"]
templates = ["zettel-core/templates"]
git = ["zettel-core/git"]
completions = ["clap_complete"]

[dependencies]
zettel-core = { path = "../zettel-core", features = ["search", "templates"] }
clap.workspace = true
clap_complete = { version = "4.0", optional = true }
serde.workspace = true
serde_json.workspace = true
serde_yaml.workspace = true
anyhow.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true
console.workspace = true
atty.workspace = true
indicatif.workspace = true

[dev-dependencies]
tempfile.workspace = true
assert_cmd = "2.0"
predicates = "3.0"
```

## Installation Script

```bash
#!/bin/bash
# scripts/install.sh - Installation script for Zettelkasten CLI

set -euo pipefail

# Configuration
REPO_URL="https://github.com/username/zettel"
INSTALL_PREFIX="${INSTALL_PREFIX:-/usr/local}"
PLATFORM="$(uname -s)"
ARCH="$(uname -m)"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

check_dependencies() {
    local missing_deps=()
    
    if ! command -v curl >/dev/null 2>&1; then
        missing_deps+=("curl")
    fi
    
    if ! command -v tar >/dev/null 2>&1; then
        missing_deps+=("tar")
    fi
    
    if [[ ${#missing_deps[@]} -gt 0 ]]; then
        log_error "Missing required dependencies: ${missing_deps[*]}"
        log_info "Please install them and try again"
        exit 1
    fi
}

detect_platform() {
    case "$PLATFORM" in
        "Linux")
            case "$ARCH" in
                "x86_64") echo "linux-x86_64" ;;
                "aarch64") echo "linux-aarch64" ;;
                *) log_error "Unsupported architecture: $ARCH"; exit 1 ;;
            esac
            ;;
        "Darwin")
            case "$ARCH" in
                "x86_64") echo "macos-x86_64" ;;
                "arm64") echo "macos-aarch64" ;;
                *) log_error "Unsupported architecture: $ARCH"; exit 1 ;;
            esac
            ;;
        *)
            log_error "Unsupported platform: $PLATFORM"
            exit 1
            ;;
    esac
}

get_latest_version() {
    curl -s "https://api.github.com/repos/username/zettel/releases/latest" | \
        grep '"tag_name":' | \
        sed -E 's/.*"([^"]+)".*/\1/' | \
        sed 's/^v//'
}

download_and_install() {
    local version="$1"
    local platform="$2"
    local temp_dir
    
    temp_dir=$(mktemp -d)
    trap "rm -rf $temp_dir" EXIT
    
    local download_url="${REPO_URL}/releases/download/v${version}/zettel-${version}-${platform}.tar.gz"
    local archive_path="${temp_dir}/zettel.tar.gz"
    
    log_info "Downloading zettel v${version} for ${platform}..."
    if ! curl -L -o "$archive_path" "$download_url"; then
        log_error "Failed to download from $download_url"
        exit 1
    fi
    
    log_info "Extracting archive..."
    tar -xzf "$archive_path" -C "$temp_dir"
    
    log_info "Installing to ${INSTALL_PREFIX}/bin..."
    
    # Create directories if they don't exist
    sudo mkdir -p "${INSTALL_PREFIX}/bin"
    sudo mkdir -p "${INSTALL_PREFIX}/share/man/man1"
    sudo mkdir -p "${INSTALL_PREFIX}/share/bash-completion/completions"
    sudo mkdir -p "${INSTALL_PREFIX}/share/zsh/site-functions"
    sudo mkdir -p "${INSTALL_PREFIX}/share/fish/vendor_completions.d"
    
    # Install binaries
    sudo cp "${temp_dir}/zettel" "${INSTALL_PREFIX}/bin/"
    sudo cp "${temp_dir}/zettel-lsp" "${INSTALL_PREFIX}/bin/"
    sudo chmod +x "${INSTALL_PREFIX}/bin/zettel"
    sudo chmod +x "${INSTALL_PREFIX}/bin/zettel-lsp"
    
    # Install man pages
    if [[ -f "${temp_dir}/man/zettel.1" ]]; then
        sudo cp "${temp_dir}/man/zettel.1" "${INSTALL_PREFIX}/share/man/man1/"
    fi
    
    # Install shell completions
    if [[ -f "${temp_dir}/completions/zettel.bash" ]]; then
        sudo cp "${temp_dir}/completions/zettel.bash" "${INSTALL_PREFIX}/share/bash-completion/completions/zettel"
    fi
    
    if [[ -f "${temp_dir}/completions/zettel.zsh" ]]; then
        sudo cp "${temp_dir}/completions/zettel.zsh" "${INSTALL_PREFIX}/share/zsh/site-functions/_zettel"
    fi
    
    if [[ -f "${temp_dir}/completions/zettel.fish" ]]; then
        sudo cp "${temp_dir}/completions/zettel.fish" "${INSTALL_PREFIX}/share/fish/vendor_completions.d/zettel.fish"
    fi
    
    log_info "Installation complete!"
}

setup_shell_integration() {
    local shell_config
    local bashrc="${HOME}/.bashrc"
    local zshrc="${HOME}/.zshrc"
    
    if [[ -f "$bashrc" ]] && ! grep -q "ZETTEL_VAULT" "$bashrc"; then
        log_info "Adding shell integration to ~/.bashrc"
        cat >> "$bashrc" << 'EOF'

# Zettelkasten CLI
export ZETTEL_VAULT="$HOME/notes"
export ZETTEL_EDITOR="${EDITOR:-vim}"

# Convenient aliases
alias zl="zettel list"
alias zs="zettel search"
alias zn="zettel note create"

# Quick note creation
zq() {
    local title="$*"
    local id=$(zettel id next-sibling $(zettel list --format=json | jq -r 'map(.id) | sort | last'))
    zettel note create "$id" "$title" --open
}
EOF
    fi
    
    if [[ -f "$zshrc" ]] && ! grep -q "ZETTEL_VAULT" "$zshrc"; then
        log_info "Adding shell integration to ~/.zshrc"
        cat >> "$zshrc" << 'EOF'

# Zettelkasten CLI
export ZETTEL_VAULT="$HOME/notes"
export ZETTEL_EDITOR="${EDITOR:-vim}"

# Convenient aliases
alias zl="zettel list"
alias zs="zettel search"
alias zn="zettel note create"

# Quick note creation
zq() {
    local title="$*"
    local id=$(zettel id next-sibling $(zettel list --format=json | jq -r 'map(.id) | sort | last'))
    zettel note create "$id" "$title" --open
}
EOF
    fi
}

main() {
    log_info "Zettelkasten CLI Installation Script"
    
    # Check for help flag
    if [[ "${1:-}" == "--help" ]] || [[ "${1:-}" == "-h" ]]; then
        cat << EOF
Zettelkasten CLI Installation Script

Usage: $0 [VERSION]

Arguments:
    VERSION    Specific version to install (default: latest)

Environment Variables:
    INSTALL_PREFIX    Installation prefix (default: /usr/local)

Examples:
    $0              # Install latest version
    $0 1.2.3        # Install specific version
    INSTALL_PREFIX=~/.local $0  # Install to user directory
EOF
        exit 0
    fi
    
    check_dependencies
    
    local platform
    platform=$(detect_platform)
    
    local version="${1:-}"
    if [[ -z "$version" ]]; then
        log_info "Fetching latest version..."
        version=$(get_latest_version)
    fi
    
    log_info "Installing zettel v${version} for ${platform}..."
    
    download_and_install "$version" "$platform"
    setup_shell_integration
    
    # Verify installation
    if command -v zettel >/dev/null 2>&1; then
        log_info "Verification: $(zettel --version)"
        log_info ""
        log_info "🎉 Installation successful!"
        log_info ""
        log_info "Quick start:"
        log_info "  zettel init ~/notes        # Initialize a vault"
        log_info "  zettel note create 1 'First Note'  # Create your first note"
        log_info "  zettel --help              # See all commands"
        log_info ""
        log_info "Documentation: https://zettel.dev/docs"
    else
        log_error "Installation verification failed"
        exit 1
    fi
}

main "$@"
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
