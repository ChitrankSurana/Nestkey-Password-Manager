# Constraints (from SPECs)

SPEC sources in this ingest: `docs/TRD.md`, `docs/THREAT_MODEL.md`,
`docs/superpowers/specs/2026-07-14-m4-health-nav-theming-design.md`,
`docs/superpowers/plans/2026-07-14-m4-health-nav-theming.md` (classified SPEC despite living in
`plans/`),
`docs/superpowers/specs/2026-07-16-m5-readonly-viewer-design.md`,
`docs/superpowers/specs/2026-07-18-m6-web-mode-design.md`,
`docs/superpowers/specs/2026-07-19-m6-responsive-layout-design.md`.

**Flagged conflict:** the M4 design spec and the M4 plan (also SPEC-typed) disagree on where
weak/old-password classification logic lives and whether "weak" honors the `strength_algorithm`
Settings toggle. Both entries are preserved below verbatim per source; see `INGEST-CONFLICTS.md`
WARNING "M4 SPEC vs SPEC: health-classification module placement and algorithm scope."

---

## Technology Stack

- source: docs/TRD.md
- type: nfr
- content: App shell Tauri 2.x (Rust core + native webview). Core logic in Rust (crypto, database,
  business logic — all secrets live here). Frontend React 18 + TypeScript + Vite + Tailwind CSS (UI
  only, no plaintext secret storage in JS memory beyond what's on screen). Database SQLite via
  SQLCipher (`rusqlite` + sqlcipher feature) — one `.vault` file per user. KDF: Argon2id (`argon2`
  crate). Symmetric encryption: AES-256-GCM (`aes-gcm` crate). Secure memory: `zeroize` crate.
  Breach check: HIBP `range` API (k-anonymity), opt-in — the only network call in the whole app.

## Non-Functional Requirements (NFR-1..6)

- source: docs/TRD.md
- type: nfr
- content: NFR-1 Local-only by default — zero network calls unless the user explicitly opts into
  HIBP. NFR-2 Zero-knowledge — master password and derived key never written to disk/logs/crash
  reports/telemetry (no telemetry exists). NFR-3 Performance — vault unlock ≤1s on typical hardware;
  search updates within one UI frame of keystroke for vaults up to ~5,000 entries. NFR-4 Resilience
  — corrupted/partial write must not silently lose data; use SQLite transactions for every mutating
  operation. NFR-5 Portability — core Rust modules must not assume Windows-only APIs. NFR-6
  Accessibility — all interactive controls keyboard-navigable; tooltips available on keyboard focus,
  not just mouse hover.

## Cryptographic Design — Registration & Login

- source: docs/TRD.md §3.1–3.2
- type: protocol
- content: Registration generates random `kdf_salt`/`verifier_salt` (16 bytes each);
  `vault_key = Argon2id(master_password, kdf_salt)`; `verifier_hash = Argon2id(master_password,
  verifier_salt)` stored for cheap login checks; a random 32-byte `sentinel` is encrypted with
  `vault_key` via AES-256-GCM and stored to later confirm a derived key is correct. Login
  recomputes `Argon2id(input_password, verifier_salt)`, compares constant-time to `verifier_hash`;
  on match derives `vault_key`, opens the SQLCipher DB, confirms by decrypting `encrypted_sentinel`
  (GCM auth failure ⇒ reject). Any failure returns one generic "incorrect master password" message
  with no distinguishing information.

## Cryptographic Design — Record Encryption & Session

- source: docs/TRD.md §3.3–3.5
- type: protocol
- content: SQLCipher encrypts the entire DB file at rest (page-level AES-256). Additionally,
  `credentials.secret` is sealed with AES-256-GCM using `vault_key` and a fresh random 96-bit nonce
  per write (defense in depth). Session: idle timer default 5 min, user-configurable 1/5/15 min; any
  UI interaction resets it; on expiry the SQLCipher handle closes and `vault_key`/master-password
  buffers are zeroized. M6 web mode issues a cryptographically random 256-bit session token per
  unlocked session, required on every IPC-over-HTTP call, HttpOnly cookie; server refuses to bind to
  any interface other than `127.0.0.1`. Argon2id starting params: 64 MiB memory, 3 iterations,
  parallelism 4 (OWASP baseline, to be benchmarked/tuned for ≤1s unlock).

## Data Model (SQL Schema)

- source: docs/TRD.md §4
- type: schema
- content: Unencrypted "registry" DB holds `users(id, username UNIQUE, kdf_salt, verifier_salt,
  verifier_hash, encrypted_sentinel, sentinel_nonce, created_at)` — usernames + KDF params only, no
  secrets, separate from each user's own SQLCipher vault. Inside each user's `<username>.vault`
  (SQLCipher-encrypted): `sites(id, name, url, notes, created_at, updated_at, deleted_at)` and
  `credentials(id, site_id REFERENCES sites(id), username, secret_enc, secret_nonce, remarks,
  strength_score, password_updated_at, created_at, updated_at, deleted_at)`, plus indexes on
  `sites(name)` and `credentials(site_id)`.

## IPC Contract (Tauri Commands, Baseline)

- source: docs/TRD.md §6
- type: api-contract
- content: `register_user`, `login`, `lock`, `unlock_status`, `list_sites`, `search`, `add_site`,
  `add_credential`, `update_credential`, `soft_delete_credential`, `restore_credential`,
  `purge_trash`, `generate_password`, `analyze_strength`, `health_report`, `check_breach` (opt-in),
  `create_backup`, `restore_backup`, `export_csv`, `import_csv_preview`, `import_csv_commit`. Every
  command touching vault data requires an active unlocked session; the frontend never passes/receives
  raw key material — only opaque session state and plaintext record fields needed for display.

## Testing Requirements

- source: docs/TRD.md §7
- type: nfr
- content: Mandatory/automated: crypto round-trip, wrong-password rejection, tamper detection (GCM
  auth failure), zeroization (best-effort verification), CRUD correctness, search correctness, CSV
  round-trip fidelity. Lighter/manual acceptable: UI/theme visuals, tooltip presence (checklist),
  responsive layout.

## Threat Model — Assets, Adversaries, Scope

- source: docs/THREAT_MODEL.md §1–3
- type: nfr
- content: Assets: master password (never stored), derived vault keys (in-memory only), stored
  credentials at rest, backup files, transient plaintext CSV exports. In-scope adversaries: A1
  stolen/powered-off device; A2 another local OS user, no admin rights; A3 casual file inspection
  (hex/sqlite3 CLI); A4 network eavesdropper/CSRF attacker against web mode; A5 tampering at rest.
  Explicitly out of scope: O1 malware/keylogger as the same OS user; O2 physical coercion/shoulder-
  surfing; O3 compromised OS/firmware; O4 supply-chain compromise of a dependency (mitigated only
  informally).

## Threat Model — Mitigations

- source: docs/THREAT_MODEL.md §4
- type: protocol
- content: File theft/inspection (A1/A3): SQLCipher full-DB encryption + per-field AES-256-GCM,
  Argon2id cost. Weak master password: real-time strength/entropy feedback at registration, a
  minimum-entropy warning shown but not a hard block. Shared machine (A2): per-user vault files with
  OS-level permissions. Web-mode eavesdropping (A4): server binds `127.0.0.1` only, hard-coded, not
  configurable; 256-bit CSPRNG session token required per request. Web-mode CSRF (A4): session
  cookie `HttpOnly; SameSite=Strict; Path=/`, plus every mutating (non-GET) request must carry a
  custom `x-pwmgr-csrf` header. Tampering (A5): AES-GCM authenticated, any bit-flip fails loudly.
  Backup files: encrypted with the same scheme as the live vault. Plaintext CSV export: gated behind
  explicit typed confirmation, export path logged locally. HIBP check: k-anonymity (first 5 hex
  chars of SHA-1 only), off by default, explicit opt-in. Wrong-password oracle: single generic
  failure message. Key lingering in memory: `zeroize` on lock/logout/exit, configurable auto-lock
  (default 5 min).

## Threat Model — Residual Risks

- source: docs/THREAT_MODEL.md §5
- type: nfr
- content: O1 malware-with-memory-access during the unlocked window exposes in-use credentials —
  accepted, true of every password manager. Argon2id params trade off unlock speed vs. brute-force
  cost; not stopped forever, only slowed. Viewer and web mode reuse the same crypto core, so a
  vulnerability there affects all surfaces equally. Web mode's session store holds N vault + N
  registry connections open per concurrent browser session — accepted at household scale, not
  load-tested beyond that. Session store is in-memory only by design; a server crash/restart
  silently logs out every active browser session (intentional, documented availability property).

## M4 Design — Health Dashboard Classification (design spec version)

- source: docs/superpowers/specs/2026-07-14-m4-health-nav-theming-design.md §3–4
- type: protocol
- content: "Weak" reuses the M2 strength scoring (`do_analyze_strength`), **"respecting the user's
  `strength_algorithm` setting."** "Reused" computed on demand (decrypt every credential's secret in
  memory, group by exact plaintext match, group of 2+ is reused) — no stored fingerprint. "Old" is
  `password_updated_at` older than a configurable `old_password_days` threshold (default 365).
  `core::health` (new, `core/src/health.rs`) is specified to own weak/reused/old classification as
  pure functions, plus the k-anonymity match logic; no I/O, no new `core` dependency. **See
  INGEST-CONFLICTS.md — this module-placement and algorithm-scope statement is contradicted by the
  M4 plan entry below.**

## M4 Design — HIBP Opt-in Gate & Config Fields

- source: docs/superpowers/specs/2026-07-14-m4-health-nav-theming-design.md §3–4
- type: protocol
- content: "Breached" is opt-in via new `hibp_opt_in` config bool (default false — NFR-1 holds).
  Once enabled, a "Check now" button triggers the check; nothing is sent over the network until that
  specific click, every time. `check_breach` itself refuses to run unless `hibp_opt_in` is true,
  independent of what the frontend sends. `reqwest`/`sha1` are app-crate-scoped only (never in
  `core`); the HTTP call's base URL must be injectable for testing without live network access.
  `config.json` gains: `accent_color` (5 valid values), `old_password_days` (validated enum, default
  365), `hibp_opt_in` (bool, default false), `nav_collapsed` (bool, default false).

## M4 Design — Navigation Rail & Theming

- source: docs/superpowers/specs/2026-07-14-m4-health-nav-theming-design.md §5–6
- type: protocol
- content: Nav rail destinations: Vault, Health, Generator, Trash, Data (hub), Settings, plus Lock
  pinned at bottom. Collapsible (icon+label ⇄ icon-only, ~48px collapsed), toggle pinned above the
  list, flyout tooltip on hover when collapsed, state persisted in `nav_collapsed`, starts expanded.
  Theming: `config.theme` drives `<html>`'s `dark` class reactively; 5-value `accent_color`
  (Indigo/Sky/Teal/Purple/Pink) drives a `data-accent` attribute and CSS custom properties; ~11
  hardcoded `bg-blue-*`/`text-blue-*` occurrences across 9 screen files swap to reference the accent
  variable.

## M4 Plan — Health Module Actual Placement (deviates from design spec)

- source: docs/superpowers/plans/2026-07-14-m4-health-nav-theming.md (lines 13–34, Task 7 notes at
  lines 917–934)
- type: protocol
- content: **Only `classify_reused` lives in `core::health`.** "Old" password detection is instead a
  SQL query in `pwmgr-core::vault` (SQLite's own `datetime('now', '-N days')`, no date-parsing
  crate). "Weak" classification is inline in the app-crate IPC command (`do_health_report`), reusing
  `zxcvbn` — and **explicitly does NOT honor the `strength_algorithm` Settings toggle**: "the design
  doc said weak detection should 'respect the user's `strength_algorithm` setting,' but the 'simple'
  entropy algorithm only exists in the frontend... Porting a second copy of the same formula into
  Rust... isn't worth the duplication. This command always classifies 'weak' using zxcvbn only." The
  plan explicitly frames this as a "second judgment call," self-flagged as a deviation from the
  design spec, not an oversight. **This is corroborated as actual shipped behavior by
  `docs/KNOWN_LIMITATIONS.md` §2** ("The Health Dashboard's 'Weak' classification always scores with
  zxcvbn, regardless of what you've picked in Settings... confirmed intentional after review").

## M5 Design — Viewer IPC Surface

- source: docs/superpowers/specs/2026-07-16-m5-readonly-viewer-design.md §2
- type: api-contract
- content: `viewer-src-tauri`'s IPC surface is **only**: `login`, `lock`, `touch`, `unlock_status`,
  `list_sites`, `list_credentials_for_site`, `search`, `get_config`, `copy_to_clipboard`. No other
  command is registered in this binary's `generate_handler!` list — no `register_user`, no
  add/update/delete, no generator/health/backup/portio commands. Read-only is a compile-time fact
  about which commands exist in the binary.

## M5 Design — Architecture / Module Placement

- source: docs/superpowers/specs/2026-07-16-m5-readonly-viewer-design.md §3
- type: protocol
- content: `pwmgr-core` gains `AppState` and the read-path DTOs (`SiteDto`, `CredentialDto`,
  `SearchResultDto`, `ConfigDto`), promoted as-is from `src-tauri` (no behavior change). New binary
  crate `viewer-src-tauri` depends only on `pwmgr-core`, never on `src-tauri`. `core` remains
  network-free — an invariant established specifically because M5 reuses it as a library.

## M5 Design — Security Notes

- source: docs/superpowers/specs/2026-07-16-m5-readonly-viewer-design.md §5
- type: nfr
- content: The read-only guarantee is structural — no disabled/hidden write command exists to
  re-enable by mistake; the code implementing write commands simply isn't linked into this binary.
  Auto-lock parity with the main app is required since an unattended unlocked viewer would be a
  worse exposure window than an unattended main app.

## M6 Web-Mode Design — Command Parity & Server Shape

- source: docs/superpowers/specs/2026-07-18-m6-web-mode-design.md §2–3
- type: api-contract
- content: New binary crate `web-src-tauri` (package `pwmgr-web`) — pure Rust HTTP server, no Tauri
  dependency, no GUI window; depends on `pwmgr-core`, `axum`, `tower`, `tower-http`, `tokio` (full
  runtime). Full command parity with the desktop app — every command in
  `src-tauri/src/commands.rs` becomes reachable over HTTP, not a curated subset (requires promoting
  the remaining ~25 `do_*` functions into `pwmgr-core`). All three front-ends consume one shared copy
  of every command.

## M6 Web-Mode Design — Multi-Session & Auth

- source: docs/superpowers/specs/2026-07-18-m6-web-mode-design.md §2–3
- type: protocol
- content: Multi-session support scoped to **one shared OS user account with multiple app-level
  named vaults** — not isolation between separate OS user accounts (127.0.0.1 TCP binding does not
  provide that; explicitly out of scope). Session-token auth: 256-bit random token per login,
  HttpOnly + `SameSite=Strict` cookie, required on every request after login. Background sweep
  (2-second cadence) locks sessions past idle timeout; locked entries persist in the map, pruned
  after 24h of inactivity. Session store: `Arc<RwLock<HashMap<Token, UserSession>>>`, each
  `UserSession` bundling its own vault connection and lock-timer.

## M6 Web-Mode Design — CSRF & Bind Address

- source: docs/superpowers/specs/2026-07-18-m6-web-mode-design.md §2, §5
- type: nfr
- content: CSRF mitigation: `SameSite=Strict` cookie + a required custom header on all mutating
  (non-GET) requests; no CORS headers configured at all (web UI served same-origin). Binds to
  `127.0.0.1` only, hard-coded at the listener level — not a config option. Port number is an
  explicit open item in this spec, left for implementation planning to pin down (see
  `context.md` — the resolved value, 7373, is recorded in `docs/HOW_TO_USE_WEB_PORTAL.md`, a DOC
  source, not this SPEC).

## M6 Web-Mode Design — HTTP Status Mapping & Clipboard

- source: docs/superpowers/specs/2026-07-18-m6-web-mode-design.md §2
- type: api-contract
- content: `CommandError`'s `code()`/`message()` maps to HTTP status: `locked`→401,
  `invalid_credentials`→401, `invalid_input`→400, `not_found`→404, `internal_error`→500, etc.
  `copy_to_clipboard` is not exposed as an HTTP endpoint — the browser uses
  `navigator.clipboard.writeText` with a JS-side 30-second auto-clear re-implementing the existing
  Rust-side timing property.

## M6 Responsive-Layout Design — Breakpoint

- source: docs/superpowers/specs/2026-07-19-m6-responsive-layout-design.md §3
- type: nfr
- content: Single breakpoint at 640px, matching Tailwind v4's built-in `sm` breakpoint (no custom
  value). `≥640px` = today's layout, byte-for-byte unchanged. `<640px` = narrow-layout rules apply.
  Chosen over Tailwind's `md` (768px) so tablets in landscape and most laptop windows keep the
  existing sidebar.

## M6 Responsive-Layout Design — Navigation Patterns & Config

- source: docs/superpowers/specs/2026-07-19-m6-responsive-layout-design.md §4
- type: protocol
- content: Below 640px, the sidebar is replaced by one of three selectable narrow-nav patterns: icon
  rail (default), bottom tab bar, or hamburger + slide-over drawer. Selectable via a new `Config`
  field `mobileNavStyle`, validated against `["icon-rail", "bottom-tabs", "drawer"]`, default
  `icon-rail`, exposed as both a Tauri command and an HTTP route
  (`do_update_mobile_nav_style`/`ConfigDto.mobileNavStyle`). Has no effect at ≥640px.

## M6 Responsive-Layout Design — Modals & Content Screens

- source: docs/superpowers/specs/2026-07-19-m6-responsive-layout-design.md §5–6
- type: nfr
- content: Every editor dialog keeps today's dim-backdrop-plus-centered-card pattern at every width;
  below 640px the card becomes fluid-width with side margins instead of a fixed pixel width. All
  remaining content screens get a straightforward responsive sweep below 640px (fluid/stacking
  layouts, touch-reasonable interactive-element sizing). No new "full-screen sheet" concept
  introduced.
