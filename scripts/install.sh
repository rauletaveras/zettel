#!/bin/bash
# scripts/install.sh - Installation script for Zettelkasten CLI

set -euo pipefail

# Configuration
REPO_URL="https://github.com/rauletaveras/zettel"
INSTALL_PREFIX="${INSTALL_PREFIX:-/usr/local}"
PLATFORM="$(uname -s)"
ARCH="$(uname -m)"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
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

show_help() {
    cat << EOF
Zettelkasten CLI Installation Script

Usage: $0 [OPTIONS] [VERSION]

Options:
    -h, --help          Show this help message
    -p, --prefix PATH   Installation prefix (default: /usr/local)
    -u, --user          Install to user directory (~/.local)
    -f, --force         Force installation even if already installed
    -c, --completions   Setup shell completions
    -v, --verbose       Verbose output

Arguments:
    VERSION             Specific version to install (default: latest)

Environment Variables:
    INSTALL_PREFIX      Installation prefix (overridden by --prefix)
    ZETTEL_VAULT       Default vault directory (will be set to ~/notes)

Examples:
    $0                              # Install latest version to /usr/local
    $0 --user                       # Install to ~/.local
    $0 --prefix ~/tools             # Install to custom location
    $0 1.2.3                        # Install specific version
    $0 --completions --user         # User install with shell completions
EOF
}

parse_args() {
    FORCE=false
    SETUP_COMPLETIONS=false
    VERBOSE=false
    VERSION=""
    
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                show_help
                exit 0
                ;;
            -p|--prefix)
                INSTALL_PREFIX="$2"
                shift 2
                ;;
            -u|--user)
                INSTALL_PREFIX="$HOME/.local"
                shift
                ;;
            -f|--force)
                FORCE=true
                shift
                ;;
            -c|--completions)
                SETUP_COMPLETIONS=true
                shift
                ;;
            -v|--verbose)
                VERBOSE=true
                shift
                ;;
            -*)
                log_error "Unknown option: $1"
                show_help
                exit 1
                ;;
            *)
                if [[ -z "$VERSION" ]]; then
                    VERSION="$1"
                else
                    log_error "Too many arguments"
                    show_help
                    exit 1
                fi
                shift
                ;;
        esac
    done
}

check_dependencies() {
    local missing_deps=()
    
    if ! command -v curl >/dev/null 2>&1; then
        missing_deps+=("curl")
    fi
    
    if ! command -v tar >/dev/null 2>&1; then
        missing_deps+=("tar")
    fi
    
    if ! command -v gzip >/dev/null 2>&1; then
        missing_deps+=("gzip")
    fi
    
    if [[ ${#missing_deps[@]} -gt 0 ]]; then
        log_error "Missing required dependencies: ${missing_deps[*]}"
        log_info "Please install them and try again"
        log_info ""
        case "$PLATFORM" in
            "Linux")
                log_info "Ubuntu/Debian: sudo apt install curl tar gzip"
                log_info "RHEL/CentOS:   sudo yum install curl tar gzip"
                log_info "Alpine:        sudo apk add curl tar gzip"
                ;;
            "Darwin")
                log_info "macOS: Dependencies should be available by default"
                log_info "If missing, install Xcode Command Line Tools:"
                log_info "  xcode-select --install"
                ;;
        esac
        exit 1
    fi
}

detect_platform() {
    case "$PLATFORM" in
        "Linux")
            case "$ARCH" in
                "x86_64"|"amd64") echo "linux-x86_64" ;;
                "aarch64"|"arm64") echo "linux-aarch64" ;;
                "armv7l") echo "linux-armv7" ;;
                *) 
                    log_error "Unsupported architecture: $ARCH"
                    log_info "Supported: x86_64, aarch64, armv7l"
                    exit 1 
                    ;;
            esac
            ;;
        "Darwin")
            case "$ARCH" in
                "x86_64") echo "macos-x86_64" ;;
                "arm64") echo "macos-aarch64" ;;
                *) 
                    log_error "Unsupported architecture: $ARCH"
                    log_info "Supported: x86_64, arm64"
                    exit 1 
                    ;;
            esac
            ;;
        *)
            log_error "Unsupported platform: $PLATFORM"
            log_info "Supported platforms: Linux, macOS"
            log_info "For Windows, use: https://github.com/rauletaveras/zettel/releases"
            exit 1
            ;;
    esac
}

get_latest_version() {
    log_info "Fetching latest version from GitHub..."
    local version
    version=$(curl -s "https://api.github.com/repos/rauletaveras/zettel/releases/latest" | \
        grep '"tag_name":' | \
        sed -E 's/.*"([^"]+)".*/\1/' | \
        sed 's/^v//')
    
    if [[ -z "$version" ]]; then
        log_error "Failed to fetch latest version"
        log_info "You can specify a version manually: $0 1.0.0"
        exit 1
    fi
    
    echo "$version"
}

check_existing_installation() {
    local zettel_path="$INSTALL_PREFIX/bin/zettel"
    
    if [[ -f "$zettel_path" ]] && [[ "$FORCE" != true ]]; then
        log_warn "Zettel appears to be already installed at $zettel_path"
        local current_version
        current_version=$("$zettel_path" --version 2>/dev/null | grep -o '[0-9]\+\.[0-9]\+\.[0-9]\+' || echo "unknown")
        log_info "Current version: $current_version"
        log_info "Use --force to reinstall, or choose a different --prefix"
        echo
        read -p "Continue anyway? [y/N] " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            log_info "Installation cancelled"
            exit 0
        fi
    fi
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
    if [[ "$VERBOSE" == true ]]; then
        log_info "URL: $download_url"
    fi
    
    if ! curl -fsSL -o "$archive_path" "$download_url"; then
        log_error "Failed to download from $download_url"
        log_info "Please check:"
        log_info "1. Version $version exists at: ${REPO_URL}/releases"
        log_info "2. Platform $platform is supported"
        log_info "3. Your internet connection"
        exit 1
    fi
    
    log_info "Extracting archive..."
    if ! tar -xzf "$archive_path" -C "$temp_dir"; then
        log_error "Failed to extract archive"
        exit 1
    fi
    
    log_info "Installing to ${INSTALL_PREFIX}..."
    
    # Create directories with appropriate permissions
    local needs_sudo=false
    if [[ ! -w "$INSTALL_PREFIX" ]] && [[ "$INSTALL_PREFIX" != "$HOME"* ]]; then
        needs_sudo=true
    fi
    
    local sudo_cmd=""
    if [[ "$needs_sudo" == true ]]; then
        if command -v sudo >/dev/null 2>&1; then
            sudo_cmd="sudo"
            log_info "Using sudo for system installation..."
        else
            log_error "Need elevated privileges but sudo not available"
            log_info "Try: --user flag for user installation"
            exit 1
        fi
    fi
    
    # Create directory structure
    $sudo_cmd mkdir -p "${INSTALL_PREFIX}/bin"
    $sudo_cmd mkdir -p "${INSTALL_PREFIX}/share/man/man1"
    
    # Install main binary
    if [[ -f "${temp_dir}/zettel" ]]; then
        $sudo_cmd cp "${temp_dir}/zettel" "${INSTALL_PREFIX}/bin/"
        $sudo_cmd chmod +x "${INSTALL_PREFIX}/bin/zettel"
        log_info "✓ Installed: zettel"
    else
        log_error "zettel binary not found in package"
        exit 1
    fi
    
    # Install LSP server if present
    if [[ -f "${temp_dir}/zettel-lsp" ]]; then
        $sudo_cmd cp "${temp_dir}/zettel-lsp" "${INSTALL_PREFIX}/bin/"
        $sudo_cmd chmod +x "${INSTALL_PREFIX}/bin/zettel-lsp"
        log_info "✓ Installed: zettel-lsp"
    fi
    
    # Install man pages if present
    if [[ -f "${temp_dir}/man/zettel.1" ]]; then
        $sudo_cmd cp "${temp_dir}/man/zettel.1" "${INSTALL_PREFIX}/share/man/man1/"
        log_info "✓ Installed: man page"
    fi
    
    # Install completions if requested and present
    if [[ "$SETUP_COMPLETIONS" == true ]]; then
        install_completions "$sudo_cmd" "$temp_dir"
    fi
}

install_completions() {
    local sudo_cmd="$1"
    local temp_dir="$2"
    
    log_info "Setting up shell completions..."
    
    # Create completion directories
    $sudo_cmd mkdir -p "${INSTALL_PREFIX}/share/bash-completion/completions"
    $sudo_cmd mkdir -p "${INSTALL_PREFIX}/share/zsh/site-functions"
    $sudo_cmd mkdir -p "${INSTALL_PREFIX}/share/fish/vendor_completions.d"
    
    # Install bash completion
    if [[ -f "${temp_dir}/completions/zettel.bash" ]]; then
        $sudo_cmd cp "${temp_dir}/completions/zettel.bash" \
            "${INSTALL_PREFIX}/share/bash-completion/completions/zettel"
        log_info "✓ Bash completions"
    fi
    
    # Install zsh completion
    if [[ -f "${temp_dir}/completions/zettel.zsh" ]]; then
        $sudo_cmd cp "${temp_dir}/completions/zettel.zsh" \
            "${INSTALL_PREFIX}/share/zsh/site-functions/_zettel"
        log_info "✓ Zsh completions"
    fi
    
    # Install fish completion
    if [[ -f "${temp_dir}/completions/zettel.fish" ]]; then
        $sudo_cmd cp "${temp_dir}/completions/zettel.fish" \
            "${INSTALL_PREFIX}/share/fish/vendor_completions.d/zettel.fish"
        log_info "✓ Fish completions"
    fi
}

setup_shell_integration() {
    if [[ "$INSTALL_PREFIX" == "$HOME"* ]]; then
        # User installation - safe to modify shell configs
        local bashrc="${HOME}/.bashrc"
        local zshrc="${HOME}/.zshrc"
        local fish_config="${HOME}/.config/fish/config.fish"
        
        # Add to PATH if needed
        local bin_dir="$INSTALL_PREFIX/bin"
        if [[ ":$PATH:" != *":$bin_dir:"* ]]; then
            log_info "Adding $bin_dir to PATH in shell configurations..."
            
            # Bash
            if [[ -f "$bashrc" ]] && ! grep -q "zettel" "$bashrc"; then
                cat >> "$bashrc" << EOF

# Zettelkasten CLI
export PATH="\$PATH:$bin_dir"
export ZETTEL_VAULT="\$HOME/notes"

# Convenient aliases
alias zl="zettel list"
alias zs="zettel search"
alias zn="zettel note create"
alias zo="zettel note open"

# Quick note creation with auto-generated ID
zq() {
    local title="\$*"
    if [[ -z "\$title" ]]; then
        echo "Usage: zq <note title>"
        return 1
    fi
    
    # Get the last used ID and generate next sibling
    local last_id=\$(zettel list --json 2>/dev/null | jq -r 'map(.id) | sort | last' 2>/dev/null || echo "1")
    local next_id=\$(zettel id next-sibling "\$last_id" 2>/dev/null || echo "1")
    
    zettel note create "\$next_id" "\$title" --open
}
EOF
                log_info "✓ Added to ~/.bashrc"
            fi
            
            # Zsh
            if [[ -f "$zshrc" ]] && ! grep -q "zettel" "$zshrc"; then
                cat >> "$zshrc" << EOF

# Zettelkasten CLI
export PATH="\$PATH:$bin_dir"
export ZETTEL_VAULT="\$HOME/notes"

# Convenient aliases
alias zl="zettel list"
alias zs="zettel search"
alias zn="zettel note create"
alias zo="zettel note open"

# Quick note creation with auto-generated ID
zq() {
    local title="\$*"
    if [[ -z "\$title" ]]; then
        echo "Usage: zq <note title>"
        return 1
    fi
    
    # Get the last used ID and generate next sibling
    local last_id=\$(zettel list --json 2>/dev/null | jq -r 'map(.id) | sort | last' 2>/dev/null || echo "1")
    local next_id=\$(zettel id next-sibling "\$last_id" 2>/dev/null || echo "1")
    
    zettel note create "\$next_id" "\$title" --open
}
EOF
                log_info "✓ Added to ~/.zshrc"
            fi
            
            # Fish
            if command -v fish >/dev/null 2>&1; then
                mkdir -p "$(dirname "$fish_config")"
                if [[ -f "$fish_config" ]] && ! grep -q "zettel" "$fish_config"; then
                    cat >> "$fish_config" << EOF

# Zettelkasten CLI
set -gx PATH \$PATH $bin_dir
set -gx ZETTEL_VAULT \$HOME/notes

# Convenient aliases
alias zl="zettel list"
alias zs="zettel search"
alias zn="zettel note create"
alias zo="zettel note open"

# Quick note creation function
function zq
    if test (count \$argv) -eq 0
        echo "Usage: zq <note title>"
        return 1
    end
    
    set title (string join " " \$argv)
    set last_id (zettel list --json 2>/dev/null | jq -r 'map(.id) | sort | last' 2>/dev/null; or echo "1")
    set next_id (zettel id next-sibling \$last_id 2>/dev/null; or echo "1")
    
    zettel note create \$next_id \$title --open
end
EOF
                    log_info "✓ Added to fish config"
                fi
            fi
        fi
    else
        # System installation - just provide instructions
        log_info "For shell integration, add the following to your shell config:"
        echo -e "${BLUE}export ZETTEL_VAULT=\"\$HOME/notes\"${NC}"
        
        if [[ ":$PATH:" != *":$INSTALL_PREFIX/bin:"* ]]; then
            echo -e "${BLUE}export PATH=\"\$PATH:$INSTALL_PREFIX/bin\"${NC}"
        fi
    fi
}

verify_installation() {
    local zettel_path="$INSTALL_PREFIX/bin/zettel"
    
    if command -v zettel >/dev/null 2>&1; then
        local installed_version
        installed_version=$(zettel --version 2>/dev/null | grep -o '[0-9]\+\.[0-9]\+\.[0-9]\+' || echo "unknown")
        log_info "✓ Verification: zettel v$installed_version"
        return 0
    elif [[ -f "$zettel_path" ]]; then
        local installed_version
        installed_version=$("$zettel_path" --version 2>/dev/null | grep -o '[0-9]\+\.[0-9]\+\.[0-9]\+' || echo "unknown")
        log_info "✓ Verification: zettel v$installed_version (at $zettel_path)"
        log_warn "Note: $INSTALL_PREFIX/bin is not in your PATH"
        return 0
    else
        log_error "Installation verification failed"
        return 1
    fi
}

show_success_message() {
    echo
    log_info "🎉 Installation successful!"
    echo
    log_info "Quick start:"
    echo -e "  ${BLUE}zettel init ~/notes${NC}                    # Initialize a vault"
    echo -e "  ${BLUE}zettel note create 1 'My First Note'${NC}   # Create your first note"
    echo -e "  ${BLUE}zettel list${NC}                            # List all notes"
    echo -e "  ${BLUE}zettel search 'keyword'${NC}                # Search notes"
    echo
    log_info "Convenient aliases (if shell integration was set up):"
    echo -e "  ${BLUE}zl${NC}                                     # List notes"
    echo -e "  ${BLUE}zs 'keyword'${NC}                           # Search notes"
    echo -e "  ${BLUE}zn <id> 'title'${NC}                        # Create note"
    echo -e "  ${BLUE}zq 'title'${NC}                             # Quick create with auto-ID"
    echo
    log_info "Documentation:"
    echo -e "  ${BLUE}zettel --help${NC}                          # Built-in help"
    echo -e "  ${BLUE}man zettel${NC}                             # Manual page"
    echo -e "  ${BLUE}https://github.com/rauletaveras/zettel${NC} # Online documentation"
    echo
    
    if [[ "$INSTALL_PREFIX" != "$HOME"* ]] && [[ ":$PATH:" != *":$INSTALL_PREFIX/bin:"* ]]; then
        log_warn "Note: $INSTALL_PREFIX/bin is not in your PATH"
        log_info "To use 'zettel' command globally, add this to your shell config:"
        echo -e "  ${BLUE}export PATH=\"\$PATH:$INSTALL_PREFIX/bin\"${NC}"
        echo
    fi
}

main() {
    # Parse command line arguments
    parse_args "$@"
    
    log_info "🗂️  Zettelkasten CLI Installation Script"
    echo
    
    # Verify system compatibility
    check_dependencies
    
    # Detect platform
    local platform
    platform=$(detect_platform)
    if [[ "$VERBOSE" == true ]]; then
        log_info "Detected platform: $platform"
        log_info "Install prefix: $INSTALL_PREFIX"
    fi
    
    # Get version to install
    if [[ -z "$VERSION" ]]; then
        VERSION=$(get_latest_version)
    fi
    log_info "Target version: v$VERSION"
    
    # Check for existing installation
    check_existing_installation
    
    # Download and install
    download_and_install "$VERSION" "$platform"
    
    # Set up shell integration
    setup_shell_integration
    
    # Verify installation worked
    if verify_installation; then
        show_success_message
    else
        log_error "Installation failed verification"
        exit 1
    fi
}

# Only run main if script is executed directly (not sourced)
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
