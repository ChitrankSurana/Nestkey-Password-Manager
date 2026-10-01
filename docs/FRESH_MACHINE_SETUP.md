# Fresh Machine Setup

Every step to go from a brand-new Windows PC to all three apps running. Windows-only today (see
`docs/KNOWN_LIMITATIONS.md` §4 — macOS/Linux builds aren't done yet).

All three apps share one data directory (`~/.pwmgr` — registry, vaults, config, backups, all the
same files), so you only register a user once and can log into that same account from any of the
three.

---

## 1. Prerequisites

- Windows 10 (2004/build 19041) or later, or Windows 11. `winget` (used below) ships built in on
  these; if it's missing, install "App Installer" from the Microsoft Store first.
- Admin rights (the installer steps need them).
- ~2 GB free disk, a real internet connection (first build downloads and compiles OpenSSL +
  SQLCipher from source — see step 4).

## 2. Install system dependencies

Two options — pick one:

**Option A — automated (recommended):**
```powershell
git clone https://github.com/ChitrankSurana/Password_Manager_Local.git
cd Password_Manager_Local
.\scripts\setup-dependencies.ps1
```
That script checks for and installs Git, Rust, Strawberry Perl, MSVC Build Tools, Node.js, and the
WebView2 Runtime via `winget`, then runs `npm install` + `cargo fetch` for you. See the script's
header comment for why each one is needed. If it reports anything still `MISSING` after running,
close the terminal, open a new one, and re-run it once (PATH changes from a fresh install don't
always reach an already-open shell).

**Option B — manual**, if you'd rather install by hand:

| Tool | What for | Install |
|---|---|---|
| Git | clone/manage the repo | `winget install --id Git.Git -e` |
| Rust (rustup) | builds the whole Cargo workspace | `winget install --id Rustlang.Rustup -e`, then `rustup default stable-x86_64-pc-windows-msvc` |
| MSVC Build Tools (C++ workload) | linker + compiler for the msvc target and for vendored OpenSSL/SQLCipher | `winget install --id Microsoft.VisualStudio.2022.BuildTools -e --override "--quiet --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"` |
| Strawberry Perl | needed by vendored OpenSSL's `Configure` script (`rusqlite`'s `bundled-sqlcipher-vendored-openssl` feature) | `winget install --id StrawberryPerl.StrawberryPerl -e` |
| Node.js (LTS) | npm/npx run Vite/tsc/vitest for the frontend | `winget install --id OpenJS.NodeJS.LTS -e` |
| WebView2 Runtime | renders the Tauri app window | usually already present; `winget install --id Microsoft.EdgeWebView2Runtime -e` if not |

See `docs/PATH_AUDIT.md` for the exact PATH entries these tools need and why.

## 3. Clone the repo (if you used Option B above)

```powershell
git clone https://github.com/ChitrankSurana/Password_Manager_Local.git
cd Password_Manager_Local
```

## 4. Put Rust/Perl on PATH for this shell

New terminals don't have `.cargo\bin`/Strawberry Perl on PATH until you open a fresh one after
install, or prepend manually every session:

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;C:\Strawberry\perl\bin;C:\Strawberry\c\bin;" + $env:PATH
```

## 5. Install project dependencies (skip if `setup-dependencies.ps1` already did this)

```powershell
npm install
```

## 6. Verify the build

```powershell
cargo test --workspace   # first run compiles vendored OpenSSL + SQLCipher from source — can take
                          # 5-15+ minutes depending on hardware; subsequent runs are fast (cached)
npm test
npm run build             # produces ./dist (index.html, viewer.html, assets) — needed by step 7's Web Portal
```
All three should pass/succeed with 0 errors before you go further — see
`docs/testing/HOW_TO_TEST_MAIN_APP.md` for what each one covers.

## 7. Run each app

**Main App** (full read/write desktop app):
```powershell
npm run tauri:dev
```
(`tauri:dev` passes `src-tauri/tauri.dev.conf.json`. A single-instance guard means a second
launch focuses the running window instead of opening another.)

**Viewer App** (read-only desktop app — separate crate, own `tauri.conf.json`):
```powershell
npm run viewer:dev
```

**Web Portal** (localhost browser access — plain Axum server, not Tauri):
```powershell
npm run build              # if you haven't already (step 6)
cargo run -p pwmgr-web      # must be run from the repo root — serves ./dist relative to cwd
```
Then open `http://127.0.0.1:7373` in a browser. Binds to `127.0.0.1` only, never reachable from
another machine on the network.

## 8. First run

1. In the **Main App**, register a new account (username + master password). This creates
   `~/.pwmgr/registry.db` and your vault file.
2. Add a site/credential to confirm the vault works end to end.
3. Log into the **Viewer App** and **Web Portal** with the same username/password — same account,
   same data, three surfaces.

See `docs/guides/HOW_TO_USE_APP.md` (current, all three UIs), `docs/guides/HOW_TO_USE_VIEWER_APP.md` and
`docs/guides/HOW_TO_USE_WEB_PORTAL.md` for what each app can do, and `docs/testing/` for full manual test
walkthroughs. After first login, Settings → Interface switches between the Classic, New and Make UIs.

## Troubleshooting

- **`cargo`/`rustc`/`perl` not found** → step 4's PATH prepend, or open a new terminal after
  installing.
- **Build fails compiling `openssl-sys`/`libsqlite3-sys`** → MSVC Build Tools' C++ workload is
  missing; re-run `scripts/setup-dependencies.ps1` or the manual `winget` command in step 2.
- **Tauri window never opens / blank window** → WebView2 Runtime missing; install via step 2's
  table.
- **`npm run viewer:dev` doesn't pick up its config** → from inside `viewer-src-tauri/`, run
  `npx tauri dev --config tauri.conf.json` explicitly.
- **Black/blank window on launch (WebView2)** → a mitigation shipped in commit `c891ce6`; update to
  the latest build, and make sure the WebView2 Runtime is installed (step 2).
