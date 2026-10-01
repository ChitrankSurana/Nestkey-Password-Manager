# How to Test — Viewer App

**Scope:** the read-only desktop app (`pwmgr-viewer`, crate `viewer-src-tauri`, entry point
`viewer.html` / `ViewerApp.tsx`). It reuses `pwmgr-core` (crypto/vault/registry) as a library but
registers only 8 commands — `unlock_status`, `login`, `lock`, `touch`, `list_sites`,
`list_credentials_for_site`, `search`, `get_config`, `copy_to_clipboard`. There is no
`register_user`, `add_site`, `add_credential`, `update_credential`, `soft_delete_credential`, or any
other mutating command registered in this binary at all — "read-only" is a compiled-in fact, not a
UI restriction.

> **Heads up:** actually launching this app's dev workflow end-to-end was explicitly deferred through
> both M5 and M6 (see `docs/KNOWN_LIMITATIONS.md` §1) — this is the first written procedure for it.
> If the exact commands below need a tweak to work on your machine, that's useful information, not a
> sign you're doing something wrong.

---

## Part 1 — Automated tests

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;C:\Strawberry\perl\bin;C:\Strawberry\c\bin;" + $env:PATH
cargo test --workspace
```

**Expected:** all workspace crates compile and pass, including `pwmgr-viewer` (its own test count is
small/none beyond what it inherits from `pwmgr-core`, which is exercised heavily). There is no
separate `npm test` suite specific to the viewer — the frontend automated tests
(`src/lib/clipboard.test.ts`) exercise shared code used by both entry points.

---

## Part 2 — Launching the Viewer App

The viewer is a **separate Tauri app** living in its own crate (`viewer-src-tauri/`), with its own
`tauri.conf.json` and `Cargo.toml` sitting side by side (the standard Tauri "multiple apps in one
workspace" layout).

> **N16/D-viewer-launch, fixed in `83066b9`:** the config's `beforeDevCommand` used to be
> `npm --prefix .. run dev`, which assumed the Tauri CLI runs it with `cwd = viewer-src-tauri/` -
> the CLI actually always spawns it at the repo root regardless of which crate's `tauri.conf.json`
> you're pointing it at, so `--prefix ..` overshot one directory up (landing on `Account_1` instead
> of `Password_Manager`). It's now plain `npm run dev`, which is already correct when run from the
> repo root. `package.json` also gained `viewer:dev`/`viewer:build` scripts that `cd` into
> `viewer-src-tauri` first, so the Tauri CLI resolves the right crate without you needing to `cd`
> there yourself:

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;C:\Strawberry\perl\bin;C:\Strawberry\c\bin;" + $env:PATH
npm run viewer:dev
```

**Expected:** a Tauri window opens titled "Password Manager — Viewer", showing the Login screen.

The equivalent manual form (if you want to `cd` there yourself, or pass extra CLI flags):
```powershell
cd viewer-src-tauri
npx tauri dev
```
`npx` resolves `tauri` from the repo-root `node_modules/@tauri-apps/cli` (found by walking up from
`viewer-src-tauri/`), so this works without a separate `package.json` in that folder.

If `npx tauri dev` doesn't pick up the right config, the equivalent explicit form is:
```powershell
npx tauri dev --config tauri.conf.json
```
(run from inside `viewer-src-tauri/` either way — the CLI resolves `Cargo.toml`/`tauri.conf.json`
relative to the current directory).

---

## Part 3 — Manual walkthrough

**Prerequisite:** run through
[`HOW_TO_TEST_MAIN_APP.md`](HOW_TO_TEST_MAIN_APP.md) first (or at least its §1–2) so you have a
registered user with at least one site and credential — the Viewer opens the exact same
`~/.pwmgr/registry.db` and `~/.pwmgr/vaults/<user>.vault` files, it doesn't have its own separate
data.

### 1. Login (shared credentials, shared error behavior)

1. **Action:** On the Viewer's Login screen, enter the username/password from your Main App test
   user.
   **Expected:** Logs in successfully, landing on "Vault — `<username>` (read-only)".
2. **Action:** Try a wrong password.
   **Expected:** Same generic "Incorrect master password." message as the Main App — this is the
   same `auth`/`registry` code, just recompiled into a different binary.

### 2. Confirm it's genuinely read-only, not just UI-hidden

1. **Action:** Look at the whole screen — the header, the site rows, each credential row.
   **Expected:** There is no "Add site" button, no "Add credential" button, no Edit/Delete affordance
   anywhere, no Settings/Trash/Health/Generator/Data nav (there's no nav rail at all in this app —
   just the vault list and a Lock button).
2. **Optional deeper check:** open `viewer-src-tauri/src/lib.rs` and confirm the
   `tauri::generate_handler![...]` list contains only the 8 commands named at the top of this doc —
   there is no code path in this binary that could add, edit, or delete anything even if the frontend
   were compromised.

### 3. Browsing and real-time search

1. **Action:** Confirm every site from your test data appears in the list.
2. **Action:** Click a site to expand it.
   **Expected:** Its credentials load (decrypted only for this site, only now — not prefetched for
   the whole list).
3. **Action:** Type into the search box — site name, then a credential's username, then a word from
   its remarks.
   **Expected:** Results update live, same real-time behavior as the Main App's search, backed by the
   same `search` command.

### 4. Reveal / hide / copy a password

1. **Action:** Expand a site, click the 👁 button next to a credential.
   **Expected:** The masked `••••••••` reveals the actual secret; button becomes 🙈; click again to
   re-mask.
2. **Action:** Click **Copy**.
   **Expected:** Copies to clipboard via the same `copy_to_clipboard` command the Main App uses
   (same 30-second auto-clear behavior — wait and confirm the clipboard clears).

### 5. Lock button and auto-lock

1. **Action:** Click **Lock**.
   **Expected:** Returns to Login immediately.
2. **Action:** Log back in, then leave the window idle past your configured auto-lock timeout (this
   reads the same `~/.pwmgr/config.json` your Main App test set — see
   `HOW_TO_TEST_MAIN_APP.md` §7).
   **Expected:** Auto-locks back to Login on its own, same as the Main App, via the same
   `autolock::tick` loop and `vault-locked` event.

### 6. Theme

1. **Action:** In the Main App, change Theme/Accent in Settings, then relaunch the Viewer (or if
   already open, re-login).
   **Expected:** Viewer reflects the same theme/accent — it reads the same `config.json`, it just has
   no Settings screen of its own to change it from.

---

## If something doesn't match

Note exactly: which step, what you did, what you expected, what you actually saw. Since this is the
first time this app's dev launch has been exercised end-to-end, also note anything about the launch
process itself that didn't match Part 2 above (wrong window, build error, wrong port) — that's
valuable to capture regardless of whether the feature behavior itself is correct.

## If everything matches

The Viewer App's manual walkthrough is complete. Continue to
[`HOW_TO_TEST_WEB_PORTAL.md`](HOW_TO_TEST_WEB_PORTAL.md).
