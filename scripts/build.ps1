# build.ps1 - Windows build script for Zettelkasten CLI
param(
    [string]$Target = "build",
    [switch]$Release,
    [switch]$Verbose,
    [string]$InstallPrefix = "$env:LOCALAPPDATA\Programs\Zettel"
)

$ErrorActionPreference = "Stop"

# Configuration
$CargoCmd = "cargo"
$TargetDir = "target"
$Version = (Get-Content "crates\zettel-cli\Cargo.toml" | Select-String '^version = "([^"]+)"').Matches[0].Groups[1].Value

function Write-InfoMessage {
    param([string]$Message)
    Write-Host "[INFO] $Message" -ForegroundColor Green
}

function Write-WarnMessage {
    param([string]$Message)
    Write-Host "[WARN] $Message" -ForegroundColor Yellow
}

function Write-ErrorMessage {
    param([string]$Message)
    Write-Host "[ERROR] $Message" -ForegroundColor Red
}

function Test-Dependencies {
    $MissingDeps = @()
    
    if (!(Get-Command $CargoCmd -ErrorAction SilentlyContinue)) {
        $MissingDeps += "cargo (Rust toolchain)"
    }
    
    if ($MissingDeps.Count -gt 0) {
        Write-ErrorMessage "Missing required dependencies: $($MissingDeps -join ', ')"
        Write-InfoMessage "Please install Rust from https://rustup.rs/ and try again"
        exit 1
    }
}

function Invoke-Build {
    Write-InfoMessage "Building workspace..."
    
    $BuildArgs = @("build", "--workspace")
    if ($Release) {
        $BuildArgs += "--release"
    }
    
    if ($Verbose) {
        $BuildArgs += "--verbose"
    }
    
    & $CargoCmd @BuildArgs
    if ($LASTEXITCODE -ne 0) {
        Write-ErrorMessage "Build failed"
        exit $LASTEXITCODE
    }
}

function Invoke-Test {
    Write-InfoMessage "Running tests..."
    
    & $CargoCmd test --workspace
    if ($LASTEXITCODE -ne 0) {
        Write-ErrorMessage "Tests failed"
        exit $LASTEXITCODE
    }
}

function Invoke-Check {
    Write-InfoMessage "Running clippy and format checks..."
    
    & $CargoCmd clippy --workspace -- -D warnings
    if ($LASTEXITCODE -ne 0) {
        Write-ErrorMessage "Clippy checks failed"
        exit $LASTEXITCODE
    }
    
    & $CargoCmd fmt --check
    if ($LASTEXITCODE -ne 0) {
        Write-ErrorMessage "Format checks failed"
        exit $LASTEXITCODE
    }
}

function New-Completions {
    Write-InfoMessage "Generating shell completions..."
    
    $CompletionsDir = "scripts\completions"
    if (!(Test-Path $CompletionsDir)) {
        New-Item -ItemType Directory -Path $CompletionsDir | Out-Null
    }
    
    $BinaryPath = if ($Release) { "$TargetDir\release\zettel.exe" } else { "$TargetDir\debug\zettel.exe" }
    
    if (Test-Path $BinaryPath) {
        & $BinaryPath completions powershell > "$CompletionsDir\zettel.ps1"
        & $BinaryPath completions bash > "$CompletionsDir\zettel.bash"
        & $BinaryPath completions zsh > "$CompletionsDir\zettel.zsh"
        & $BinaryPath completions fish > "$CompletionsDir\zettel.fish"
        Write-InfoMessage "Completions generated in $CompletionsDir"
    } else {
        Write-WarnMessage "Binary not found: $BinaryPath. Run build first."
    }
}

function Install-Zettel {
    Write-InfoMessage "Installing to $InstallPrefix..."
    
    # Create installation directories
    $BinDir = Join-Path $InstallPrefix "bin"
    $DocsDir = Join-Path $InstallPrefix "docs"
    $CompletionsDir = Join-Path $InstallPrefix "completions"
    
    New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
    New-Item -ItemType Directory -Path $DocsDir -Force | Out-Null
    New-Item -ItemType Directory -Path $CompletionsDir -Force | Out-Null
    
    # Copy binaries
    $SourceDir = if ($Release) { "$TargetDir\release" } else { "$TargetDir\debug" }
    
    Copy-Item "$SourceDir\zettel.exe" -Destination $BinDir -Force
    Copy-Item "$SourceDir\zettel-lsp.exe" -Destination $BinDir -Force
    
    # Copy completions if they exist
    if (Test-Path "scripts\completions\zettel.ps1") {
        Copy-Item "scripts\completions\*" -Destination $CompletionsDir -Force
    }
    
    # Add to PATH if not already there
    $CurrentPath = [Environment]::GetEnvironmentVariable("PATH", "User")
    if ($CurrentPath -notlike "*$BinDir*") {
        Write-InfoMessage "Adding $BinDir to user PATH..."
        $NewPath = "$CurrentPath;$BinDir"
        [Environment]::SetEnvironmentVariable("PATH", $NewPath, "User")
        Write-InfoMessage "PATH updated. Restart your shell to use 'zettel' command."
    }
    
    # Create PowerShell profile integration
    $ProfilePath = $PROFILE.CurrentUserAllHosts
    if (Test-Path $ProfilePath) {
        $ProfileContent = Get-Content $ProfilePath -Raw
        if ($ProfileContent -notlike "*ZETTEL_VAULT*") {
            Write-InfoMessage "Adding Zettel integration to PowerShell profile..."
            Add-Content $ProfilePath @"

# Zettelkasten CLI
`$env:ZETTEL_VAULT = "`$env:USERPROFILE\Documents\Notes"
`$env:ZETTEL_EDITOR = "notepad"  # Change to your preferred editor

# Load completions
if (Test-Path "$CompletionsDir\zettel.ps1") {
    . "$CompletionsDir\zettel.ps1"
}

# Convenient aliases
Set-Alias zl zettel-list
Set-Alias zs zettel-search
Set-Alias zn zettel-note-create

function zettel-list { zettel list @args }
function zettel-search { zettel search @args }
function zettel-note-create { zettel note create @args }

# Quick note creation
function zq {
    param([string]`$Title)
    `$LastId = zettel list --format=json | ConvertFrom-Json | ForEach-Object { `$_.id } | Sort-Object | Select-Object -Last 1
    `$NextId = zettel id next-sibling `$LastId
    zettel note create `$NextId "`$Title" --open
}
"@
        }
    }
    
    Write-InfoMessage "Installation complete!"
    Write-InfoMessage "Binary location: $BinDir\zettel.exe"
}

function New-WindowsPackage {
    Write-InfoMessage "Creating Windows package..."
    
    $PackageDir = "target\package\zettel-$Version-windows"
    if (Test-Path $PackageDir) {
        Remove-Item $PackageDir -Recurse -Force
    }
    New-Item -ItemType Directory -Path $PackageDir -Force | Out-Null
    
    # Copy binaries
    Copy-Item "$TargetDir\release\zettel.exe" -Destination $PackageDir
    Copy-Item "$TargetDir\release\zettel-lsp.exe" -Destination $PackageDir
    
    # Copy documentation
    $DocsDir = Join-Path $PackageDir "docs"
    New-Item -ItemType Directory -Path $DocsDir | Out-Null
    Copy-Item "README.md" -Destination $DocsDir
    Copy-Item "CHANGELOG.md" -Destination $DocsDir
    Copy-Item "LICENSE*" -Destination $DocsDir
    
    # Copy completions
    if (Test-Path "scripts\completions") {
        $CompletionsDir = Join-Path $PackageDir "completions"
        New-Item -ItemType Directory -Path $CompletionsDir | Out-Null
        Copy-Item "scripts\completions\*" -Destination $CompletionsDir
    }
    
    # Create install script
    $InstallScript = Join-Path $PackageDir "install.ps1"
    @"
# Zettel Windows Installation Script
param([string]`$InstallPath = "`$env:LOCALAPPDATA\Programs\Zettel")

Write-Host "Installing Zettel to `$InstallPath..." -ForegroundColor Green

# Create directories
`$BinDir = Join-Path `$InstallPath "bin"
New-Item -ItemType Directory -Path `$BinDir -Force | Out-Null

# Copy files
Copy-Item "zettel.exe" -Destination `$BinDir -Force
Copy-Item "zettel-lsp.exe" -Destination `$BinDir -Force

# Add to PATH
`$CurrentPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if (`$CurrentPath -notlike "*`$BinDir*") {
    `$NewPath = "`$CurrentPath;`$BinDir"
    [Environment]::SetEnvironmentVariable("PATH", `$NewPath, "User")
    Write-Host "Added to PATH. Restart your shell to use 'zettel' command." -ForegroundColor Yellow
}

Write-Host "Installation complete!" -ForegroundColor Green
"@ | Set-Content $InstallScript
    
    # Create ZIP package
    $ZipPath = "target\package\zettel-$Version-windows-x64.zip"
    if (Test-Path $ZipPath) {
        Remove-Item $ZipPath -Force
    }
    
    Compress-Archive -Path "$PackageDir\*" -DestinationPath $ZipPath
    Write-InfoMessage "Package created: $ZipPath"
}

function Show-Help {
    @"
Zettel Windows Build Script

Usage: .\build.ps1 [Target] [Options]

Targets:
    build         Build the project (default)
    test          Run tests
    check         Run linting and format checks
    install       Install to system
    package       Create Windows package
    completions   Generate shell completions
    clean         Clean build artifacts
    help          Show this help

Options:
    -Release      Build in release mode
    -Verbose      Enable verbose output
    -InstallPrefix <path>  Installation directory (default: $env:LOCALAPPDATA\Programs\Zettel)

Examples:
    .\build.ps1                          # Build in debug mode
    .\build.ps1 build -Release           # Build in release mode
    .\build.ps1 install -Release         # Build and install release version
    .\build.ps1 package -Release         # Create release package

Environment Variables:
    ZETTEL_VAULT      Default vault directory
    ZETTEL_EDITOR     Default editor command
"@
}

# Main execution
switch ($Target.ToLower()) {
    "build" {
        Test-Dependencies
        Invoke-Build
    }
    "test" {
        Test-Dependencies
        Invoke-Test
    }
    "check" {
        Test-Dependencies
        Invoke-Check
    }
    "install" {
        Test-Dependencies
        if (!$Release) {
            Write-WarnMessage "Installing debug build. Use -Release for optimized version."
        }
        Invoke-Build
        New-Completions
        Install-Zettel
    }
    "package" {
        Test-Dependencies
        if (!$Release) {
            Write-ErrorMessage "Package target requires -Release flag"
            exit 1
        }
        Invoke-Build
        New-Completions
        New-WindowsPackage
    }
    "completions" {
        New-Completions
    }
    "clean" {
        Write-InfoMessage "Cleaning build artifacts..."
        & $CargoCmd clean
        if (Test-Path "scripts\completions") {
            Remove-Item "scripts\completions\*" -Force
        }
        if (Test-Path "target\package") {
            Remove-Item "target\package" -Recurse -Force
        }
    }
    "help" {
        Show-Help
    }
    default {
        Write-ErrorMessage "Unknown target: $Target"
        Show-Help
        exit 1
    }
}
