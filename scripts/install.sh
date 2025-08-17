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
