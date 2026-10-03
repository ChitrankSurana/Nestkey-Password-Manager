# How to rename the app

Step-by-step guide to changing the app's name everywhere it appears. Written against the repo as of
`main` @ 2026-10-04.

## 0. Read this first

The app has **no single "app name" setting**. The name lives in five different layers, and the layers
have very different blast radius:

| Layer | Examples | Risk if changed |
|---|---|---|
| **A. Display name** (what users see) | window title, sidebar brand, installer name, docs | None. Safe. |
| **B. Bundle identity** | Tauri `productName`, `identifier` | Changes install folder and WebView data folder. Moves user prefs. |
| **C. Internal names** | Cargo crate names, `pwmgr_core::` imports, binary names | Large mechanical edit. Safe if done completely. Breaks build if half-done. |
| **D. Wire/protocol names** | `X-PWMGR-CSRF` header, `pwmgr-session-locked` event, `pwmgr:*` localStorage keys, SSH pipe name | Cosmetic. Must change in lockstep on both sides. Not worth it. |
| **E. On-disk data and crypto** | `~/.pwmgr` directory, `pwmgr-sentinel-v1` | **Never change without a migration. Can lock users out of their vault.** |

**Recommendation:** do layer A only (the app already shows "LocalVault" in the Make UI, so most of
it is done). Do B if you ship installers. Do C only if you also want the repo and binaries renamed.
Skip D. Never touch E.

Throughout this guide `OLD` = current name, `NEW` = your new name. Pick three spellings up front:

| Form | Used for | Example |
|---|---|---|
| Display name | UI text, window titles, `productName` | `Acme Vault` |
| kebab-case slug | Cargo package names, npm name, binary names | `acme-vault` |
| snake_case slug | Rust lib/crate identifiers | `acme_vault` |
| reverse-DNS id | Tauri `identifier` | `com.acme.vault` |

---

## 1. Before you start

1. Branch off `main`: `git checkout main && git pull && git checkout -b chore/rename-app`.
2. Run the baseline so you know what "green" is:
   - `cargo test --workspace`
   - `npm test`
   - `npm run build`
3. Back up your real vault: copy `%USERPROFILE%\.pwmgr` somewhere safe. (Layer E is untouched by
   this guide, but backups are cheap.)
4. Make sure no app instance is running (`Get-Process pwmgr-app, pwmgr-viewer, pwmgr-web`).
5. List every current hit so you can tick them off:
   ```
   git grep -n -i -E "LocalVault|pwmgr|tauri-app|tauri_app|Password Manager|Tauri \+ React"
   ```

---

## 2. Layer A — Display name (safe)

Current state: the Make UI and main window already say **LocalVault**. Classic UI, New UI, Viewer and
`package.json` still show template or generic names. Make all of them consistent.

### 2.1 Window and page titles

| File | Line (approx.) | Current | Change to |
|---|---|---|---|
| `src-tauri/src/lib.rs` | ~79 `.title("LocalVault")` | `LocalVault` | `NEW` |
| `index.html` | 7 `<title>` | `Tauri + React + Typescript` | `NEW` |
| `makeui.html` | 7 `<title>` | `LocalVault` | `NEW` |
| `newui.html` | 7 `<title>` | `Vault` | `NEW` |
| `viewer.html` | 7 `<title>` | `Password Manager — Viewer` | `NEW — Viewer` |
| `viewer-src-tauri/tauri.conf.json` | `app.windows[0].title` | `Password Manager — Viewer` | `NEW — Viewer` |

`index.html` also affects the Classic UI window tab and the web portal, which serve the same HTML.

### 2.2 In-app text (Make UI)

| File | What | Current |
|---|---|---|
| `src/makeui/MakeVaultShell.tsx` ~98 | top-bar brand `<span>` | `LocalVault` |
| `src/makeui/components/Sidebar.tsx` ~246 | sidebar brand `<div>` | `LocalVault` |
| `src/makeui-main.tsx` ~33 | error fallback text: "Something went wrong loading the LocalVault interface." | `LocalVault` |

Also search the Classic and New UI screens for a hard-coded brand:
`git grep -n -i -E "Vault|Password Manager" -- src/screens src/newui src/App.tsx`. Today only code
comments match, but re-check after any UI work lands.

### 2.3 Installer and product name

| File | Key | Current | Change to |
|---|---|---|---|
| `src-tauri/tauri.conf.json` | `productName` | `tauri-app` | `NEW` (display name) |
| `viewer-src-tauri/tauri.conf.json` | `productName` | `Password Manager Viewer` | `NEW Viewer` |
| `src-tauri/tauri.conf.json` | `version` | `0.1.0` | set a real version |

`productName` is the name shown in Windows "Apps & features", the Start-menu entry, and the NSIS/MSI
file name. It also decides the install folder (the UAT notes show an install under
`%LOCALAPPDATA%\tauri-app`). **After you change it, uninstall the old build first**, or Windows keeps
both side by side.

`src-tauri/tauri.dev.conf.json` only overrides the dev CSP. No name there.

### 2.4 Icons

Icons are the other half of "the name" in the taskbar and installer. If you have a new logo, replace
the files in `src-tauri/icons/` (the Viewer reuses them via `../src-tauri/icons/...`). Regenerate with
`npx tauri icon path/to/logo.png`. Also replace the favicon: `index.html`, `viewer.html` and
`newui.html` still point at the template's `/vite.svg`, and `public/` still holds `vite.svg` and
`tauri.svg`.

### 2.5 Descriptions and metadata

| File | Key | Current |
|---|---|---|
| `src-tauri/Cargo.toml` | `description` | `A Tauri App` (template) |
| `src-tauri/Cargo.toml` | `authors` | `["you"]` (template) |
| `viewer-src-tauri/Cargo.toml` | `description` | `Password Manager - Read-only Viewer` |
| `web-src-tauri/Cargo.toml` | `description` | `Password Manager - Localhost Web Mode` |

These do not affect runtime. Update them for hygiene. Changing only these keys does not touch
`Cargo.lock`.

### 2.6 Docs and README

| File | Change |
|---|---|
| `README.md` line 1 | `# 🔐 Password Manager` title, plus the intro paragraph |
| `docs/README.md` | index intro, if it names the app |
| `docs/guides/*.md`, `docs/USAGE_GUIDE.md`, `docs/FRESH_MACHINE_SETUP.md` | user-facing mentions |
| `docs/design/*.md` | PRD, TRD, ARCHITECTURE, THREAT_MODEL, SECURITY_CONCERNS |
| `docs/testing/*.md` | test scripts that quote window titles or installer names |

Generic uses of the phrase "password manager" (the product category) can stay. Only change the
product's own name. Do **not** rewrite frozen history: `.planning/**`, `docs/Plans/**`,
`docs/superpowers/**` and `docs/research/**` record what was true at the time. Leave them.
`figma-make/` is a separate prototype that also says "LocalVault"; leave it unless you want the
prototype to match.

Counts at time of writing (for scoping, not for blind find-and-replace): "LocalVault" appears in 26
places across 13 docs/planning/prototype files, "Password Manager" in 51 places across 30.

### 2.7 npm package name

`package.json` `"name": "tauri-app"` → `NEW` kebab-case slug. Then update the top-level `name` in
`package-lock.json` (two spots: the root `"name"` and `packages[""].name`), or run
`npm install --package-lock-only` to regenerate. Nothing in `tauri.conf.json`, the scripts or CI reads
this name, so it is cosmetic.

### 2.8 Verify layer A

```
npm test
npm run build
npm run tauri:dev
```
Check: window title bar, taskbar label, sidebar brand, Viewer title (`npm run viewer:dev`).

---

## 3. Layer B — Bundle identity

Only needed if the `identifier` still carries the old name and you care.

`src-tauri/tauri.conf.json` → `identifier`: `com.pwmgr.app` → `com.<you>.<name>`
`viewer-src-tauri/tauri.conf.json` → `identifier`: `com.pwmgr.viewer` → `com.<you>.<name>.viewer`

What changes when the identifier changes:
- Tauri keys the **WebView2 data folder** (localStorage, cache) and the **single-instance lock**
  (`tauri-plugin-single-instance` in `src-tauri/src/lib.rs`) off the identifier. The app starts with a
  fresh WebView profile.
- **Lost:** only per-browser prefs stored in localStorage: remembered last username
  (`pwmgr:lastUsername`), sidebar open/closed state. Users re-tick them. The vault, config and
  backups are **not** affected, because they live in `~/.pwmgr`, not the WebView folder.
- A new identifier installs as a **different app**. Uninstall the old one first.
- Use only letters, digits, `-` and `.` in the identifier. Do not end it in `.app` (macOS warns).

Do not edit `identifier: "default"` in the two `capabilities/default.json` files. That is a Tauri
capability id, not the app identifier. The window label **must stay `main`** (see the comment in
`src-tauri/src/lib.rs`): the capability files scope clipboard and dialog permissions to it.

---

## 4. Layer C — Internal crate and binary names (optional, mechanical)

This renames what developers and Task Manager see: `pwmgr-app.exe`, `pwmgr-viewer.exe`,
`pwmgr-web.exe`, and `use pwmgr_core::...`. It does **not** change what end users see. Skip it
unless you want the whole repo to match.

Pick slugs, e.g. `acme-core`, `acme-app`, `acme-viewer`, `acme-web` (and the matching snake_case).
Do the whole of section 4 in one commit, then build. A partial rename will not compile.

### 4.1 Cargo manifests

| File | Key | Current |
|---|---|---|
| `core/Cargo.toml` | `[package] name` | `pwmgr-core` |
| `src-tauri/Cargo.toml` | `[package] name` | `pwmgr-app` |
| `src-tauri/Cargo.toml` | `[lib] name` | `tauri_app_lib` |
| `viewer-src-tauri/Cargo.toml` | `[package] name` | `pwmgr-viewer` |
| `viewer-src-tauri/Cargo.toml` | `[lib] name` | `pwmgr_viewer_lib` |
| `web-src-tauri/Cargo.toml` | `[package] name` | `pwmgr-web` |
| `web-src-tauri/Cargo.toml` | `[lib] name`, `[[bin]] name` | `pwmgr_web`, `pwmgr-web` |
| `src-tauri/Cargo.toml`, `viewer-src-tauri/Cargo.toml`, `web-src-tauri/Cargo.toml` | dependency `pwmgr-core = { path = "../core" }` | rename the key |

The root `Cargo.toml` lists workspace members by **directory** (`core`, `src-tauri`,
`viewer-src-tauri`, `web-src-tauri`). Those are paths, not names. Leave them (see 4.4).

### 4.2 Rust source

1. `main.rs` entry points call the lib by name:
   - `src-tauri/src/main.rs`: `tauri_app_lib::run()`
   - `viewer-src-tauri/src/main.rs`: `pwmgr_viewer_lib::run()`
   - `web-src-tauri/src/main.rs` (uses `pwmgr_web`)
2. Every `use pwmgr_core::...` and fully-qualified `pwmgr_core::` path. About 83 uses across 16 files:
   `core/tests/end_to_end.rs`, `src-tauri/src/{commands,lib,biometric_provider,ssh_agent_service}.rs`,
   `viewer-src-tauri/src/{commands,lib}.rs`, `web-src-tauri/src/{handlers,error_map,kdf_guard,...}.rs`,
   `web-src-tauri/tests/http_integration.rs`.
3. Comments that mention `pwmgr-core`, `pwmgr-web`, `pwmgr-app`. Harmless, but update for accuracy.

Do it with a single scoped replace per slug and review the diff:
```
git grep -l -E "pwmgr_core|pwmgr-core" -- '*.rs' '*.toml'
```
**Do not** blanket-replace the bare string `pwmgr`. That also matches layers D and E, which must not
change (sections 5 and 6).

### 4.3 Regenerate the lockfile

```
cargo update -w
```
Cargo rewrites the package names in `Cargo.lock`. Commit it. Do not hand-edit it.

### 4.4 Folders (skip)

Folder names `core/`, `src-tauri/`, `viewer-src-tauri/`, `web-src-tauri/` are generic and referenced
from the workspace manifest, `tauri.conf.json` (`frontendDist`, icon paths such as
`../src-tauri/icons/...`), `vite.config.ts` (watch ignore globs), `package.json` scripts
(`cd viewer-src-tauri`), CI, `scripts/setup-dependencies.ps1` and many docs. Renaming them means
editing every one of those. Do not.

### 4.5 Scripts, CI and docs that name binaries

- `scripts/setup-dependencies.ps1` line ~14 mentions the crate list in help text.
- `.github/workflows/ci.yml` runs `cargo test --workspace`. It names no crate, so it needs no edit.
- Docs that tell people to run `Get-Process pwmgr-app` or kill `pwmgr-app.exe` / `pwmgr-web`:
  `docs/testing/UAT_USER_TESTING_GUIDE.md` and the current testing docs. Update those. Leave the
  historical plans alone.

### 4.6 Verify layer C

```
cargo clean
cargo test --workspace
npm run build
npm run tauri:build
```
Confirm the built exe names, then run the Viewer and web portal once each.

---

## 5. Layer D — Wire and protocol names (leave alone)

These contain `pwmgr` but are internal strings no user sees. Renaming gains nothing and costs risk.
If you still insist, each pair **must change together**, and a mismatch makes web mode fail with
403 or silently stops events.

| String | Where | Counterpart |
|---|---|---|
| `X-PWMGR-CSRF` / `x-pwmgr-csrf` | `src/lib/api.ts` (3 places), `web-src-tauri/src/middleware.rs` (`CSRF_HEADER`), ~110 uses in `web-src-tauri/tests/http_integration.rs` | server and client must match, or every web-mode POST is rejected |
| `pwmgr-session-locked` | `src/lib/api.ts` `SESSION_LOCKED_EVENT` | `window` event used only inside the frontend (`api.ts` dispatches, `App.tsx` listens). No Rust side. |
| `pwmgr:rememberLastUsername`, `pwmgr:lastUsername`, `pwmgr:sidebarVaultOpen`, `pwmgr:sidebarFoldersOpen` | `src/lib/lastUsername.ts`, `src/lib/sidebarPrefs.ts` | renaming resets users' saved prefs |
| `\\.\pipe\pwmgr-ssh-agent` | `src-tauri/src/ssh_agent_service.rs` `PIPE_NAME` | **users' `SSH_AUTH_SOCK` / `ssh` config point at this pipe name; renaming breaks every configured client** |

The SSH pipe is the one that bites: if you change it, document the new pipe path in
`docs/testing/HOW_TO_TEST_SSH_AGENT.md` and tell users to update their SSH config.

---

## 6. Layer E — Data and crypto (do NOT change)

| String | Where | Why it must stay |
|---|---|---|
| `b"pwmgr-sentinel-v1"` | `core/src/auth.rs` `SENTINEL_PLAINTEXT` | Registration encrypts this value into the vault. Login decrypts and compares it. Change it and **every existing vault fails to unlock**, as if the master password were wrong. It is a format constant, not a brand. |
| `~/.pwmgr` | `src-tauri/src/lib.rs`, `viewer-src-tauri/src/lib.rs`, `web-src-tauri/src/main.rs` (`home.join(".pwmgr")`) | Holds `registry.db`, `vaults/`, `backups/`, `config.json`. Rename the folder in code and the app sees an empty install. Users appear to have lost everything. |

If you really want a new data folder name, it needs a migration, not a rename:
1. In all three entry points, resolve the new path.
2. If the new path does not exist and `~/.pwmgr` does, move or copy the old folder over **before**
   `AppPaths::ensure_app_dirs()` runs.
3. Keep reading from the old folder as a fallback for at least one release.
4. Add a test in `core/` that opens a vault from a legacy-path fixture.
5. Update every doc that mentions `~/.pwmgr` (about 108 hits across 38 docs/planning files).

Never change `SENTINEL_PLAINTEXT`. If its value ever must change, you need a versioned sentinel that
accepts both old and new values on unlock.

---

## 7. Final checklist

- [ ] Layer A: titles, brand text, `productName`, icons, favicon, metadata, docs.
- [ ] Layer B (optional): `identifier` for app and Viewer. Old build uninstalled.
- [ ] Layer C (optional): all crate/lib/bin names, `use` paths, `Cargo.lock` regenerated, in one commit.
- [ ] Layers D and E: untouched. `git grep -n "pwmgr-sentinel-v1\|\.pwmgr"` still shows the same hits.
- [ ] `cargo test --workspace`, `npm test`, `npm run build` all green.
- [ ] Launch the main app, Viewer and web portal. Register a new user, log in, add a credential,
      lock, unlock.
- [ ] **Open your pre-rename vault** (from the backup in step 1.3). It must unlock. This is the test
      that proves layer E is intact.
- [ ] CI is manual-only in this repo. Dispatch it yourself when you open the PR.

## 8. Quick reference: every file that contains a name

| Group | Files |
|---|---|
| Window and page titles | `src-tauri/src/lib.rs`, `index.html`, `makeui.html`, `newui.html`, `viewer.html`, `viewer-src-tauri/tauri.conf.json` |
| Brand text in UI | `src/makeui/MakeVaultShell.tsx`, `src/makeui/components/Sidebar.tsx`, `src/makeui-main.tsx` |
| Bundle config | `src-tauri/tauri.conf.json`, `viewer-src-tauri/tauri.conf.json` |
| JS package | `package.json`, `package-lock.json` |
| Cargo manifests | `core/Cargo.toml`, `src-tauri/Cargo.toml`, `viewer-src-tauri/Cargo.toml`, `web-src-tauri/Cargo.toml`, `Cargo.lock` |
| Rust entry points | `src-tauri/src/main.rs`, `viewer-src-tauri/src/main.rs`, `web-src-tauri/src/main.rs` |
| Rust `pwmgr_core::` users | 16 `.rs` files across all four crates |
| Icons and favicon | `src-tauri/icons/*`, `public/*`, favicon `<link>` in 3 HTML files |
| Docs | `README.md`, `docs/README.md`, `docs/guides`, `docs/design`, `docs/testing`, `docs/*.md` |
| **Leave alone** | `core/src/auth.rs` (sentinel), the three `.pwmgr` entry points, `ssh_agent_service.rs` pipe, CSRF header, localStorage keys |
