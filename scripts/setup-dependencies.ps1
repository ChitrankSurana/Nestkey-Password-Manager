<#
.SYNOPSIS
    Checks for and installs every system-level dependency this project needs to build/run
    (Rust toolchain, Node.js, Git, Strawberry Perl, MSVC Build Tools, WebView2 Runtime),
    then runs `npm install` and `cargo fetch` so the workspace is ready to build.

.DESCRIPTION
    Uses winget (built into Windows 10 2004+ / Windows 11) as the sole installer mechanism —
    no extra package manager needed. Safe to re-run: every check is idempotent, already-present
    tools are left alone.

    Why each dependency is needed:
      - Rust (rustup, stable-x86_64-pc-windows-msvc): builds the whole Cargo workspace
        (nestkey-core, src-tauri, viewer-src-tauri, web-src-tauri).
      - MSVC Build Tools (C++ workload): linker (link.exe) + C compiler for the msvc target,
        and to compile vendored OpenSSL/SQLCipher (see rusqlite feature below).
      - Strawberry Perl: `core`'s rusqlite dependency uses the
        "bundled-sqlcipher-vendored-openssl" feature, which compiles OpenSSL from source;
        OpenSSL's Configure script requires Perl.
      - Node.js: npm/npx run Vite, tsc, and vitest for the React/TypeScript frontend.
      - Git: clone/manage this repo.
      - WebView2 Runtime: the OS component Tauri 2 uses to render the app window on Windows.
        Present by default on most Win10 21H2+/Win11 machines, checked anyway since it's a
        real hard requirement, not optional.

.NOTES
    Run from the repo root: .\scripts\setup-dependencies.ps1
    If anything gets freshly installed, PATH changes may need a NEW terminal window to take
    full effect everywhere (this script refreshes its own session's PATH best-effort so it can
    keep going without one).
#>

$ErrorActionPreference = 'Stop'

function Refresh-SessionPath {
    # Winget-registered PATH changes land in the registry immediately but not in already-open
    # shells. Reloading from Machine+User here lets this script see a tool it just installed
    # without requiring the user to close and reopen the terminal.
    $machine = [Environment]::GetEnvironmentVariable('PATH', 'Machine')
    $user = [Environment]::GetEnvironmentVariable('PATH', 'User')
    $env:PATH = "$machine;$user"
}

function Test-ToolOnPath {
    param([string]$Name)
    return [bool](Get-Command $Name -ErrorAction SilentlyContinue)
}

function Test-WebView2Runtime {
    $clientId = '{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}' # Evergreen WebView2 Runtime
    $paths = @(
        "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\$clientId",
        "HKLM:\SOFTWARE\Microsoft\EdgeUpdate\Clients\$clientId",
        "HKCU:\SOFTWARE\Microsoft\EdgeUpdate\Clients\$clientId"
    )
    foreach ($p in $paths) { if (Test-Path $p) { return $true } }
    return $false
}

function Test-MsvcBuildTools {
    $vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
    if (-not (Test-Path $vswhere)) { return $false }
    $found = & $vswhere -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    return [bool]$found
}

function Install-Winget-Package {
    param([string]$Id, [string]$OverrideArgs = $null)
    Write-Host "  Installing $Id via winget ..." -ForegroundColor Yellow
    $args = @('install', '--id', $Id, '-e', '--accept-source-agreements', '--accept-package-agreements', '--silent')
    if ($OverrideArgs) { $args += @('--override', $OverrideArgs) }
    & winget @args
}

Write-Host "=== Checking system dependencies ===" -ForegroundColor Cyan

if (-not (Test-ToolOnPath 'winget')) {
    Write-Host "winget not found. This script needs it to install missing tools automatically." -ForegroundColor Red
    Write-Host "Install 'App Installer' from the Microsoft Store, then re-run this script." -ForegroundColor Red
    exit 1
}

$checks = [ordered]@{
    'Git'               = { Test-ToolOnPath 'git' }
    'Rust (cargo)'      = { Test-ToolOnPath 'cargo' }
    'Strawberry Perl'   = { Test-ToolOnPath 'perl' }
    'Node.js (npm)'     = { Test-ToolOnPath 'npm' }
    'MSVC Build Tools'  = { Test-MsvcBuildTools }
    'WebView2 Runtime'  = { Test-WebView2Runtime }
}

$status = [ordered]@{}
foreach ($name in $checks.Keys) { $status[$name] = & $checks[$name] }

Write-Host ""
foreach ($name in $status.Keys) {
    $mark = if ($status[$name]) { 'OK      ' } else { 'MISSING ' }
    $color = if ($status[$name]) { 'Green' } else { 'Red' }
    Write-Host ("  [{0}] {1}" -f $mark, $name) -ForegroundColor $color
}
Write-Host ""

if (-not $status['Git']) { Install-Winget-Package -Id 'Git.Git' }

if (-not $status['Rust (cargo)']) {
    Install-Winget-Package -Id 'Rustlang.Rustup'
    Refresh-SessionPath
    if (Test-ToolOnPath 'rustup') {
        rustup toolchain install stable-x86_64-pc-windows-msvc
        rustup default stable-x86_64-pc-windows-msvc
    }
}

if (-not $status['Strawberry Perl']) { Install-Winget-Package -Id 'StrawberryPerl.StrawberryPerl' }

if (-not $status['Node.js (npm)']) { Install-Winget-Package -Id 'OpenJS.NodeJS.LTS' }

if (-not $status['MSVC Build Tools']) {
    $override = '--quiet --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended'
    Install-Winget-Package -Id 'Microsoft.VisualStudio.2022.BuildTools' -OverrideArgs $override
}

if (-not $status['WebView2 Runtime']) { Install-Winget-Package -Id 'Microsoft.EdgeWebView2Runtime' }

Refresh-SessionPath
# Rust/Perl installs don't always land on the Machine/User PATH registry keys the same way a
# normal installer does — mirror the README's manual prepend so the rest of THIS script (and
# this shell, until you open a new one) can see them right away.
$env:PATH = "$env:USERPROFILE\.cargo\bin;C:\Strawberry\perl\bin;C:\Strawberry\c\bin;" + $env:PATH

Write-Host "=== Re-checking after install ===" -ForegroundColor Cyan
$anyMissing = $false
foreach ($name in $checks.Keys) {
    $ok = & $checks[$name]
    $mark = if ($ok) { 'OK      ' } else { 'MISSING ' }
    $color = if ($ok) { 'Green' } else { 'Red' }
    Write-Host ("  [{0}] {1}" -f $mark, $name) -ForegroundColor $color
    if (-not $ok) { $anyMissing = $true }
}
Write-Host ""

if ($anyMissing) {
    Write-Host "Some tools still show MISSING in this session even though winget installed them." -ForegroundColor Yellow
    Write-Host "Close this terminal, open a new one, and re-run this script to confirm." -ForegroundColor Yellow
    exit 1
}

$repoRoot = Split-Path -Parent $PSScriptRoot
Write-Host "=== Installing project dependencies ===" -ForegroundColor Cyan
Push-Location $repoRoot
try {
    Write-Host "  npm install ..."
    npm install
    Write-Host "  cargo fetch (downloads Rust crate sources; does not compile) ..."
    cargo fetch --manifest-path (Join-Path $repoRoot 'Cargo.toml')
}
finally {
    Pop-Location
}

Write-Host ""
Write-Host "All dependencies present. Next: see docs/FRESH_MACHINE_SETUP.md to run the app." -ForegroundColor Green
