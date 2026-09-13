# Context (from DOCs)

31 documents were classified `DOC` in this ingestion batch (30 unique source paths — `docs/FRESH_MACHINE_SETUP.md` was classified twice by the classifier, collapsed to one entry below; see `INGEST-CONFLICTS.md` INFO). Of those 30, **7 were excluded from this file** because they form two cross-reference cycles detected in the `cross_refs` graph (see `INGEST-CONFLICTS.md` BLOCKERS): `HOW_TO_USE_MAIN_APP.md`, `HOW_TO_USE_VIEWER_APP.md`, `HOW_TO_USE_WEB_PORTAL.md` (cycle A), and `docs/testing/HOW_TO_TEST_MAIN_APP.md`, `docs/testing/HOW_TO_TEST_VIEWER_APP.md`, `docs/testing/HOW_TO_TEST_WEB_PORTAL.md`, `docs/KNOWN_LIMITATIONS.md` (cycle B). The remaining 23 are extracted below, topic-keyed.

## Architecture Overview
- source: docs/ARCHITECTURE.md
- Three-layer shape: React/TypeScript/Tailwind frontend (webview) → Tauri IPC → Rust core (`src-tauri`, all secrets and business logic: `crypto`, `registry`, `auth`, `session`, `vault`, `generator`, `health`, `backup`, `portio`, `commands`) → disk (`~/.pwmgr/registry.db` unencrypted usernames+salts, `~/.pwmgr/vaults/<user>.vault` SQLCipher-encrypted, `~/.pwmgr/backups/*.bak` encrypted, `~/.pwmgr/config.json`).
- Design rationale: secrets never cross the IPC boundary in derived form; registry DB is deliberately unencrypted but secret-free; SQLCipher + per-field AES-GCM is defense in depth, not redundancy; M5 viewer reuses `crypto`+`vault`+`registry` as a library with no write-path commands registered; M6 web mode reuses the same Rust core with an HTTP layer bound to `127.0.0.1` and session-token middleware.
- Module isolation: `crypto` has no knowledge of SQLite; `vault` has no knowledge of Argon2/passwords (handed an already-derived key); `auth`/`session` are the only modules that ever see the master password, transiently; `commands` is a thin IPC translation layer with no business logic.

## Deferred Items
- source: docs/DEFERRED.md
- Automated viewport/screenshot testing (e.g. Playwright) was raised during M6 responsive-layout design and explicitly deferred. Reason: no browser/UI test tooling exists today (only Node-env `vitest` unit tests); every prior milestone (M4, M5, M6 web-mode) treated visual/frontend verification as manual QA via a deferred walkthrough rather than introducing browser automation.
- Revisit trigger: (a) a real regression slips through manual QA that automated tests would have caught, or (b) responsive layout code accumulates enough screens/breakpoints that manual re-verification becomes the bottleneck.

## Figma UI Design vs. Current Feature Set
- source: docs/FIGMA_UI_VS_CURRENT_FEATURES.md
- Feature-by-feature comparison between a Figma-designed vault UI ("Password manager UI design") and the shipped application, produced independently of the codebase.
- Gaps where the Figma design assumes capability the app lacks today (status "Closing" = scheduled for the new-UI port): four item types (Login/Secure Note/Payment Card/Identity, app has Login only), Favourites (star toggle, no backing column/command today), Folders (no folders table/reference today), per-item password health (today's health analysis is vault-wide only, not per-item). "Sync" is marked "Open" (deliberately deferred — no server/account/sync architecture exists; implementing this is "a project in its own right, not a UI feature"). TOTP presentation is marked "Aligned (app is ahead)" — the app already has a live authenticator display, the Figma design only shows a static placeholder box.
- Shipped features absent from the Figma design (need a home in the new UI): Health dashboard, password generator, Data hub (backup/CSV export/CSV import), Settings — security (auto-lock, old-password threshold, breach-check opt-in, **PIN quick-unlock**, **Windows Hello biometric unlock**), Settings — **SSH agent** (Windows named pipe, desktop build only), Settings — appearance (theme, 5-color accent picker, narrow-window nav style choice), Settings — strength algorithm choice, Login screen (master password + optional PIN + Windows Hello), **reprompt-on-reveal** (per-credential flag requiring master-password re-entry before reveal/copy), **custom fields** (text/hidden/boolean typed), clipboard auto-clear (30s), auto-lock-on-suspend, multiple credentials per site.
- Deferred items explicitly not being closed in this UI-port pass: Sync (button removed, not shown disabled); CSV import/export for new item types (logins only, for now); the read-only viewer build (stays login-only subset); the accent-colour picker (design assumes one fixed accent, app has five — open design question on whether to keep/reduce/drop).
- Note: several of these shipped features (PIN quick-unlock, Windows Hello biometric unlock, SSH agent, custom fields, reprompt-on-reveal, TOTP) are not mentioned anywhere in `docs/PRD.md`'s milestone table (§7) or user stories (§6) — PRD/TRD (last updated 2026-07-01) appear to predate this later feature work. See `INGEST-CONFLICTS.md` INFO.

## Bitwarden Comparison — Summary and Ranked Recommendations
- source: docs/bitwarden_compare_ideas/00-summary.md
- Research-only comparison (2026-07-26), explicitly not yet built. Top-ranked actionable ideas across all three sub-reports: (1) split the data-encryption key from the password-derived key — currently `vault_key` is derived straight from Argon2id and used directly as the SQLCipher key; no `change_password` function exists anywhere in `core/src`, so a password change today means re-encrypting the whole vault; a wrapped random data key would make it "re-wrap one 32-byte key" — flagged as the single biggest architectural gap found, needs a one-time migration for existing vaults; (2) decide and explicitly document unlock-attempt throttling (currently undocumented on both this project's and Bitwarden's side); (3) scripting-friendly flags (`?raw=true`) on the Web Portal HTTP API; (4) ambiguous-character avoidance + per-class minimum counts in the random generator; (5) JSON export alongside CSV (to represent multiple credentials per site without flattening); (6) surface the "device compromise is out of scope" statement to end users, not just contributors; (7) a ciphertext version/type tag on `seal()`/`open()` — flagged only, not recommended to build without a concrete need.
- Explicitly NOT recommended: a standalone CLI binary (real gap, but new attack surface — a deliberate YAGNI call for the user to make, not a default yes); Card/Identity/SSH-key item types, folders/collections, org-sharing (exist in Bitwarden to power autofill/multi-user sharing, both explicit PRD non-goals); PBKDF2 fallback/dual-KDF support, RSA org-key wrapping, per-item Cipher Key envelope (solve problems — legacy migration, multi-tenant sharing — this project doesn't have).
- Areas already judged as good or better than Bitwarden: compile-time-enforced read-only Viewer binary; explicit plaintext-CSV export confirmation gate; AEAD-only ciphertext (no legacy unauthenticated format); username-enumeration timing defense (decoy-salt Argon2id, test-asserted ≥5ms); concretely specified CSRF mitigation; structurally zero-knowledge (no server exists for Main/Viewer; Web Portal hard-bound to 127.0.0.1).

## Bitwarden Comparison — Crypto Core Detail
- source: docs/bitwarden_compare_ideas/01-crypto-core.md
- Bitwarden's KDF enum supports both `PBKDF2 { iterations }` (legacy, min 5,000, default 600,000) and `Argon2id { iterations, memory, parallelism }` (min 2 iter/16 MiB/1 lane, default 6 iter/32 MiB/4 lanes), with hard-errored minimums enforced in code. Bitwarden derives a `MasterKey` used only to wrap things (never decrypts vault content directly) and a separately-derived `master_password_hash` (PBKDF2) for server-side login verification — analogous to this project's `verifier_key` split, but this project uses Argon2id only, fixed params (64 MiB/3 iterations/4 lanes/32-byte output), no PBKDF2 fallback, no enforced minimum (hardcoded so nothing to violate). Bitwarden's zeroization uses pinned heap allocations plus a `clear_stack()` trick specifically for Argon2's C-adjacent implementation leaving working memory on the stack; this project's `derive_key` already zeroizes via `ZeroizeOnDrop`, flagged as marginal-value-but-worth-knowing-about.
- Verifier check comparison: this project's `verifier_matches` uses `subtle::ConstantTimeEq` plus a decoy-salt Argon2id call on unknown usernames to close a username-enumeration timing oracle — judged more defense-in-depth than what was found visible in Bitwarden's crypto crate.

## Bitwarden Comparison — Vault Data Model & CLI/UX Detail
- source: docs/bitwarden_compare_ideas/02-vault-and-cli.md
- Bitwarden vault item types: Login, Secure Note, Card, Identity, SSH key (5 types), plus folders (personal) vs. collections (org-shared) as distinct concepts, favorites, and typed custom fields (Text/Hidden/Boolean). This project's data model (`core/src/vault.rs`) is effectively Bitwarden's "Login" type only — no Card/Identity/Secure-Note/SSH-key equivalents, no folders, no collections, no favorites, no custom fields — but supports multiple credentials per site, which Bitwarden's flat item list doesn't do as cleanly.
- Bitwarden `bw` CLI: session-key based (`bw login`/`bw unlock` → `BW_SESSION`), `bw serve` exposes the full CLI as a local REST API, global scripting flags (`--raw`, `--response`, `--pretty`, `--quiet`, `--nointeraction`). This project has no CLI at all — all three surfaces (Main App, Viewer, Web Portal) are GUI-only.
- This project's generator (`core/src/generator.rs`): four methods (random, memorable via EFF short wordlist, pattern DSL, pronounceable) — random mode lacks per-class minimum-count and ambiguous-character-avoidance options that Bitwarden-equivalent generators commonly have.
- This project's export/import (`core/src/portio.rs`, `core/src/backup.rs`): CSV export/import (generic 6-column schema), Chromium-format import parser; encrypted backup is a custom length-prefixed binary bundle wrapping the registry record + raw SQLCipher vault bytes (not JSON, not per-item granular).

## Bitwarden Comparison — Server/Security Model Detail
- source: docs/bitwarden_compare_ideas/03-server-security-notes.md
- Bitwarden's key hierarchy: master password + email salt → PBKDF2 (default 600,000 iter, Argon2id optional) → Master Key → HKDF stretch → Stretched Master Key; separate PBKDF2-derived Master Password Hash sent server-side for auth only; a random Generated Symmetric Key wraps a per-item Cipher Key (per-item envelope encryption). This project uses one derived key path (Argon2id only) since there's no server to authenticate against separately, so no Master-Key/Master-Password-Hash split is needed; uses SQLCipher full-DB + per-field AES-256-GCM instead of per-item Cipher Key envelopes (coarser granularity, no per-item key-management surface).
- Bitwarden's third-party audits (2022–2025): Cure53, IOActive, Unit 42, Fracture Labs, SOC 2 Type 2 + SOC 3, and a 2025 ETH Zurich cryptography audit modeling a fully malicious server (10-12 attacks found, all medium/low severity, 7 fixed, 3 accepted as intentional tradeoffs). No public documentation found (on either project) of specific brute-force/lockout throttling parameters.
- Ideas worth adopting: explicit user-facing statement that device/OS compromise is out of scope (already in THREAT_MODEL.md as O1 for contributors, not yet surfaced to end users); explicitly document failed-unlock-attempt throttling behavior (or the deliberate decision not to add one) in THREAT_MODEL.md; third-party audit transparency as a future practice once any external review happens.

## M1 Phase A — Secure Core Implementation Plan
- source: docs/superpowers/plans/2026-07-01-m1-secure-core.md
- Step-by-step plan to build the standalone Rust crypto/vault/auth/session core library (`pwmgr-core`) with full TDD test coverage: Argon2id, AES-256-GCM, SQLCipher, registry.

## M1 Phase B — Desktop App + IPC + React Vault UI Implementation Plan
- source: docs/superpowers/plans/2026-07-05-m1-phase-b.md
- Step-by-step plan for the Tauri desktop shell, IPC command layer, and React vault UI: authentication, auto-lock, dark theme, Tailwind CSS. Companion manual-verification checklist is `docs/testing/PHASE_B_MANUAL_VERIFICATION.md` (below).

## M2 — Password Generator + Strength/Entropy Implementation Plan
- source: docs/superpowers/plans/2026-07-12-m2-password-generator.md
- Task-by-task plan for the four generator methods (random, memorable, pattern, pronounceable), zxcvbn-based strength estimation, and clipboard integration, touching `CredentialEditor` and `Settings`.

## M3 — Encrypted Backup/Restore + CSV Export/Import Implementation Plan
- source: docs/superpowers/plans/2026-07-12-m3-backup-csv.md
- Task-by-task plan for encrypted backup/restore bundles, CSV export in 3 modes, and CSV import with a two-phase preview/commit flow, in Rust `core::backup` and `core::portio` modules.

## M6 — Web Mode Implementation Plan (executes the web-mode SPEC)
- source: docs/superpowers/plans/2026-07-18-m6-web-mode.md
- Runbook decomposing the M6 web-mode design (see `constraints.md`) into Slice 1/2/3 task sequences (pwmgr-core promotion, web server scaffolding). Contains "findings surfaced during planning" — implementation-time deviations from the spec's wording, explicitly flagged as implementation notes, not new locked architectural decisions.

## M6 — Responsive Layout Implementation Plan (executes the responsive-layout SPEC)
- source: docs/superpowers/plans/2026-07-19-m6-responsive-layout.md
- Runbook for the M6 responsive layout: mobile nav pattern implementation, backend config plumbing, manual QA below the 640px breakpoint. Companion manual test checklist is `docs/HOW_TO_TEST_M6_RESPONSIVE.md` (below).

## npm Audit Fix — vitest 2 → 4 Upgrade (two independent write-ups, same decision)
- source: docs/superpowers/plans/2026-07-19-npm-audit-fix.md AND docs/Plans/npm_vulnerability_fix_plan_19-07-2026.md
- Both documents independently describe the identical fix: upgrade `vitest` from 2.1.9 to 4.1.10 to collapse a vulnerable nested `vite@5.4.21`/`esbuild@0.21.5` subtree (5 npm audit findings: 3 moderate, 1 high, 1 critical), resolving GHSA-67mh-4wv8-2f99. Top-level `vite@7.3.6`/`esbuild@0.28.1` toolchain is already safe; the vulnerable path is a dev-only esbuild dev-server issue, never shipped in the Tauri production build — severity is low in practice but the fix is a trivial one-line `package.json` bump plus `npm install`. Verified compatible: only 1 test file in the repo, uses only stable vitest APIs unchanged across v2→v4. No content contradiction between the two write-ups; see `INGEST-CONFLICTS.md` INFO for the duplication note.

## Fresh Machine Setup
- source: docs/FRESH_MACHINE_SETUP.md (classified twice by the classifier under two different JSON filenames for the same source path; collapsed here — see `INGEST-CONFLICTS.md` INFO)
- Windows-only setup guide (macOS/Linux builds not done yet). All three apps (Main, Viewer, Web Portal) share one data directory (`~/.pwmgr`) — register once, log into the same account from any of the three. Prereqs: Windows 10 2004+/11, admin rights, ~2GB disk, internet (first build compiles OpenSSL+SQLCipher from source). Automated setup via `scripts/setup-dependencies.ps1` (installs Git, Rust, Strawberry Perl, MSVC Build Tools, Node.js, WebView2 Runtime via winget) or manual per-tool winget install. Verify via `cargo test --workspace`, `npm test`, `npm run build`. Web Portal served at `http://127.0.0.1:7373`.

## How to Test — Biometric Unlock (Windows Hello)
- source: docs/HOW_TO_TEST_BIOMETRIC_UNLOCK.md
- Manual QA checklist for Windows Hello (`UserConsentVerifier`) + DPAPI (`CryptProtectData`/`CryptUnprotectData`) integration, added on branch `feature/bitwarden-port-phase1`. Only unit-tested against a fake provider (`core/src/biometric.rs`); never exercised against a real Windows Hello prompt or real DPAPI round-trip — this checklist is the missing verification step. Defaults off (`biometric_enabled: false`). Desktop app only — no web-mode route (Windows Hello cannot run server-side for a remote browser). Checklist covers: off-by-default, setup flow (Windows Hello prompt success/cancel), lock/unlock via biometric with master-password fallback, removal, feature-off hides everything (server-side gate, not just UI), restart persistence, and a DPAPI cross-account sanity check (a biometric record set up under Windows account A should not work under account B).

## How to Test — M6 Responsive Layout
- source: docs/HOW_TO_TEST_M6_RESPONSIVE.md
- Manual QA checklist (no automated viewport/screenshot tooling — see Deferred Items above). Test at 375px (phone), 640px (breakpoint), 1024px (desktop) for both `index.html` and `viewer.html`: ≥640px must show no visual regression; <640px must correctly render each of the three `mobileNavStyle` values (icon rail / bottom tab bar / hamburger drawer); editor dialogs stay on-screen with margins at 320-375px; CSV import preview table scrolls horizontally rather than breaking layout; no screen produces a `<body>`-level horizontal scrollbar.

## How to Test — SSH Agent
- source: docs/HOW_TO_TEST_SSH_AGENT.md
- Manual QA checklist for the SSH agent feature, added on branch `feature/bitwarden-port-phase1`. Signing logic (`core/src/ssh_agent.rs`) is unit-tested with a real Ed25519 key including sign+verify round-trip; the actual named-pipe server (`ssh-agent-lib` crate) and real interop with OpenSSH/PuTTY clients is not automatable and needs this manual pass. Defaults off (`ssh_agent_enabled: false`). Desktop app only, no web-mode route. Checklist covers: off-by-default; live listener start/stop on toggle (named pipe `\\.\pipe\pwmgr-ssh-agent` appears/disappears without app restart); key validation (rejects garbage/public keys, rejects passphrase-encrypted keys with a clear error — a known deliberate v1 limitation, accepts valid unencrypted keys); `ssh-add -l` identity listing via a real OpenSSH client; locked vault yields empty identity list, not an error; per-operation signing approval dialog (Allow/Deny); live key deletion; toggle-off/on preserves stored keys. Known deliberate v1 gaps: no in-app key generation (import only), no passphrase-encrypted key support, no trash/undo for deleted SSH keys.

## Web Portal (HTTP Server) — Usage
- source: docs/HOW_TO_USE_WEB_PORTAL.md
- Tutorial for the browser-based localhost web portal: startup, login, feature usage, session behavior, clipboard behavior over HTTP.

## PATH Environment Variable Audit
- source: docs/PATH_AUDIT.md
- Reference doc of required PATH entries for the Rust workspace build (audit date 2026-07-19): Rust toolchain (`.cargo\bin`), Node.js, Git, OpenSSH, Strawberry Perl (`perl\bin` and `perl\site\bin` — required because `rusqlite`'s `bundled-sqlcipher-vendored-openssl` feature compiles OpenSSL from source, and OpenSSL's `Configure` script needs Perl), Windows core paths, PowerShell. MSVC `link.exe`/`cl.exe` is required but intentionally NOT kept permanently on PATH (rustc auto-locates MSVC via registry/vswhere; adding it conflicts with other `link`/`cl`-named tools) — use "Developer PowerShell for VS 2022" or `vcvars64.bat` if a build ever needs it directly.

## M1 Phase B — Manual UI Verification Walkthrough
- source: docs/testing/PHASE_B_MANUAL_VERIFICATION.md
- Manual checklist for the one Phase B verification step (plan doc Task 28, Step 2) that automated tests/code review cannot cover — there is no visual/UI test harness in this repo (no Playwright, no `*.test.tsx`). Covers: registration + generic-error login security (wrong password or nonexistent username both yield the identical "Incorrect master password." message — TRD §3.2/THREAT_MODEL requirement); add site/credential + live search; edit credential; delete → Trash → Restore → Empty Trash; Settings auto-lock taking live effect on the running session (not just next login); manual Lock; dark theme consistency across every screen; keyboard-accessible tooltips (TRD NFR-6) including on Tab-focus and on disabled controls.

## Repo vs. Local Folder Comparison (2026-07-19 snapshot)
- source: docs/REPO_VS_LOCAL_COMPARISON_2026-07-19.md
- Point-in-time git state snapshot, not an ongoing project fact: branch `docs/security-and-setup-audit` (PR #7, adding `SECURITY_CONCERNS.md`, `docs/PATH_AUDIT.md`, `docs/FRESH_MACHINE_SETUP.md`, `scripts/setup-dependencies.ps1`) was in sync with its origin counterpart but not yet merged into `main` as of this snapshot; the `m6-responsive-layout` git worktree was 2 commits behind its origin branch; `origin/m4-health-nav-theming` and `origin/m5-readonly-viewer` exist as remote-only (already-merged) branches with no local tracking branch.

## Usage Guide (Index)
- source: docs/USAGE_GUIDE.md
- Index document pointing to the three per-app usage guides (Main App, Viewer App, Web Portal — the latter three are excluded from this file as cycle-A members, see header note) and to `KNOWN_LIMITATIONS.md`/`testing/` (KNOWN_LIMITATIONS.md is a cycle-B member, also excluded).
