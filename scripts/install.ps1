# install.ps1 - Windows installation script for Zettelkasten CLI
[CmdletBinding()]
param(
    [string]$Version = "",
    [string]$InstallPath = "$env:LOCALAPPDATA\Programs\Zettel",
    [switch]$AddToPath,
    [switch]$CreateShortcuts,
    [switch]$SetupProfile,
    [switch]$Force
)

$ErrorActionPreference = "Stop"

# Configuration
$RepoUrl = "https://github.com/username/zettel"
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

function Test-Dependencies {
    $MissingDeps = @()
    
    # Check for PowerShell 5.1+ or PowerShell Core
    if ($PSVersionTable.PSVersion.Major -lt 5) {
        $MissingDeps += "PowerShell 5.1 or later"
    }
    
    # Check for .NET Framework (for Expand-Archive on PS 5.1)
    if ($PSVersionTable.PSVersion.Major -eq 5 -and !(Get-Command Expand-Archive -ErrorAction SilentlyContinue)) {
        $MissingDeps += ".NET Framework 4.5 or later"
    }
    
    if ($MissingDeps.Count -gt 0) {
        Write-ErrorMessage "Missing required dependencies: $($MissingDeps -join ', ')"
        exit 1
    }
}

function Get-LatestVersion {
    try {
        Write-InfoMessage "Fetching latest version..."
        $Response = Invoke-RestMethod -Uri "https://api.github.com/repos/username/zettel/releases/latest"
        return $Response.tag_name -replace '^v', ''
    }
    catch {
        Write-ErrorMessage "Failed to fetch latest version: $($_.Exception.Message)"
        Write-InfoMessage "You can specify a version manually with -Version parameter"
        exit 1
    }
}

function Get-ZettelRelease {
    param([string]$Version)
    
    $TempDir = Join-Path $env:TEMP "zettel-install-$(Get-Random)"
    New-Item -ItemType Directory -Path $TempDir | Out-Null
    
    try {
        $DownloadUrl = "$RepoUrl/releases/download/v$Version/zettel-$Version-$Platform.zip"
        $ZipPath = Join-Path $TempDir "zettel.zip"
        
        Write-InfoMessage "Downloading zettel v$Version for $Platform..."
        Write-InfoMessage "URL: $DownloadUrl"
        
        # Use different download methods based on PowerShell version
        if ($PSVersionTable.PSVersion.Major -ge 6) {
            # PowerShell Core
            Invoke-WebRequest -Uri $DownloadUrl -OutFile $ZipPath -UseBasicParsing
        } else {
            # Windows PowerShell 5.1
            $WebClient = New-Object System.Net.WebClient
            $WebClient.DownloadFile($DownloadUrl, $ZipPath)
            $WebClient.Dispose()
        }
        
        if (!(Test-Path $ZipPath)) {
            throw "Download failed - file not found"
        }
        
        Write-InfoMessage "Extracting archive..."
        Expand-Archive -Path $ZipPath -DestinationPath $TempDir -Force
        
        return $TempDir
    }
    catch {
        Write-ErrorMessage "Failed to download or extract: $($_.Exception.Message)"
        Remove-Item $TempDir -Recurse -Force -ErrorAction SilentlyContinue
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
    
    New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
    New-Item -ItemType Directory -Path $DocsDir -Force | Out-Null
    New-Item -ItemType Directory -Path $CompletionsDir -Force | Out-Null
    
    # Find and copy binaries
    $ZettelExe = Get-ChildItem -Path $SourceDir -Name "zettel.exe" -Recurse | Select-Object -First 1
    $ZettelLspExe = Get-ChildItem -Path $SourceDir -Name "zettel-lsp.exe" -Recurse | Select-Object -First 1
    
    if ($ZettelExe) {
        $SourcePath = Join-Path $SourceDir $ZettelExe
        Copy-Item $SourcePath -Destination $BinDir -Force
        Write-InfoMessage "Installed: zettel.exe"
    } else {
        Write-ErrorMessage "zettel.exe not found in downloaded package"
        exit 1
    }
    
    if ($ZettelLspExe) {
        $SourcePath = Join-Path $SourceDir $ZettelLspExe
        Copy-Item $SourcePath -Destination $BinDir -Force
        Write-InfoMessage "Installed: zettel-lsp.exe"
    }
    
    # Copy documentation
    $ReadmePath = Get-ChildItem -Path $SourceDir -Name "README.md" -Recurse | Select-Object -First 1
    if ($ReadmePath) {
        Copy-Item (Join-Path $SourceDir $ReadmePath) -Destination $DocsDir -Force
    }
    
    # Copy completions
    $CompletionsSource = Get-ChildItem -Path $SourceDir -Name "completions" -Recurse -Directory | Select-Object -First 1
    if ($CompletionsSource) {
        Copy-Item (Join-Path $SourceDir $CompletionsSource.Name "\*") -Destination $CompletionsDir -Force -Recurse
    }
    
    return $BinDir
}

function Add-ToSystemPath {
    param([string]$Path)
    
    if (!$AddToPath) {
        Write-InfoMessage "Skipping PATH modification. Use -AddToPath to add automatically."
        Write-InfoMessage "To add manually, run: `$env:PATH += ';$Path'"
        return
    }
    
    # Get current user PATH
    $CurrentPath = [Environment]::GetEnvironmentVariable("PATH", "User")
    
    # Check if already in PATH
    if ($CurrentPath -split ';' -contains $Path) {
        Write-InfoMessage "Directory already in PATH: $Path"
        return
    }
    
    # Add to PATH
    Write-InfoMessage "Adding to user PATH: $Path"
    $NewPath = if ($CurrentPath) { "$CurrentPath;$Path" } else { $Path }
    [Environment]::SetEnvironmentVariable("PATH", $NewPath, "User")
    
    # Update current session PATH
    $env:PATH += ";$Path"
    
    Write-InfoMessage "PATH updated. You may need to restart your shell."
}

function New-DesktopShortcuts {
    param([string]$BinDir)
    
    if (!$CreateShortcuts) {
        return
    }
    
    $WScriptShell = New-Object -ComObject WScript.Shell
    $DesktopPath = [Environment]::GetFolderPath("Desktop")
    
    # Create shortcut for zettel CLI
    $ShortcutPath = Join-Path $DesktopPath "Zettel CLI.lnk"
    $Shortcut = $WScriptShell.CreateShortcut($ShortcutPath)
    $Shortcut.TargetPath = "powershell.exe"
    $Shortcut.Arguments = "-NoExit -Command `"Write-Host 'Zettel CLI Ready' -ForegroundColor Green; zettel --help`""
    $Shortcut.WorkingDirectory = "$env:USERPROFILE\Documents"
    $Shortcut.Description = "Zettelkasten CLI"
    $Shortcut.Save()
    
    Write-InfoMessage "Created desktop shortcut: $ShortcutPath"
}

function Set-PowerShellProfile {
    param([string]$CompletionsDir)
    
    if (!$SetupProfile) {
        return
    }
    
    $ProfilePath = $PROFILE.CurrentUserAllHosts
    $ProfileDir = Split-Path $ProfilePath -Parent
    
    # Create profile directory if it doesn't exist
    if (!(Test-Path $ProfileDir)) {
        New-Item -ItemType Directory -Path $ProfileDir -Force | Out-Null
    }
    
    # Check if profile already has zettel configuration
    $ProfileContent = if (Test-Path $ProfilePath) { Get-Content $ProfilePath -Raw } else { "" }
    
    if ($ProfileContent -like "*ZETTEL_VAULT*") {
        Write-InfoMessage "PowerShell profile already configured for Zettel"
        return
    }
    
    Write-InfoMessage "Configuring PowerShell profile..."
    
    $ZettelConfig = @"

# ==========================================
# Zettelkasten CLI Configuration
# ==========================================

# Environment variables
`$env:ZETTEL_VAULT = "`$env:USERPROFILE\Documents\Notes"
`$env:ZETTEL_EDITOR = "notepad"  # Change to your preferred editor (code, vim, helix, etc.)

# Load tab completions
`$ZettelCompletions = "$($CompletionsDir -replace '\\', '\\')\zettel.ps1"
if (Test-Path `$ZettelCompletions) {
    . `$ZettelCompletions
}

# Convenient aliases
function zl { zettel list @args }
function zs { zettel search @args }
function zn { zettel note create @args }
function zo { zettel note open @args }
function zt { zettel tree @args }

# Quick note creation function
function zq {
    param([Parameter(ValueFromRemainingArguments)][string[]]`$Title)
    `$TitleString = `$Title -join " "
    if (!`$TitleString) {
        `$TitleString = Read-Host "Enter note title"
    }
    
    try {
        `$LastId = zettel list --format=json | ConvertFrom-Json | ForEach-Object { `$_.id } | Sort-Object | Select-Object -Last 1
        `$NextId = zettel id next-sibling `$LastId
        zettel note create `$NextId "`$TitleString" --open
    }
    catch {
        Write-Warning "Could not create note: `$_"
        Write-Host "Try: zettel init first to create a vault"
    }
}

# Quick search and open
function zf {
    param([string]`$Query)
    try {
        `$Results = zettel search "`$Query" --format=json | ConvertFrom-Json
        if (`$Results.Count -eq 0) {
            Write-Host "No results found for: `$Query" -ForegroundColor Yellow
            return
        }
        elseif (`$Results.Count -eq 1) {
            zettel note open `$Results[0].id
        }
        else {
            Write-Host "Multiple results found:" -ForegroundColor Cyan
            for (`$i = 0; `$i -lt `$Results.Count; `$i++) {
                Write-Host "  [`$i] `$(`$Results[`$i].title) (`$(`$Results[`$i].id))"
            }
            `$Choice = Read-Host "Enter number to open"
            if (`$Choice -match '^\d+$' -and [int]`$Choice -lt `$Results.Count) {
                zettel note open `$Results[[int]`$Choice].id
            }
        }
    }
    catch {
        Write-Warning "Search failed: `$_"
    }
}

# Welcome message (remove this section after you're familiar with the commands)
if (Get-Command zettel -ErrorAction SilentlyContinue) {
    Write-Host "🗂️  Zettelkasten CLI loaded!" -ForegroundColor Green
    Write-Host "Quick commands: zl (list), zs (search), zn (new note), zq (quick note), zf (find & open)" -ForegroundColor Cyan
    Write-Host "Full help: zettel --help" -ForegroundColor Gray
}

# ==========================================
"@
    
    Add-Content -Path $ProfilePath -Value $ZettelConfig -Encoding UTF8
    Write-InfoMessage "PowerShell profile configured: $ProfilePath"
    Write-InfoMessage "Restart PowerShell or run: . `$PROFILE"
}

function Test-Installation {
    param([string]$BinDir)
    
    $ZettelPath = Join-Path $BinDir "zettel.exe"
    
    if (!(Test-Path $ZettelPath)) {
        Write-ErrorMessage "Installation verification failed: zettel.exe not found"
        return $false
    }
    
    try {
        # Test if zettel runs
        $VersionOutput = & $ZettelPath --version 2>&1
        if ($LASTEXITCODE -eq 0) {
            Write-InfoMessage "Installation verified: $VersionOutput"
            return $true
        } else {
            Write-ErrorMessage "zettel.exe found but failed to run: $VersionOutput"
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
    Write-Host "🎉 Installation complete!" -ForegroundColor Green
    Write-Host ""
    Write-Host "📍 Installed to: $InstallPath" -ForegroundColor Cyan
    Write-Host "📍 Binaries: $BinDir" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "🚀 Quick start:" -ForegroundColor Yellow
    Write-Host "  zettel init `$env:USERPROFILE\Documents\Notes  # Initialize vault"
    Write-Host "  zettel note create 1 'My First Note'           # Create first note"
    Write-Host "  zettel list                                    # List all notes"
    Write-Host "  zettel search 'keyword'                       # Search notes"
    Write-Host ""
    
    if ($SetupProfile) {
        Write-Host "🔧 Convenient aliases available:" -ForegroundColor Yellow
        Write-Host "  zl            # List notes"
        Write-Host "  zs <query>    # Search notes"
        Write-Host "  zn <id> <title>  # Create note"
        Write-Host "  zq <title>    # Quick create with auto-ID"
        Write-Host "  zf <query>    # Find and open note"
        Write-Host ""
    }
    
    Write-Host "📚 Documentation: https://zettel.dev/docs" -ForegroundColor Blue
    Write-Host "❓ Help: zettel --help" -ForegroundColor Blue
    Write-Host ""
    
    if (!$AddToPath) {
        Write-Host "⚠️  To use 'zettel' command globally, add to PATH:" -ForegroundColor Yellow
        Write-Host "   `$env:PATH += ';$BinDir'" -ForegroundColor Gray
        Write-Host "   Or re-run with -AddToPath flag" -ForegroundColor Gray
        Write-Host ""
    }
}

function Show-Help {
    @"
Zettelkasten CLI Windows Installer

Usage: .\install.ps1 [Options]

Options:
    -Version <string>     Specific version to install (default: latest)
    -InstallPath <path>   Installation directory (default: $env:LOCALAPPDATA\Programs\Zettel)
    -AddToPath           Add installation directory to user PATH
    -CreateShortcuts     Create desktop shortcuts
    -SetupProfile        Configure PowerShell profile with aliases and completions
    -Force              Force installation even if already installed

Examples:
    .\install.ps1                                    # Basic installation
    .\install.ps1 -AddToPath -SetupProfile           # Full setup with conveniences
    .\install.ps1 -Version "1.2.3"                  # Install specific version
    .\install.ps1 -InstallPath "C:\Tools\Zettel"    # Custom install location
    .\install.ps1 -Force                             # Force reinstall

Environment Variables (set automatically with -SetupProfile):
    ZETTEL_VAULT      Default vault directory
    ZETTEL_EDITOR     Default editor command
"@
}

# Main execution
if ($PSCmdlet.ParameterSetName -eq "Help" -or $PSBoundParameters.ContainsKey('Help')) {
    Show-Help
    exit 0
}

Write-Host "🗂️  Zettelkasten CLI Windows Installer" -ForegroundColor Cyan
Write-Host ""

# Verify system requirements
Test-Dependencies

# Check if already installed
$ExistingInstall = Join-Path $InstallPath "bin\zettel.exe"
if ((Test-Path $ExistingInstall) -and !$Force) {
    Write-WarnMessage "Zettel appears to be already installed at $InstallPath"
    Write-InfoMessage "Use -Force to reinstall, or choose a different -InstallPath"
    
    $Response = Read-Host "Continue anyway? (y/N)"
    if ($Response -notmatch '^[Yy]') {
        Write-InfoMessage "Installation cancelled"
        exit 0
    }
}

# Get version to install
if (!$Version) {
    $Version = Get-LatestVersion
}

Write-InfoMessage "Installing zettel v$Version..."

# Download and extract
$TempDir = Get-ZettelRelease -Version $Version

try {
    # Install binaries
    $BinDir = Install-ZettelBinaries -SourceDir $TempDir -TargetDir $InstallPath
    
    # Configure system integration
    Add-ToSystemPath -Path $BinDir
    
    if ($CreateShortcuts) {
        New-DesktopShortcuts -BinDir $BinDir
    }
    
    if ($SetupProfile) {
        $CompletionsDir = Join-Path $InstallPath "completions"
        Set-PowerShellProfile -CompletionsDir $CompletionsDir
    }
    
    # Verify installation
    if (Test-Installation -BinDir $BinDir) {
        Show-PostInstallInstructions -InstallPath $InstallPath -BinDir $BinDir
    } else {
        Write-ErrorMessage "Installation verification failed"
        exit 1
    }
}
finally {
    # Cleanup
    if (Test-Path $TempDir) {
        Remove-Item $TempDir -Recurse -Force
    }
}
