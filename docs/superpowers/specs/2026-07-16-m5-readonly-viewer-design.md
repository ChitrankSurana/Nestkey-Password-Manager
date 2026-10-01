# M5 Design — Read-Only Viewer App

**Status:** Approved (brainstormed interactively), pending write-up as an implementation plan via
`superpowers:writing-plans`.
**Last updated:** 2026-07-16

## 1. Purpose

Deliver M5 per `docs/design/PRD.md`'s milestone table and user story 9: "if my main app is ever broken or
unavailable, I can open a separate read-only viewer, enter my master password, and see my data." A
second, standalone Tauri application that can log in, search, view, and copy existing credentials —
and cannot create, edit, delete, or otherwise mutate the vault, the registry, or config, even in
principle. `docs/ARCHITECTURE.md` already commits to the shape of this: "reuses `crypto` + `vault` +
`registry` as a library crate, compiled into a second, smaller Tauri binary with no write-path
commands registered — so 'read-only' is enforced by what code exists in the binary, not just by UI
restrictions." This design fills in everything that decision left open.

## 2. Scope

**In scope:**
- A new binary crate, `viewer-src-tauri`, added to the workspace. Depends only on `pwmgr-core` —
  never on `src-tauri` — so it is a fully standalone application, installable and runnable
  independently of the main app.
- A prep refactor promoting `AppState` (the in-memory session/paths holder) and the read-path DTOs
  (`SiteDto`, `CredentialDto`, `SearchResultDto`, `ConfigDto`) from `src-tauri` into `pwmgr-core`, so
  both binaries import the same types instead of duplicating ~150-300 lines of state/DTO glue.
  Behavior-identical: `src-tauri`'s `commands.rs` switches to importing instead of defining; the full
  `cargo test --workspace` suite (214 tests as of M4) must still pass unchanged.
- `viewer-src-tauri`'s IPC surface: **only** `login`, `lock`, `touch`, `unlock_status`, `list_sites`,
  `list_credentials_for_site`, `search`, `get_config`, `copy_to_clipboard`. Nothing else is registered
  in this binary's `generate_handler!` list — no `register_user`, no add/update/delete, no
  generator/health/backup/portio commands. Read-only is a compile-time fact about which commands
  *exist* in the binary, not a UI-level restriction on an otherwise-capable backend.
- Frontend: a second Vite entry point (`viewer.html` → `src/viewer-main.tsx` → new `ViewerApp.tsx`),
  reusing the existing `src/` React tree rather than a separate frontend project. One `npm run build`
  produces both `index.html` and `viewer.html` in `dist/`; each Tauri binary's `frontendDist` points at
  its own HTML file.
  - **Login**: reuses `Login.tsx` as-is. `onSwitchToRegister` becomes an optional prop (small,
    backward-compatible change — the "sign up" link only renders if the prop is passed); the viewer
    omits it, since creating a new vault is a write action out of scope here.
  - **Vault view**: new, minimal `ViewerVault.tsx` — a single searchable flat list (site name +
    username), built on the existing `search`/`listSites`/`listCredentialsForSite` calls. Clicking a
    row expands it inline (remarks + masked password + a copy button); a separate eye-icon toggle
    reveals the plaintext password. No separate detail screen, no extra navigation state.
  - **Theming**: reads `getConfig()` once on load and applies `theme`/`accentColor` via the same
    3-line `document.documentElement` effect already in `App.tsx` (`App.tsx:23-30`) — duplicated
    verbatim, not extracted into a shared hook, matching this project's established small-duplication
    bar (see M4's `algorithm`-fetch precedent). No Settings screen, no theme/accent toggle in the
    viewer.
  - **Auto-lock**: reuses `useActivityWatcher` and the `vault-locked` event listener, identical to the
    main app — idle timeout (or manual Lock) returns to the login screen and requires the master
    password again.
- Packaging: its own `tauri.conf.json` — distinct `identifier` (e.g. `com.pwmgr.viewer`), distinct
  window title ("Password Manager — Viewer") — producing its own separate installer/bundle target,
  independent of the main app's bundle.

**Out of scope (explicit, so it isn't accidentally built):**
- Register/create a new vault — the viewer only ever logs into an *existing* user from the shared
  registry.
- Trash/soft-deleted credentials — active credentials only.
- Generator, Health Dashboard, Backup/Restore, CSV Import/Export, Settings screen, nav rail — none of
  these exist in the viewer at all, per the "no write commands, minimal surface" scope above.
- A Settings/theme-editing screen in the viewer — it only ever *reads* `config.json`'s current
  theme/accent; it never writes them.
- Any new IPC command not already listed in §2's "IPC surface" bullet — if a screen would need one,
  that screen is out of scope, not the command.

## 3. Architecture — module placement

- **`pwmgr-core`** gains: `AppState` (moved as-is from `src-tauri/src/`) and the DTOs it currently
  owns that the read path needs (`SiteDto`, `CredentialDto`, `SearchResultDto`, `ConfigDto`). This is
  a promotion, not a rewrite — no behavior change, no new logic. `core` remains network-free (an
  invariant already established in M4 specifically because M5 would reuse it as a library — see the
  M4 design doc §4).
- **`src-tauri`** (existing app crate): `commands.rs` updated to `use pwmgr_core::{AppState, SiteDto,
  ...}` instead of defining them locally. No functional change; existing tests continue to exercise
  the same code, just imported from its new home.
- **`viewer-src-tauri`** (new): a thin binary crate mirroring `src-tauri`'s existing `main.rs`/`lib.rs`
  shape (`.manage(app_state)`, `.manage(app_paths)`, `tauri::generate_handler![...]`) but with only the
  9 commands from §2 wired up. Its `commands.rs` re-implements the same thin IPC-wrapper pattern
  `src-tauri` already uses (`do_*` inner function + `#[tauri::command]` wrapper) for exactly those 9
  commands, calling straight into `pwmgr-core`'s `auth`/`session`/`vault`/`registry` modules — the same
  functions `src-tauri` calls today, just from a second binary.
- **Dependencies**: `viewer-src-tauri`'s `Cargo.toml` includes only what these 9 commands need —
  `tauri`, `serde`/`serde_json`, `pwmgr-core`, `rusqlite` (bundled-sqlcipher), `dirs`, `tokio`,
  `zeroize`, `tauri-plugin-clipboard-manager`. It excludes `zxcvbn`, `reqwest`, `sha1`, `csv`,
  `tauri-plugin-dialog` — those belong to generator/health/HIBP/import-export, none of which the
  viewer has.
- **Data location**: both binaries read the same fixed on-disk locations (`~/.pwmgr/registry.db`,
  `~/.pwmgr/vaults/<user>.vault`, `~/.pwmgr/config.json`) via the same path-resolution logic already in
  `core`/`AppPaths` — no new path logic, no viewer-specific data directory.

## 4. Data flow

**Login:** UI form (reused `Login.tsx`) → `login(username, master_password)` → same `auth`/`registry`
verification as the main app → `AppState` holds the derived key in memory, idle timer starts → UI
navigates to `ViewerVault`.

**Browse:** `ViewerVault` calls `listSites()` + `listCredentialsForSite(siteId)` per expanded row
(same calls the main `Vault.tsx` already makes) — the credential's plaintext `secret` arrives over IPC
already, exactly as it does in the main app, and is masked client-side until the eye-icon toggle
reveals it. No new "reveal" IPC call is needed; masking is a frontend-only display state.

**Search:** same `search(query)` command, same `SearchResultDto` shape, filtering as-you-type.

**Copy:** existing `copy_to_clipboard(text)` command, unchanged (including its existing auto-clear
behavior).

**Lock (idle or manual):** identical to the main app — `lock()` closes the SQLCipher handle and
zeroizes the key; the `vault-locked` event returns the UI to `Login`.

## 5. Security notes

- The read-only guarantee is structural: `viewer-src-tauri`'s `generate_handler!` list is the complete
  set of commands the binary can ever receive over IPC. There is no disabled/hidden write command to
  re-enable by mistake — the code implementing `add_credential`, `update_credential`,
  `soft_delete_credential`, `register_user`, etc. simply isn't linked into this binary at all.
- `AppState`/DTO promotion into `core` changes only *where* the types live, not their fields or
  invariants (e.g. `CredentialDto.secret` is still plaintext-once-decrypted, same trust boundary as
  today — the frontend "holds currently-displayed plaintext fields only" principle from
  `ARCHITECTURE.md` §2 is unchanged).
- Auto-lock parity with the main app matters here specifically because this app exists for ad-hoc
  emergency lookups — an unattended unlocked viewer would be a worse exposure window than an
  unattended main app, not a better one.

## 6. Testing

- **Mandatory/automated (Rust):** the `core` promotion refactor must leave all 214 existing tests
  green, unchanged, as a regression gate. `viewer-src-tauri`'s new thin command wrappers get their own
  unit tests mirroring `src-tauri`'s existing convention for the same 9 commands (e.g.
  `login_with_wrong_password_returns_single_generic_error_and_stays_locked`-style coverage), run via
  `cargo test --workspace` once `viewer-src-tauri` is added as a workspace member.
- **Lighter/manual (frontend/visual):** `ViewerVault`'s inline-expand/reveal interaction, theming
  application, and the viewer's separate installer/bundle — all deferred to the single consolidated
  manual walkthrough after M6, per the standing 2026-07-14 decision (see `RESUME_HERE.md`).

## 7. Open items / deferred

None blocking — all decisions above were made interactively during brainstorming. One item to
remember, unrelated to M5 itself but still pending: the M4 follow-up question about whether the
Health Dashboard's "Weak" classification should respect the Settings strength-algorithm toggle instead
of always using zxcvbn — still queued for after M6, tracked in the memory system
(`project_m4_weak_scoring_followup.md`), not part of this spec.
