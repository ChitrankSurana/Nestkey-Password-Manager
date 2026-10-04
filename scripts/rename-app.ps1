<#
.SYNOPSIS
  Renames the app everywhere, then verifies the whole workspace still builds and passes tests.

.DESCRIPTION
  Asks for the display name (e.g. "Nestkey Password Manager") and a short slug (e.g. "nestkey"),
  then updates every place described in docs/guides/RENAME_APP.md layers A, B and C:
    A  window/page titles, UI brand text, productName, package/crate descriptions, docs
    B  Tauri bundle identifiers
    C  Cargo crate/lib/bin names, `use <slug>_core::` paths, npm package name, lockfiles
  It NEVER touches layers D/E (the on-disk ~/.pwmgr folder, the "pwmgr-sentinel-v1" crypto
  constant, the SSH agent pipe name, the CSRF header, localStorage keys). Changing those can lock
  users out of their vault. The script verifies they are still present after the rename.

  Old names are auto-detected from the repo, so the script can be run again later with a new name.

.PARAMETER Name        Display name, e.g. "Nestkey Password Manager". Prompted if omitted.
.PARAMETER Slug        kebab-case slug, e.g. "nestkey". Prompted if omitted.
.PARAMETER IdPrefix    Reverse-DNS prefix for the Tauri identifiers. Default: com.<slug>
.PARAMETER DryRun      Show what would change; write nothing; run no checks.
.PARAMETER SkipChecks  Rename only; do not run cargo/npm verification.

.EXAMPLE
  .\scripts\rename-app.ps1
  .\scripts\rename-app.ps1 -Name "Acme Vault" -Slug acme-vault -DryRun
#>
[CmdletBinding()]
param(
  [string]$Name,
  [string]$Slug,
  [string]$IdPrefix,
  [switch]$DryRun,
  [switch]$SkipChecks
)

$ErrorActionPreference = 'Stop'
Set-Location (Split-Path -Parent $PSScriptRoot)
$root = (Get-Location).Path
$utf8 = New-Object System.Text.UTF8Encoding($false)

function Read-Text($p) { [IO.File]::ReadAllText((Join-Path $root $p), $utf8) }
function Show($m, $c = 'Gray') { Write-Host $m -ForegroundColor $c }
function Die($m) { Show "ERROR: $m" Red; exit 1 }

# ---------- 1. gather input ----------
if (-not $Name) { $Name = (Read-Host 'New app display name (e.g. "Nestkey Password Manager")').Trim() }
if (-not $Name) { Die 'Name must not be empty.' }
if ($Name -match '[\\/:*?"<>|]') { Die 'Name contains characters not allowed in a file name: \ / : * ? " < > |' }

if (-not $Slug) {
  $default = ($Name.Split(' ')[0] -replace '[^A-Za-z0-9]', '').ToLower()
  $in = (Read-Host "Short slug for crates/binaries/npm [$default]").Trim()
  $Slug = if ($in) { $in } else { $default }
}
$Slug = $Slug.ToLower()
if ($Slug -notmatch '^[a-z][a-z0-9]*(-[a-z0-9]+)*$') { Die 'Slug must be kebab-case: lowercase letters, digits and single dashes, starting with a letter.' }
$snake = $Slug -replace '-', '_'
if (-not $IdPrefix) { $IdPrefix = "com.$($Slug -replace '-', '')" }
if ($IdPrefix -notmatch '^[a-z][a-z0-9]*(\.[a-z][a-z0-9-]*)+$') { Die 'IdPrefix must look like com.example (lowercase, dot separated).' }

# ---------- 2. detect current names ----------
$libRs = Read-Text 'src-tauri/src/lib.rs'
if ($libRs -notmatch '\.title\("([^"]+)"\)') { Die 'Cannot detect current display name in src-tauri/src/lib.rs (.title("...")).' }
$oldName = $Matches[1]

$coreToml = Read-Text 'core/Cargo.toml'
if ($coreToml -notmatch '(?m)^name = "([a-z0-9-]+)-core"') { Die 'Cannot detect current slug from core/Cargo.toml.' }
$oldSlug = $Matches[1]
$oldSnake = $oldSlug -replace '-', '_'

$appToml = Read-Text 'src-tauri/Cargo.toml'
if ($appToml -notmatch '(?ms)\[lib\].*?name = "([a-z0-9_]+)"') { Die 'Cannot detect current [lib] name in src-tauri/Cargo.toml.' }
$oldAppLib = $Matches[1]

Show "Current : name='$oldName'  slug='$oldSlug'  lib='$oldAppLib'"
Show "New     : name='$Name'  slug='$Slug'  idprefix='$IdPrefix'" Cyan
if ($oldName -ceq $Name -and $oldSlug -ceq $Slug) { Die 'New name and slug equal the current ones. Nothing to do.' }

$dirty = git status --porcelain
if ($dirty -and -not $DryRun) { Show 'WARNING: working tree has uncommitted changes. Commit or stash first so the rename is one clean diff.' Yellow }

# ---------- 3. edit helpers ----------
$changed = New-Object System.Collections.Generic.List[string]
function Edit-File($rel, [scriptblock]$fn) {
  $full = Join-Path $root $rel
  if (-not (Test-Path $full)) { Die "Expected file missing: $rel" }
  $old = [IO.File]::ReadAllText($full, $utf8)
  $new = & $fn $old
  if ($new -cne $old) {
    if (-not $DryRun) { [IO.File]::WriteAllText($full, $new, $utf8) }
    $changed.Add($rel)
  }
}
function Set-Regex($rel, $pattern, $replacement, [switch]$All) {
  Edit-File $rel {
    param($t)
    $rx = New-Object System.Text.RegularExpressions.Regex($pattern, 'Multiline')
    if (-not $rx.IsMatch($t)) { Die "Pattern not found in ${rel}: $pattern" }
    if ($All) { $rx.Replace($t, $replacement) } else { $rx.Replace($t, $replacement, 1) }
  }
}
function Esc($s) { $s.Replace('$', '$$') }   # keep $ literal in replacement strings

$n = Esc $Name

# ---------- 4. Layer A/B: structured fields ----------
Show 'Updating titles, bundle config and metadata...'
Set-Regex 'src-tauri/src/lib.rs'                '\.title\("[^"]*"\)'                       ".title(`"$n`")"
Set-Regex 'index.html'                          '<title>[^<]*</title>'                     "<title>$n</title>"
Set-Regex 'makeui.html'                         '<title>[^<]*</title>'                     "<title>$n</title>"
Set-Regex 'newui.html'                          '<title>[^<]*</title>'                     "<title>$n</title>"
Set-Regex 'viewer.html'                         '<title>[^<]*</title>'                     "<title>$n $([char]0x2014) Viewer</title>"
Set-Regex 'src-tauri/tauri.conf.json'           '"productName":\s*"[^"]*"'                 "`"productName`": `"$n`""
Set-Regex 'src-tauri/tauri.conf.json'           '"identifier":\s*"[^"]*"'                  "`"identifier`": `"${IdPrefix}.app`""
Set-Regex 'viewer-src-tauri/tauri.conf.json'    '"productName":\s*"[^"]*"'                 "`"productName`": `"$n Viewer`""
Set-Regex 'viewer-src-tauri/tauri.conf.json'    '"identifier":\s*"[^"]*"'                  "`"identifier`": `"${IdPrefix}.viewer`""
Set-Regex 'viewer-src-tauri/tauri.conf.json'    '"title":\s*"[^"]*"'                       "`"title`": `"$n $([char]0x2014) Viewer`""
Set-Regex 'src-tauri/Cargo.toml'                '^description = ".*"'                      "description = `"$n`""
Set-Regex 'viewer-src-tauri/Cargo.toml'         '^description = ".*"'                      "description = `"$n - Read-only Viewer`""
Set-Regex 'web-src-tauri/Cargo.toml'            '^description = ".*"'                      "description = `"$n - Localhost Web Mode`""
Set-Regex 'package.json'                        '"name":\s*"[^"]*"'                        "`"name`": `"$Slug`""
Set-Regex 'package-lock.json'                   '^(\s*)"name":\s*"[^"]*"'                  "`${1}`"name`": `"$Slug`"" -All:$false
Edit-File 'package-lock.json' {                 # second occurrence: packages[""].name
  param($t)
  $rx = New-Object System.Text.RegularExpressions.Regex('("packages":\s*\{\s*"":\s*\{\s*"name":\s*)"[^"]*"')
  $rx.Replace($t, "`${1}`"$Slug`"", 1)
}

# UI brand text (literal old display name -> new)
foreach ($f in 'src/makeui/MakeVaultShell.tsx', 'src/makeui/components/Sidebar.tsx', 'src/makeui-main.tsx') {
  Edit-File $f { param($t) $t.Replace($oldName, $Name) }
}

# ---------- 5. Layer C: crate / lib / bin names ----------
Show 'Updating crate, library and binary names...'
$tokens = [ordered]@{
  "$oldSlug-core"            = "$Slug-core"
  "${oldSnake}_core"         = "${snake}_core"
  "$oldSlug-app"             = "$Slug-app"
  "$oldSlug-viewer"          = "$Slug-viewer"
  "$oldSlug-web"             = "$Slug-web"
  "${oldSnake}_viewer_lib"   = "${snake}_viewer_lib"
  "${oldSnake}_web"          = "${snake}_web"
  $oldAppLib                 = "${snake}_app_lib"
}

$skipPrefixes = @('.planning/', 'docs/Plans/', 'docs/superpowers/', 'docs/research/', 'figma-make/', 'graphify-out/',
                  'docs/guides/RENAME_APP.md', 'docs/testing/UAT_FINDINGS_', 'docs/RELEASE_NOTES_', 'scripts/rename-app.ps1')
$skipFiles = @('Cargo.lock', 'package-lock.json')
$exts = '\.(rs|toml|md|json|ps1|yml|yaml|ts|tsx|html|css)$'

$files = git ls-files | Where-Object {
  $f = $_
  ($f -match $exts) -and ($skipFiles -notcontains $f) -and -not ($skipPrefixes | Where-Object { $f.StartsWith($_) })
}

foreach ($f in $files) {
  Edit-File $f {
    param($t)
    foreach ($k in $tokens.Keys) { $t = $t.Replace($k, $tokens[$k]) }
    # display name in prose/UI docs (only when it differs)
    if ($f -match '^(README\.md|docs/)' ) { $t = $t.Replace($oldName, $Name) }
    $t
  }
}

# ---------- 6. report ----------
Show ''
Show "Files changed: $($changed.Count)" Cyan
$changed | Sort-Object | ForEach-Object { Show "  $_" }

if ($DryRun) { Show "`nDry run: nothing written." Yellow; exit 0 }

# ---------- 7. checks ----------
$failed = New-Object System.Collections.Generic.List[string]
function Step($label, [scriptblock]$cmd) {
  Show "`n== $label ==" Cyan
  & $cmd
  if ($LASTEXITCODE -ne 0) { $failed.Add($label); Show "FAILED: $label" Red } else { Show "ok: $label" Green }
}

Show "`n== invariant: data layer untouched ==" Cyan
$inv = @(
  @{ f = 'core/src/auth.rs';                  p = 'pwmgr-sentinel-v1' },
  @{ f = 'src-tauri/src/lib.rs';              p = '.pwmgr' },
  @{ f = 'viewer-src-tauri/src/lib.rs';       p = '.pwmgr' },
  @{ f = 'web-src-tauri/src/main.rs';         p = '.pwmgr' },
  @{ f = 'src-tauri/src/ssh_agent_service.rs'; p = 'pwmgr-ssh-agent' },
  @{ f = 'web-src-tauri/src/middleware.rs';   p = 'x-pwmgr-csrf' }
)
foreach ($i in $inv) {
  if ((Read-Text $i.f).Contains($i.p)) { Show "ok: $($i.f) still has '$($i.p)'" Green }
  else { $failed.Add("invariant $($i.f)"); Show "FAILED: $($i.f) lost '$($i.p)'" Red }
}

Show "`n== leftover old names in live files ==" Cyan
$left = @()
foreach ($f in $files) {
  $t = Read-Text $f
  foreach ($k in $tokens.Keys) { if ($t.Contains($k)) { $left += "$f : $k" } }
}
if ($left.Count) { $failed.Add('leftover old names'); $left | ForEach-Object { Show "  $_" Red } } else { Show 'ok: none' Green }

Step 'cargo update -w (refresh Cargo.lock)'            { cargo update -w --offline }
Step 'cargo test --workspace'                          { cargo test --workspace }
Step 'npm test'                                        { npm test }
Step 'npm run build (tsc + vite)'                      { npm run build }

Show ''
if ($failed.Count) {
  Show "RENAME APPLIED BUT $($failed.Count) CHECK(S) FAILED:" Red
  $failed | ForEach-Object { Show "  - $_" Red }
  Show 'Review with: git diff   |   undo with: git checkout . ; git clean -fd (only if nothing else is uncommitted)' Yellow
  exit 1
}
Show "All checks passed. App is now '$Name' ($Slug)." Green
Show 'Reminders: uninstall the old build before installing the new one; manual UI smoke test (docs/testing) is still advised.' Yellow
