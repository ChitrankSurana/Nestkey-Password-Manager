# Constraints (from SPEC-type documents)

7 documents in this ingest were classified `type: SPEC`: `docs/TRD.md`, `docs/THREAT_MODEL.md`,
`docs/superpowers/specs/2026-07-14-m4-health-nav-theming-design.md`,
`docs/superpowers/plans/2026-07-14-m4-health-nav-theming.md` (an implementation plan, classified
SPEC by the upstream classifier rather than DOC — its content is treated as constraint-bearing per
that classification),
`docs/superpowers/specs/2026-07-16-m5-readonly-viewer-design.md`,
`docs/superpowers/specs/2026-07-18-m6-web-mode-design.md`, and
`docs/superpowers/specs/2026-07-19-m6-responsive-layout-design.md`. All carry `locked: false` — none
are LOCKED, so none can hard-block another SPEC; see `INGEST-CONFLICTS.md` for the one WARNING-level
competing-variant found between two of these.

---

## TRD — Non-Functional Requirements
- source: docs/TRD.md
- type: nfr
- content: NFR-1 Local-only by default (zero network calls unless the user explicitly opts into the
  HIBP breach check). NFR-2 Zero-knowledge (master password/derived key never written to disk, logs,
  crash reports, or telemetry — there is no telemetry). NFR-3 Performance (vault unlock ≤ 1s on
  typical hardware; search results update within one UI frame of keystroke for vaults up to ~5,000
  entries). NFR-4 Resilience (a corrupted/partial write must not silently lose data — SQLite
  transactions for every mutating operation). NFR-5 Portability (core Rust modules must not assume
  Windows-only APIs, so later macOS/Linux ports and the Android door stay open). NFR-6 Accessibility
  (all interactive controls keyboard-navigable; tooltips available on keyboard focus, not just mouse
  hover).

## TRD — Cryptographic Design (registration / login / session)
- source: docs/TRD.md
- type: protocol
- content: Registration: random `kdf_salt`(16B) + `verifier_salt`(16B); `vault_key =
  Argon2id(master_password, kdf_salt)`; `verifier_hash = Argon2id(master_password, verifier_salt)`
  stored for cheap login pre-check; a random 32-byte `sentinel` is AES-256-GCM-encrypted under
  `vault_key` and stored to later confirm a derived key is correct; SQLCipher DB keyed with
  `vault_key`. Login: recompute verifier hash, constant-time compare; on match derive `vault_key`,
  open SQLCipher DB, confirm by decrypting `encrypted_sentinel` (GCM auth failure ⇒ reject). Any
  failure returns one generic "incorrect master password" message. Record encryption: SQLCipher
  full-DB page-level AES-256 plus field-level AES-256-GCM on `credentials.secret` with a fresh random
  96-bit nonce per write (defense in depth). Session/auto-lock: idle timer default 5 min
  (user-configurable 1/5/15 min), any UI interaction resets it, expiry closes the SQLCipher handle
  and zeroizes `vault_key`/`master_password`. M6 web mode: 256-bit CSPRNG session token bound to the
  unlocked session, required on every IPC-over-HTTP call, HttpOnly cookie, server refuses to bind to
  any interface other than `127.0.0.1`. Argon2id starting params: 64 MiB memory / 3 iterations / 4
  parallelism (OWASP-recommended baseline, to be benchmarked/tuned for ≤1s unlock).

## TRD — Data Model (SQL schema)
- source: docs/TRD.md
- type: schema
- content: `users` table lives in a small unencrypted "registry" DB (usernames + KDF salts only, no
  secrets) — separate from each user's own SQLCipher `<username>.vault`, so the login screen can list
  users and locate the right vault file without decrypting anything. Inside each SQLCipher vault:
  `sites(id, name, url, notes, created_at, updated_at, deleted_at)` and
  `credentials(id, site_id REFERENCES sites(id), username, secret_enc, secret_nonce, remarks,
  strength_score, password_updated_at, created_at, updated_at, deleted_at)`, plus
  `idx_sites_name`/`idx_credentials_site` indexes.

## TRD — IPC Contract
- source: docs/TRD.md
- type: api-contract
- content: High-level Tauri command surface: `register_user`, `login`, `lock`, `unlock_status`,
  `list_sites`, `search`, `add_site`, `add_credential`, `update_credential`,
  `soft_delete_credential`, `restore_credential`, `purge_trash`, `generate_password`,
  `analyze_strength`, `health_report`, `check_breach` (opt-in), `create_backup`, `restore_backup`,
  `export_csv`, `import_csv_preview`, `import_csv_commit`. Every command that touches vault data
  requires an active unlocked session; the frontend never passes or receives raw key material — only
  opaque session state and plaintext record fields needed for display.

## TRD — Testing Requirements Bar
- source: docs/TRD.md
- type: nfr
- content: Mandatory, automated: crypto round-trip, wrong-password rejection, tamper detection (GCM
  auth failure), zeroization (best-effort verification), CRUD correctness, search correctness, CSV
  round-trip fidelity. Lighter/manual acceptable: UI/theme visuals, tooltip presence (checklist),
  responsive layout.

## Threat Model — Assets, Adversaries In-Scope, and Explicit Exclusions
- source: docs/THREAT_MODEL.md
- type: nfr
- content: Assets: master password (never stored), derived vault keys (in-memory only while
  unlocked), stored credentials at rest, backup files, plaintext CSV exports (transient). In-scope
  adversaries: A1 stolen/lost powered-off device, A2 another local OS user on a shared machine
  (no admin rights), A3 casual file inspection (hex editor / plain `sqlite3` CLI), A4 network
  eavesdropper/cross-site attacker against the localhost-bound web server (M6), A5 tampering at rest.
  Explicitly out of scope: O1 malware/keylogger running as the same OS user while unlocked (mitigated
  only by minimizing the unlock window, not eliminated), O2 physical coercion/shoulder-surfing, O3
  compromised OS/firmware, O4 supply-chain compromise (mitigated only informally — dependency
  checker, pinned versions, minimal dependency set; no SBOM/reproducible-build program in v1).

## Threat Model — Mitigations by Threat
- source: docs/THREAT_MODEL.md
- type: nfr
- content: A1/A3 file theft/inspection → SQLCipher full-DB + per-field AES-256-GCM, Argon2id makes
  offline brute-force expensive. A1 weak master password → real-time strength/entropy feedback at
  registration is a nudge (minimum-entropy warning shown, not a hard block). A2 shared machine →
  per-user vault files with OS-level file permissions. A4 web-mode eavesdropping → server binds to
  `127.0.0.1` only, hard-coded, not configurable; 256-bit CSPRNG session token required per request.
  A4 web-mode CSRF → session cookie is `HttpOnly; SameSite=Strict; Path=/`, and every mutating
  (non-GET) request additionally requires a custom `x-pwmgr-csrf` header (second, independent layer
  behind the cookie attribute). A5 tampering → AES-GCM authentication causes any bit-flip in
  ciphertext/nonce to fail decryption loudly. Backup file theft → backups encrypted with the same
  scheme as the live vault, never plaintext. Plaintext CSV export → gated behind an explicit typed
  confirmation, export path logged in-app locally. HIBP breach check → k-anonymity (only first 5 hex
  chars of SHA-1(password) sent), off by default, explicit opt-in. Wrong-password oracle → one
  generic failure message regardless of which part was wrong. Key lingering in memory → `zeroize` on
  lock/logout/exit; auto-lock after configurable idle period (default 5 min).

## Threat Model — Residual Risks & Verification Requirement
- source: docs/THREAT_MODEL.md
- type: nfr
- content: Residual risks accepted: O1 exposure during the unlocked window is inherent to any local
  password manager; Argon2id params trade off unlock speed vs. brute-force cost (v1 targets
  OWASP-baseline, ≤1s unlock — slows, doesn't stop, a determined offline attacker); the read-only
  viewer and web mode share the same `crypto`/`vault` core, so a vulnerability there affects all
  surfaces equally; Web Portal's per-session state is accepted at "household scale," not
  load-tested beyond it; the session store is in-memory only by design — a server crash/restart
  silently logs out every active browser session (401 on next request), and nothing sensitive is
  lost because nothing sensitive was ever only in that store. Verification requirement (§6): each
  mitigation above should have a corresponding automated test or manual check before the relevant
  milestone is considered done.

## M4 Design — Health Classification Rules (weak / reused / old / breached)
- source: docs/superpowers/specs/2026-07-14-m4-health-nav-theming-design.md
- type: protocol
- content: Weak = strength score 0 or 1 ("Very weak"/"Weak"), reusing M2's `do_analyze_strength`
  and **"respecting the user's `strength_algorithm` setting."** Reused = computed on demand (decrypt
  every credential's secret in memory, group by exact plaintext match, any group of 2+ is reused) —
  explicitly not a stored HMAC fingerprint. Old = `password_updated_at` older than a configurable
  `old_password_days` threshold (default 365); requires a prerequisite fix so
  `vault::update_credential` only bumps `password_updated_at` when the secret itself actually
  changes. Breached = opt-in via `hibp_opt_in` (default false); dashboard shows a "Check now" button,
  nothing sent over the network until that specific click, every time; `check_breach`'s backend
  itself refuses to run unless `hibp_opt_in` is true, independent of frontend state; identical
  passwords are deduped before hitting HIBP.
  **See `INGEST-CONFLICTS.md` → [WARNING] for a documented deviation from the "respecting
  strength_algorithm" clause of this entry.**

## M4 Design — config.json fields & new IPC commands
- source: docs/superpowers/specs/2026-07-14-m4-health-nav-theming-design.md
- type: api-contract
- content: `config.json` gains `accent_color` (5 valid values), `old_password_days` (validated enum,
  default 365), `hibp_opt_in` (bool, default false), `nav_collapsed` (bool, default false). New IPC
  commands: `health_report()`, `check_breach()`, `update_old_password_days(days)`,
  `update_hibp_opt_in(enabled)`, `update_theme(theme)`, `update_accent_color(color)`,
  `update_nav_collapsed(collapsed)`.

## M4 Design/Plan — `core::health` stays network-free (architecture invariant)
- source: docs/superpowers/specs/2026-07-14-m4-health-nav-theming-design.md;
  docs/superpowers/plans/2026-07-14-m4-health-nav-theming.md
- type: nfr
- content: `core::health` is pure functions only (weak/reused/old classification, k-anonymity match
  logic), no I/O, no new dependency for `core` itself. The actual HIBP HTTP call and SHA-1 hashing
  live in the app crate only (`reqwest`, `sha1`, app-crate-scoped). Enforcing "no network code in
  `core`" is explicitly justified because `core` is reused as a library by M5's read-only viewer,
  which must never make network calls — same invariant class as "no serde codegen in `core`" kept
  since M3. The HIBP HTTP call's base URL must be injectable so Rust tests can point it at a local
  mock instead of the real endpoint.

## M4 Plan — Weak-classification implementation deviation (flagged by the plan itself)
- source: docs/superpowers/plans/2026-07-14-m4-health-nav-theming.md
- type: protocol
- content: Judgment call documented in Task 7: "the design doc said weak detection should 'respect
  the user's `strength_algorithm` setting,' but the 'simple' entropy algorithm only exists in the
  frontend (`src/lib/strength.ts`) — there is no Rust port of it... This command always classifies
  'weak' using zxcvbn, regardless of the user's Settings toggle — that toggle only affects the live
  meter in CredentialEditor/Generator." Flagged in the plan as a deliberate deviation from the design
  spec, not an oversight. **See `INGEST-CONFLICTS.md` → [WARNING] — this directly contradicts the
  M4 Design entry above on the same scope.**

## M5 Design — Viewer IPC surface (read-only, compile-time enforced)
- source: docs/superpowers/specs/2026-07-16-m5-readonly-viewer-design.md
- type: api-contract
- content: `viewer-src-tauri`'s `generate_handler!` list registers **only**: `login`, `lock`,
  `touch`, `unlock_status`, `list_sites`, `list_credentials_for_site`, `search`, `get_config`,
  `copy_to_clipboard`. No `register_user`, no add/update/delete, no generator/health/backup/portio
  commands are registered in this binary at all — "read-only" is a compile-time fact about which
  commands exist in the binary, not a UI-level restriction on an otherwise-capable backend.

## M5 Design — Architecture (core promotion, structural read-only guarantee)
- source: docs/superpowers/specs/2026-07-16-m5-readonly-viewer-design.md
- type: nfr
- content: `AppState` and the read-path DTOs (`SiteDto`, `CredentialDto`, `SearchResultDto`,
  `ConfigDto`) are promoted from `src-tauri` into `pwmgr-core` so both binaries import the same types
  (promotion, not rewrite — behavior-identical, full `cargo test --workspace` suite must stay green).
  `viewer-src-tauri` depends only on `pwmgr-core`, never on `src-tauri`. The read-only guarantee is
  structural: there is no disabled/hidden write command to re-enable by mistake — write-path code
  simply isn't linked into this binary.

## M6 Web-Mode — Full command parity via core promotion
- source: docs/superpowers/specs/2026-07-18-m6-web-mode-design.md
- type: api-contract
- content: Every command in `src-tauri/src/commands.rs` becomes reachable over HTTP, not a curated
  subset — requires finishing the promotion M5 started (moving the remaining ~25 `do_*` functions
  into `pwmgr-core`). After this, all three front-ends (`src-tauri`, `viewer-src-tauri`, `pwmgr-web`)
  consume one shared copy of every command; no command's logic exists in more than one place.

## M6 Web-Mode — Session / Auth / CSRF model
- source: docs/superpowers/specs/2026-07-18-m6-web-mode-design.md
- type: protocol
- content: 256-bit random token per login (per `docs/TRD.md:68-70`), HttpOnly + `SameSite=Strict`
  cookie, required on every request after login. Multi-session support scoped specifically to **one
  shared OS user account with multiple app-level named vaults** — not isolation between separate OS
  accounts (`127.0.0.1` TCP binding does not provide that; explicitly out of scope, would need a
  different mechanism such as a Unix domain socket). Background sweep every 2 seconds locks any
  session past its idle timeout; locked entries persist (re-authenticatable) but are pruned after 24h
  of inactivity. CSRF mitigation: `SameSite=Strict` cookie + a required custom header on all mutating
  (non-GET) requests; no CORS headers configured (UI is served same-origin).

## M6 Web-Mode — Bind address and status-code convention
- source: docs/superpowers/specs/2026-07-18-m6-web-mode-design.md
- type: nfr
- content: Binds to `127.0.0.1` only, hard-coded at the listener level — not a config option, same
  "compile-time fact, not a UI restriction" philosophy M5 applied to read-only, applied here to bind
  address. `copy_to_clipboard` is deliberately not exposed as an HTTP endpoint — the browser's own
  `navigator.clipboard.writeText` is used instead, including a JS-side re-implementation of the
  30-second auto-clear (a genuine second implementation of a security-relevant timing property, not
  just a routing decision).

## M6 Web-Mode — HTTP status-code mapping
- source: docs/superpowers/specs/2026-07-18-m6-web-mode-design.md
- type: api-contract
- content: `CommandError`'s `code()`/`message()` maps to HTTP status as a documented, reviewed
  convention: `locked`→401, `invalid_credentials`→401, `invalid_input`→400, `not_found`→404,
  `internal_error`→500 (etc.) — not ad hoc per-endpoint choices.

## M6 Responsive — 640px breakpoint rule
- source: docs/superpowers/specs/2026-07-19-m6-responsive-layout-design.md
- type: nfr
- content: Single breakpoint at 640px, matching Tailwind v4's built-in `sm` breakpoint (no custom
  value, no new breakpoint system). `≥640px` = today's desktop layout, byte-for-byte unchanged;
  `<640px` = narrow-layout rules apply. Chosen over Tailwind's `md` (768px) so tablets in landscape
  and most laptop windows keep the existing sidebar. Applies to both `index.html` (main app) and
  `viewer.html` (read-only Viewer). Out of scope: any change to the desktop Tauri window experience
  at ≥640px; automated viewport/screenshot testing (explicitly deferred — see `docs/DEFERRED.md`).

## M6 Responsive — `mobile_nav_style` config field & narrow-nav patterns
- source: docs/superpowers/specs/2026-07-19-m6-responsive-layout-design.md
- type: api-contract
- content: Below 640px, the sidebar is replaced by one of three interchangeable patterns — icon rail
  (default), bottom tab bar, or hamburger + slide-over drawer — selectable via a new Settings entry.
  New `Config` field validated against `["icon-rail", "bottom-tabs", "drawer"]`; new
  `do_update_mobile_nav_style` command exposed as both a Tauri command and an HTTP route (identical
  shape to `do_update_nav_collapsed`); new `ConfigDto` field `mobileNavStyle`; default value for all
  users is `icon-rail`. Has no effect at ≥640px. Editor dialogs (`SiteEditor`, `CredentialEditor`)
  become fluid-width below 640px (`w-full max-w-80 mx-4`) instead of a fixed `w-80`.
