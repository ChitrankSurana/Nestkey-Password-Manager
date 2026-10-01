# Architecture

**Last updated:** 2026-09-30

## 1. Overview

```
+---------------------------------------------------------------+
|  Frontend  (React + TypeScript + Tailwind, runs in webview)   |
|                                                                 |
|  Three UI variants (config.ui_variant): Classic · New · Make   |
|  (src/, src/newui, src/makeui), plus ViewerApp for the viewer. |
|  Screens: Login/Register · Vault (folders, item types, search) |
|  Entry editor · Generator · Health · Settings · Data Hub       |
|  (backup, CSV/JSON import-export) · Trash                      |
|                                                                 |
|  Holds: UI state, currently-displayed plaintext fields only.   |
|  Never holds: master password beyond the login form submit,    |
|  the derived key, or any other user's data.                    |
+----------------------------- Tauri IPC ------------------------+
|  Rust Core (src-tauri) — all secrets and all business logic    |
|                                                                 |
|   crypto      Argon2id KDF · AES-256-GCM seal/open · zeroize   |
|   registry    unencrypted username/salt lookup (no secrets)    |
|   auth        register/login/verify                            |
|   session     in-memory key holder · idle timer · lock state   |
|   vault       SQLCipher CRUD · search · soft-delete/trash       |
|   generator   random / memorable / pattern / pronounceable      |
|   health      entropy · reuse · age · HIBP (opt-in)             |
|   backup      encrypted backup file · restore                  |
|   portio      CSV import (Chromium/Firefox/mapper) + export     |
|   jsonio      full-fidelity JSON export/import (sealed envelope)|
|   totp        TOTP code generation                              |
|   biometric   Windows Hello unlock (desktop)                    |
|   ssh_agent   SSH agent over named pipe; ssh_keys key storage   |
|   hibp        opt-in breach check (k-anonymity)                 |
|   autolock · config · paths · dto · state                       |
|   commands    shared do_* command logic (IPC/HTTP agnostic)     |
+---------------------------------------------------------------+
|  Disk                                                          |
|   ~/.pwmgr/registry.db          (unencrypted: usernames+salts) |
|   ~/.pwmgr/vaults/<user>.vault  (SQLCipher-encrypted)           |
|   ~/.pwmgr/backups/*.bak        (encrypted backup files)        |
|   ~/.pwmgr/config.json          (theme, auto-lock minutes, etc) |
+---------------------------------------------------------------+
```

## 2. Why this shape

- **Secrets never cross the IPC boundary in derived form.** The frontend sends the master password
  once (login/register submit) and from then on only issues intent commands ("add this credential",
  "search for X"); the Rust core does all decryption and returns only the plaintext needed to render
  the current screen.
- **The registry DB is deliberately unencrypted but secret-free** — it stores only usernames and KDF
  salts, so the login screen can list existing users and route to the right vault file without ever
  touching a password.
- **SQLCipher + per-field AES-GCM is defense in depth**, not redundancy for its own sake: SQLCipher
  protects the file as a whole (so table/schema/metadata aren't visible either), while per-field GCM
  means a single record is independently tamper-evident and a bug in one layer doesn't fully compromise
  the other.
- **Read-only viewer (M5) reuses `crypto` + `vault` + `registry`** as a library crate, compiled into a
  second, smaller Tauri binary with no write-path commands registered — so "read-only" is enforced by
  what code exists in the binary, not just by UI restrictions.
- **Web mode (M6) reuses the same Rust core**, adding an HTTP layer (`web-src-tauri`, Axum) bound to
  `127.0.0.1` only, with a session-token middleware in front of the same `commands` module. Phase 9
  hardened it: no client-supplied filesystem paths (export is a download body; CSV import and backup
  restore are raw-body uploads cached per session), a login/register/PIN throttle with doubling
  lockout, a bounded Argon2 semaphore, and a strict CSP plus security headers.
- **Four crates, one core.** `pwmgr-core` (`core/`) is shared by `src-tauri` (desktop app),
  `viewer-src-tauri` (read-only), and `web-src-tauri` (web portal). Each feature lands in the order
  core → desktop wrapper → web wrapper/route → frontend API client + UI.
- **Reprompt gate.** Secrets are gated by the master password by default; the per-credential
  `reprompt_exempt` column opts one item out (the retired `reprompt` column is unused).
- **History vs. change log.** Password history (`credential_history`) stays password-only; the
  per-credential change log (`credential_change_log` table, written from `core/src/vault.rs`)
  records general edits (username, fields, etc.).

## 3. Key Data Flows

**Register:** UI form → `register_user(username, master_password)` → `auth` generates salts, derives
key via `crypto`, creates sentinel, calls `vault` to init a new SQLCipher file, writes salts to
`registry` → returns success (never returns key/password) → `session` starts unlocked.

**Login:** UI form → `login(username, master_password)` → `registry` lookup → `auth` verifies →
`session` holds key in memory, starts idle timer → UI navigates to vault list (fetched via
`list_sites`, which returns decrypted-for-display fields only).

**Add credential:** UI form → `add_credential(site_id, username, secret, remarks)` → `session`
confirms unlocked → `crypto` seals `secret` with the in-memory key → `vault` inserts row in a
transaction → UI refreshes list.

**Lock (idle or manual):** `session` timer fires (or user clicks Lock) → close SQLCipher handle →
`zeroize` the key buffer → UI forced to login screen; re-entry requires the master password again.

**CSV import:** UI selects file + source format → `portio` parses to a normalized in-memory record
list → UI preview/mapping screen → user confirms → `portio` writes via `vault` in one transaction
(rollback on any row failure) → summary shown (imported/skipped/duplicates).

## 4. Module Boundaries (isolation & testability)

Each Rust module in §5 of the TRD is a separate file/sub-crate with a narrow public API:
- `crypto` has **no knowledge of SQLite** — pure functions over bytes, easiest to unit test exhaustively.
- `vault` has **no knowledge of Argon2/passwords** — it's handed an already-derived key.
- `auth`/`session` are the only modules that ever see the master password, and only transiently.
- `commands` is a thin translation layer (IPC ⇄ core calls); it contains no business logic itself, so
  bugs are easy to localize to the module that owns the behavior.

This means: crypto can be fuzz/property-tested with no DB involved; vault logic can be tested against
a pre-derived test key with no Argon2 cost; UI can be developed against mocked IPC commands without a
real vault.
