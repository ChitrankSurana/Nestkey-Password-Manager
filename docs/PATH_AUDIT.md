# PATH Environment Variable Audit — Project-Required Entries

Scope: only PATH entries this project (Rust workspace `core`/`src-tauri`/`viewer-src-tauri`/
`web-src-tauri` + Vite/React frontend) actually needs to build, run, and test. Everything else in
the full-system audit (IDEs, media tools, other language runtimes, etc.) is dropped as not a project
dependency. Audit date: 2026-07-19.

## Required

| Entry | Why required | Present |
|---|---|---|
| `C:\Users\Chitrank Surana\.cargo\bin` | Rust toolchain — cargo/rustc build the whole workspace | YES |
| `F:\Installed_softwares\NodeJS\` | node/npm/npx — Vite dev server, `tsc`, `vitest`, frontend build | YES |
| `C:\Program Files\Git\cmd` | Git — version control for this repo | YES |
| `C:\WINDOWS\System32\OpenSSH\` | ssh — repo's `origin` remote is an SSH URL (`git@github-account-1:...`) | YES |
| `C:\Strawberry\perl\bin` | Perl — `core`'s `rusqlite` dependency uses `bundled-sqlcipher-vendored-openssl`, which compiles OpenSSL from source; OpenSSL's `Configure` script requires Perl | YES |
| `C:\Strawberry\perl\site\bin` | Perl site scripts, same reason as above | YES |
| `C:\WINDOWS\system32` | Windows core — process/DLL resolution for every tool above | YES |
| `C:\WINDOWS` | Windows core | YES |
| `C:\WINDOWS\System32\WindowsPowerShell\v1.0\` | PowerShell — shell used to drive builds/tests | YES |

## Required, but not currently on PATH

| Entry | Why required | Present | Notes |
|---|---|---|---|
| MSVC `link.exe`/`cl.exe` dir (`...\VC\Tools\MSVC\<ver>\bin\Hostx64\x64`) | Linker + C compiler for the `x86_64-pc-windows-msvc` target, and for compiling vendored OpenSSL/SQLCipher (`rusqlite` feature above) | NO | VS 2022 Build Tools are installed; `cargo build` still works today because rustc auto-locates MSVC via the registry/vswhere without needing PATH. Don't add permanently — conflicts with other `link`/`cl`-named tools. Use "Developer PowerShell for VS 2022" or `vcvars64.bat` if a build ever needs it directly. |

## Not required for this project (present on system, excluded here)

Everything else from the full system audit — Python, ffmpeg, Ollama, LM Studio, PowerToys, NVIDIA
tools, Chocolatey, .NET SDK, GitHub CLI, Cursor/Warp/VS Code/Antigravity editor bins, Copilot-chat's
auto-injected debug dirs, `Wbem`, WindowsApps shims, global npm folder, `.local\bin` — none of these
are touched by this project's build/test/run path. See the full-system audit (git history / earlier
version of this file) if you need that broader picture again.

## Maintenance notes

- Backup before editing: `[Environment]::GetEnvironmentVariable('PATH','User') | Out-File path-backup.txt`
- Edit via `[Environment]::SetEnvironmentVariable('PATH', $value, 'User')` or GUI (`sysdm.cpl` → Advanced → Environment Variables) — never hand-edit the registry directly.
- Restart any open terminal/IDE after a PATH change — running shells don't pick it up.
- Dead-entry / duplicate check:
  ```powershell
  $env:PATH -split ';' | Where-Object {$_ -ne ''} | Where-Object { -not (Test-Path $_) }   # dead
  $env:PATH -split ';' | Group-Object | Where-Object {$_.Count -gt 1}                       # duplicates
  ```
