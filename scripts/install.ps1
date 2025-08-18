# install.ps1 - Windows installation script for Zettelkasten CLI
[CmdletBinding()]
param(
    [string]$Version = "",
    [string]$InstallPath = "$env:LOCALAPPDATA\Programs\Zettel",
    [switch]$AddToPath,
    [switch]$CreateShortcuts,
    [switch]$SetupProfile,
    [switch]$Force,
    [switch]$User,
    [switch]$System,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

# Configuration
$RepoUrl = "https://github.com/rauletaveras/zettel"
$Platform = "windows-x64"

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

function Write-VerboseMessage {
    param([string]$Message)
    if ($VerbosePreference -eq 'Continue') {
        Write-Host "[DEBUG] $Message" -ForegroundColor Gray
    }
}

function Show-Help {
    @"
Zettelkasten CLI Windows Installer

Usage: .\install.ps1 [Options] [Version]

Options:
    -Version <string>     Specific version to install (default: latest)
    -InstallPath <path>   Installation directory (default: $env:LOCALAPPDATA\Programs\Zettel)
    -User                 Install to user directory (sets InstallPath to $env:LOCALAPPDATA\Programs\Zettel)
    -System               Install to system directory (sets InstallPath to $env:ProgramFiles\Zettel, requires admin)
    -AddToPath            Add installation directory to user PATH
    -CreateShortcuts      Create desktop and start menu shortcuts
    -SetupProfile         Configure PowerShell profile with aliases and completions
    -Force                Force installation even if already installed
    -Verbose              Show detailed output
    -Help                 Show this help message

Arguments:
    Version               Specific version to install (alternative to -Version parameter)

Examples:
    .\install.ps1                                    # Basic installation
    .\install.ps1 -User -AddToPath -SetupProfile     # Full user setup with conveniences
    .\install.ps1 -System                            # System-wide installation (requires admin)
    .\install.ps1 -Version "1.2.3"                  # Install specific version
    .\install.ps1 -InstallPath "C:\Tools\Zettel"    # Custom install location
    .\install.ps1 -Force                             # Force reinstall

Environment Variables (set automatically with -SetupProfile):
    ZETTEL_VAULT      Default vault directory
    ZETTEL_EDITOR     Default editor command

Post-Installation:
    - Restart PowerShell to use new aliases and completions
    - Run 'zettel init' to create your first vault
    - Run 'zettel --help' to see all available commands
"@
}

function Test-Dependencies {
    Write-VerboseMessage "Checking system dependencies..."
    
    $MissingDeps = @()
    
    # Check for PowerShell 5.1+ or PowerShell Core
    if ($PSVersionTable.PSVersion.Major -lt 5) {
        $MissingDeps += "PowerShell 5.1 or later"
    }
    
    # Check for .NET Framework (for Expand-Archive on PS 5.1)
    if ($PSVersionTable.PSVersion.Major -eq 5) {
        try {
            Add-Type -AssemblyName System.IO.Compression.FileSystem -ErrorAction SilentlyContinue
        }
        catch {
            $MissingDeps += ".NET Framework 4.5 or later"
        }
    }
    
    if ($MissingDeps.Count -gt 0) {
        Write-ErrorMessage "Missing required dependencies: $($MissingDeps -join ', ')"
        Write-InfoMessage "Please install the required dependencies and try again."
        Write-InfoMessage ""
        Write-InfoMessage "PowerShell: https://github.com/PowerShell/PowerShell/releases"
        Write-InfoMessage ".NET Framework: https://dotnet.microsoft.com/download/dotnet-framework"
        exit 1
    }
    
    Write-VerboseMessage "All dependencies satisfied"
}

function Resolve-InstallationPath {
    # Handle special installation modes
    if ($User) {
        $script:InstallPath = "$env:LOCALAPPDATA\Programs\Zettel"
        Write-VerboseMessage "User installation: $script:InstallPath"
    }
    elseif ($System) {
        $script:InstallPath = "$env:ProgramFiles\Zettel"
        Write-VerboseMessage "System installation: $script:InstallPath"
        
        # Check for admin privileges
        if (-not ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole] "Administrator")) {
            Write-ErrorMessage "System installation requires administrator privileges"
            Write-InfoMessage "Please run PowerShell as Administrator or use -User flag"
            exit 1
        }
    }
    
    Write-VerboseMessage "Final install path: $script:InstallPath"
}

function Test-ExistingInstallation {
    $ExistingBinary = Join-Path $InstallPath "bin\zettel.exe"
    
    if ((Test-Path $ExistingBinary) -and -not $Force) {
        Write-WarnMessage "Zettel appears to be already installed at $InstallPath"
        
        try {
            $CurrentVersion = & $ExistingBinary --version 2>$null | Select-String '\d+\.\d+\.\d+' | ForEach-Object { $_.Matches[0].Value }
            Write-InfoMessage "Current version: $CurrentVersion"
        }
        catch {
            Write-InfoMessage "Current version: unknown"
        }
        
        Write-InfoMessage "Use -Force to reinstall, or choose a different -InstallPath"
        
        $Response = Read-Host "Continue anyway? (y/N)"
        if ($Response -notmatch '^[Yy]') {
            Write-InfoMessage "Installation cancelled"
            exit 0
        }
    }
}

function Get-LatestVersion {
    try {
        Write-InfoMessage "Fetching latest version from GitHub..."
        Write-VerboseMessage "API URL: https://api.github.com/repos/rauletaveras/zettel/releases/latest"
        
        $Response = Invoke-RestMethod -Uri "https://api.github.com/repos/rauletaveras/zettel/releases/latest" -TimeoutSec 30
        $LatestVersion = $Response.tag_name -replace '^v', ''
        
        Write-VerboseMessage "Latest version: $LatestVersion"
        return $LatestVersion
    }
    catch {
        Write-ErrorMessage "Failed to fetch latest version: $($_.Exception.Message)"
        Write-InfoMessage "You can specify a version manually with -Version parameter"
        Write-InfoMessage "Check releases at: $RepoUrl/releases"
        exit 1
    }
}

function Get-ZettelRelease {
    param([string]$Version)
    
    $TempDir = Join-Path $env:TEMP "zettel-install-$(Get-Random)"
    New-Item -ItemType Directory -Path $TempDir -Force | Out-Null
    Write-VerboseMessage "Created temp directory: $TempDir"
    
    try {
        $DownloadUrl = "$RepoUrl/releases/download/v$Version/zettel-$Version-$Platform.zip"
        $ZipPath = Join-Path $TempDir "zettel.zip"
        
        Write-InfoMessage "Downloading zettel v$Version for $Platform..."
        Write-VerboseMessage "Download URL: $DownloadUrl"
        Write-VerboseMessage "Zip path: $ZipPath"
        
        # Use different download methods based on PowerShell version
        if ($PSVersionTable.PSVersion.Major -ge 6) {
            # PowerShell Core - better progress and error handling
            try {
                $ProgressPreference = 'Continue'
                Invoke-WebRequest -Uri $DownloadUrl -OutFile $ZipPath -UseBasicParsing -TimeoutSec 300
            }
            finally {
                $ProgressPreference = 'Continue'
            }
        }
        else {
            # Windows PowerShell 5.1 - use WebClient for better compatibility
            $WebClient = New-Object System.Net.WebClient
            try {
                $WebClient.DownloadFile($DownloadUrl, $ZipPath)
            }
            finally {
                $WebClient.Dispose()
            }
        }
        
        if (-not (Test-Path $ZipPath)) {
            throw "Download failed - file not found at $ZipPath"
        }
        
        $ZipSize = (Get-Item $ZipPath).Length
        Write-VerboseMessage "Downloaded $ZipSize bytes"
        
        if ($ZipSize -lt 1KB) {
            throw "Downloaded file is too small - likely an error page"
        }
        
        Write-InfoMessage "Extracting archive..."
        try {
            Expand-Archive -Path $ZipPath -DestinationPath $TempDir -Force
        }
        catch {
            throw "Failed to extract archive: $($_.Exception.Message)"
        }
        
        Write-VerboseMessage "Extraction completed to: $TempDir"
        return $TempDir
    }
    catch {
        Write-ErrorMessage "Failed to download or extract: $($_.Exception.Message)"
        Write-InfoMessage "Please check:"
        Write-InfoMessage "1. Version $Version exists at: $RepoUrl/releases"
        Write-InfoMessage "2. Your internet connection"
        Write-InfoMessage "3. Windows Defender or antivirus isn't blocking the download"
        
        if (Test-Path $TempDir) {
            Remove-Item $TempDir -Recurse -Force -ErrorAction SilentlyContinue
        }
        exit 1
    }
}

function Install-ZettelBinaries {
    param(
        [string]$SourceDir,
        [string]$TargetDir
    )
    
    Write-InfoMessage "Installing to $TargetDir..."
    
    # Create installation directories
    $BinDir = Join-Path $TargetDir "bin"
    $DocsDir = Join-Path $TargetDir "docs"
    $CompletionsDir = Join-Path $TargetDir "completions"
    $TemplatesDir = Join-Path $TargetDir "templates"
    
    Write-VerboseMessage "Creating directory structure..."
    New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
    New-Item -ItemType Directory -Path $DocsDir -Force | Out-Null
    New-Item -ItemType Directory -Path $CompletionsDir -Force | Out-Null
    New-Item -ItemType Directory -Path $TemplatesDir -Force | Out-Null
    
    # Find and copy binaries
    $ZettelExe = Get-ChildItem -Path $SourceDir -Name "zettel.exe" -Recurse -File | Select-Object -First 1
    $ZettelLspExe = Get-ChildItem -Path $SourceDir -Name "zettel-lsp.exe" -Recurse -File | Select-Object -First 1
    
    if ($ZettelExe) {
        $SourcePath = Join-Path $SourceDir $ZettelExe
        Copy-Item $SourcePath -Destination $BinDir -Force
        Write-InfoMessage "Installed: zettel.exe"
        Write-VerboseMessage "Binary path: $BinDir\zettel.exe"
    }
    else {
        Write-ErrorMessage "zettel.exe not found in downloaded package"
        Write-VerboseMessage "Package contents:"
        Get-ChildItem -Path $SourceDir -Recurse | ForEach-Object { Write-VerboseMessage "  $($_.FullName)" }
        exit 1
    }
    
    if ($ZettelLspExe) {
        $SourcePath = Join-Path $SourceDir $ZettelLspExe
        Copy-Item $SourcePath -Destination $BinDir -Force
        Write-InfoMessage "Installed: zettel-lsp.exe"
    }
    else {
        Write-VerboseMessage "zettel-lsp.exe not found (optional)"
    }
    
    # Copy documentation
    $ReadmePath = Get-ChildItem -Path $SourceDir -Name "README.md" -Recurse -File | Select-Object -First 1
    if ($ReadmePath) {
        Copy-Item (Join-Path $SourceDir $ReadmePath) -Destination $DocsDir -Force
        Write-VerboseMessage "Copied documentation"
    }
    
    # Copy license
    $LicensePath = Get-ChildItem -Path $SourceDir -Name "LICENSE*" -Recurse -File | Select-Object -First 1
    if ($LicensePath) {
        Copy-Item (Join-Path $SourceDir $LicensePath) -Destination $DocsDir -Force
        Write-VerboseMessage "Copied license"
    }
    
    # Copy completions
    $CompletionsSource = Get-ChildItem -Path $SourceDir -Name "completions" -Recurse -Directory | Select-Object -First 1
    if ($CompletionsSource) {
        $CompletionsSourcePath = Join-Path $SourceDir $CompletionsSource.Name
        Get-ChildItem -Path $CompletionsSourcePath -File | ForEach-Object {
            Copy-Item $_.FullName -Destination $CompletionsDir -Force
        }
        Write-VerboseMessage "Copied shell completions"
    }
    
    return $BinDir
}

function Add-ToSystemPath {
    param([string]$Path)
    
    if (-not $AddToPath) {
        Write-InfoMessage "Skipping PATH modification. Use -AddToPath to add automatically."
        Write-InfoMessage "To add manually, run these commands:"
        Write-Host "  `$env:PATH += ';$Path'" -ForegroundColor Cyan
        Write-Host "  [Environment]::SetEnvironmentVariable('PATH', `$env:PATH, 'User')" -ForegroundColor Cyan
        return
    }
    
    Write-VerboseMessage "Adding $Path to system PATH..."
    
    # Determine scope based on installation type
    $Scope = if ($System) { "Machine" } else { "User" }
    
    # Get current PATH
    $CurrentPath = [Environment]::GetEnvironmentVariable("PATH", $Scope)
    
    # Check if already in PATH
    $PathEntries = $CurrentPath -split ';' | Where-Object { $_ -ne '' }
    if ($PathEntries -contains $Path) {
        Write-InfoMessage "Directory already in PATH: $Path"
        return
    }
    
    # Add to PATH
    Write-InfoMessage "Adding to $Scope PATH: $Path"
    $NewPath = if ($CurrentPath) { "$CurrentPath;$Path" } else { $Path }
    
    try {
        [Environment]::SetEnvironmentVariable("PATH", $NewPath, $Scope)
        
        # Update current session PATH
        $env:PATH += ";$Path"
        
        Write-InfoMessage "PATH updated successfully"
        Write-InfoMessage "Note: You may need to restart PowerShell or other applications to see the change"
    }
    catch {
        Write-ErrorMessage "Failed to update PATH: $($_.Exception.Message)"
        if ($Scope -eq "Machine") {
            Write-InfoMessage "Try running as Administrator or use -User flag"
        }
    }
}

function New-DesktopShortcuts {
    param([string]$BinDir)
    
    if (-not $CreateShortcuts) {
        return
    }
    
    Write-InfoMessage "Creating desktop shortcuts..."
    
    try {
        $WScriptShell = New-Object -ComObject WScript.Shell
        
        # Desktop shortcut
        $DesktopPath = [Environment]::GetFolderPath("Desktop")
        $ShortcutPath = Join-Path $DesktopPath "Zettel CLI.lnk"
        $Shortcut = $WScriptShell.CreateShortcut($ShortcutPath)
        $Shortcut.TargetPath = "powershell.exe"
        $Shortcut.Arguments = '-NoExit -Command "Write-Host ''Zettel CLI Ready'' -ForegroundColor Green"'
        $Shortcut.WorkingDirectory = "$env:USERPROFILE\Documents"
        $Shortcut.Description = "Zettelkasten CLI - Knowledge Management Tool"
        $Shortcut.Save()
        Write-InfoMessage "Created desktop shortcut: $ShortcutPath"
        
        # Start menu shortcut
        $StartMenuPath = [Environment]::GetFolderPath("StartMenu")
        $ProgramsPath = Join-Path $StartMenuPath "Programs"
        $ZettelFolder = Join-Path $ProgramsPath "Zettel"
        
        if (-not (Test-Path $ZettelFolder)) {
            New-Item -ItemType Directory -Path $ZettelFolder -Force | Out-Null
        }
        
        $StartShortcutPath = Join-Path $ZettelFolder "Zettel CLI.lnk"
        $StartShortcut = $WScriptShell.CreateShortcut($StartShortcutPath)
        $StartShortcut.TargetPath = "powershell.exe"
        $StartShortcut.Arguments = '-NoExit -Command "Write-Host ''Zettel CLI Ready'' -ForegroundColor Green"'
        $StartShortcut.WorkingDirectory = "$env:USERPROFILE\Documents"
        $StartShortcut.Description = "Zettelkasten CLI - Knowledge Management Tool"
        $StartShortcut.Save()
        Write-InfoMessage "Created start menu shortcut: $StartShortcutPath"
    }
    catch {
        Write-WarnMessage "Failed to create shortcuts: $($_.Exception.Message)"
    }
}

function Set-PowerShellProfile {
    param([string]$CompletionsDir, [string]$BinDir)
    
    if (-not $SetupProfile) {
        return
    }
    
    Write-InfoMessage "Configuring PowerShell profile..."
    
    $ProfilePath = $PROFILE.CurrentUserAllHosts
    $ProfileDir = Split-Path $ProfilePath -Parent
    
    # Create profile directory if it doesn't exist
    if (-not (Test-Path $ProfileDir)) {
        New-Item -ItemType Directory -Path $ProfileDir -Force | Out-Null
        Write-VerboseMessage "Created profile directory: $ProfileDir"
    }
    
    # Check if profile already has zettel configuration
    $ProfileContent = if (Test-Path $ProfilePath) { Get-Content $ProfilePath -Raw } else { "" }
    
    if ($ProfileContent -like "*ZETTEL_VAULT*") {
        Write-InfoMessage "PowerShell profile already configured for Zettel"
        return
    }
    
    Write-VerboseMessage "Adding Zettel configuration to: $ProfilePath"
    
    $ZettelConfig = @"

# ==========================================
# Zettelkasten CLI Configuration
# ==========================================

# Environment variables
`$env:ZETTEL_VAULT = "`$env:USERPROFILE\Documents\Notes"
`$env:ZETTEL_EDITOR = "notepad"

# Add zettel to PATH if not already there
if (`$env:PATH -notlike "*$BinDir*") {
    `$env:PATH += ";$BinDir"
}

# Convenient aliases
function zl { zettel list @args }
function zs { zettel search @args }
function zn { zettel note create @args }
function zo { zettel note open @args }

# Quick note creation with auto-generated ID
function zq {
    param([string]`$Title)
    if (-not `$Title) {
        `$Title = Read-Host "Enter note title"
    }
    
    try {
        `$LastId = zettel list --json 2>`$null | ConvertFrom-Json | ForEach-Object { `$_.id } | Sort-Object | Select-Object -Last 1
        if (-not `$LastId) { `$LastId = "1" }
        
        `$NextId = zettel id next-sibling `$LastId 2>`$null
        if (-not `$NextId) { `$NextId = "1" }
        
        zettel note create `$NextId `$Title --open
        Write-Host "Created note: `$NextId - `$Title" -ForegroundColor Green
    }
    catch {
        Write-Warning "Could not create note: `$_"
        Write-Host "Try running 'zettel init' first" -ForegroundColor Yellow
    }
}

# Welcome message
if (Get-Command zettel -ErrorAction SilentlyContinue) {
    Write-Host "Zettelkasten CLI loaded! Commands: zl (list), zs (search), zq (quick note)" -ForegroundColor Green
}

# ==========================================
"@
    
    try {
        Add-Content -Path $ProfilePath -Value $ZettelConfig -Encoding UTF8
        Write-InfoMessage "PowerShell profile configured: $ProfilePath"
        Write-InfoMessage "Restart PowerShell to load new functions"
    }
    catch {
        Write-ErrorMessage "Failed to configure PowerShell profile: $($_.Exception.Message)"
    }
}

function Test-Installation {
    param([string]$BinDir)
    
    Write-VerboseMessage "Verifying installation..."
    
    $ZettelPath = Join-Path $BinDir "zettel.exe"
    
    if (-not (Test-Path $ZettelPath)) {
        Write-ErrorMessage "Installation verification failed: zettel.exe not found at $ZettelPath"
        return $false
    }
    
    try {
        # Test if zettel runs and reports version
        $VersionOutput = & $ZettelPath --version 2>&1
        if ($LASTEXITCODE -eq 0) {
            $Version = $VersionOutput | Select-String '\d+\.\d+\.\d+' | ForEach-Object { $_.Matches[0].Value }
            Write-InfoMessage "Installation verified: zettel v$Version"
            return $true
        }
        else {
            Write-ErrorMessage "zettel.exe found but failed to run properly"
            return $false
        }
    }
    catch {
        Write-ErrorMessage "Failed to test installation: $($_.Exception.Message)"
        return $false
    }
}

function Show-PostInstallInstructions {
    param([string]$InstallPath, [string]$BinDir)
    
    Write-Host ""
    Write-Host "Installation complete!" -ForegroundColor Green
    Write-Host ""
    Write-Host "Installed to: $InstallPath" -ForegroundColor Cyan
    Write-Host "Binaries: $BinDir" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Quick start:" -ForegroundColor Yellow
    Write-Host "  zettel init `$env:USERPROFILE\Documents\Notes  # Initialize vault" -ForegroundColor White
    Write-Host "  zettel note create 1 'My First Note'           # Create first note" -ForegroundColor White
    Write-Host "  zettel list                                    # List all notes" -ForegroundColor White
    Write-Host ""
    
    if ($SetupProfile) {
        Write-Host "Convenient aliases available (restart PowerShell):" -ForegroundColor Yellow
        Write-Host "  zl              # List notes" -ForegroundColor White
        Write-Host "  zs <query>      # Search notes" -ForegroundColor White
        Write-Host "  zn <id> <title> # Create note" -ForegroundColor White
        Write-Host "  zq <title>      # Quick create with auto-ID" -ForegroundColor White
        Write-Host ""
    }
    
    Write-Host "Documentation: $RepoUrl" -ForegroundColor Blue
    Write-Host "Help: zettel --help" -ForegroundColor Blue
    Write-Host ""
}

# Main execution starts here
try {
    if ($Help) {
        Show-Help
        exit 0
    }
    
    # Handle positional version argument
    if ($args -and -not $Version) {
        $Version = $args[0]
    }
    
    Write-Host "Zettelkasten CLI Windows Installer" -ForegroundColor Cyan
    Write-Host ""
    
    # Resolve installation path based on flags
    Resolve-InstallationPath
    
    # Verify system requirements
    Test-Dependencies
    
    # Check for existing installation
    Test-ExistingInstallation
    
    # Get version to install
    if (-not $Version) {
        $Version = Get-LatestVersion
    }
    
    Write-InfoMessage "Installing zettel v$Version to $InstallPath..."
    
    # Download and extract
    $TempDir = Get-ZettelRelease -Version $Version
    
    try {
        # Install binaries and files
        $BinDir = Install-ZettelBinaries -SourceDir $TempDir -TargetDir $InstallPath
        
        # Configure system integration
        Add-ToSystemPath -Path $BinDir
        
        if ($CreateShortcuts) {
            New-DesktopShortcuts -BinDir $BinDir
        }
        
        if ($SetupProfile) {
            $CompletionsDir = Join-Path $InstallPath "completions"
            Set-PowerShellProfile -CompletionsDir $CompletionsDir -BinDir $BinDir
        }
        
        # Verify installation
        if (Test-Installation -BinDir $BinDir) {
            Show-PostInstallInstructions -InstallPath $InstallPath -BinDir $BinDir
        }
        else {
            Write-ErrorMessage "Installation verification failed"
            exit 1
        }
    }
    finally {
        # Cleanup temporary directory
        if (Test-Path $TempDir) {
            Remove-Item $TempDir -Recurse -Force -ErrorAction SilentlyContinue
            Write-VerboseMessage "Cleaned up temporary directory: $TempDir"
        }
    }
}
catch {
    Write-ErrorMessage "Installation failed: $($_.Exception.Message)"
    if ($Verbose) {
        Write-Host "Full exception details:" -ForegroundColor Red
        Write-Host $_.Exception.ToString() -ForegroundColor Red
    }
    exit 1
}
