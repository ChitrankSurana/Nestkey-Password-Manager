# Context (from DOC-type documents)

31 unique documents (32 classification JSONs, one duplicate pair collapsed — see
`INGEST-CONFLICTS.md` [INFO]) were classified `type: DOC` in this ingest. Notes below are grouped by
topic; every note is attributed to its source path(s).

---

## Topic: Architecture

- source: docs/ARCHITECTURE.md
- React frontend (webview) + Rust core (`src-tauri`) over Tauri IPC + disk (`~/.pwmgr/registry.db`,
  `~/.pwmgr/vaults/<user>.vault`, `~/.pwmgr/backups/*.bak`, `~/.pwmgr/config.json`).
- Rust core modules: `crypto`, `registry`, `auth`, `session`, `vault`, `generator`, `health`,
  `backup`, `portio`, `commands`.
- Design rationale: secrets never cross the IPC boundary in derived form (frontend sends the master
  password once, then only issues intent commands); the registry DB is deliberately unencrypted but
  secret-free (usernames + KDF salts only); SQLCipher + per-field AES-GCM is defense in depth, not
  redundancy; read-only viewer (M5) reuses `crypto`+`vault`+`registry` as a library with no write-path
  commands registered; web mode (M6) reuses the same Rust core with an HTTP layer bound to
  `127.0.0.1` only and a session-token middleware in front of the same `commands` module.
- Module isolation for testability: `crypto` has no knowledge of SQLite; `vault` has no knowledge of
  Argon2/passwords; `auth`/`session` are the only modules that ever see the master password;
  `commands` is a thin translation layer with no business logic of its own.

## Topic: Usage Guides (per-app)

- source: docs/USAGE_GUIDE.md — index document pointing to the three per-app guides below plus
  `KNOWN_LIMITATIONS.md` and `testing/`.
- source: docs/HOW_TO_USE_MAIN_APP.md — full read/write desktop app. Covers install/run
  (`npm run tauri dev`), registration (no password recovery, by design), generic-error login,
  the Vault screen (nav rail: Health/Generator/Trash/Data/Settings/Lock), 4 generator methods
  (Random/Memorable/Pattern/Pronounceable) with a Simple/Advanced(zxcvbn) strength-meter toggle that
  explicitly does **not** change Health Dashboard "Weak" classification, Health Dashboard
  (Weak/Reused/Old/Breached cards, HIBP opt-in), Settings table, Trash (soft-delete + Empty Trash
  confirmation), Backup/Restore, CSV export (3 modes: Encrypted-this-device, Encrypted-with-password,
  Plaintext gated behind typing `EXPORT PLAINTEXT`), CSV import (Chrome/Firefox/generic mapper,
  preview→commit), and on-disk data layout.
- source: docs/HOW_TO_USE_VIEWER_APP.md — second, smaller desktop app for when the Main App is
  unavailable; opens the same encrypted vault but has no add/edit/delete commands compiled in at
  all. Same login/search/reveal/copy/lock behavior as Main App; no Register screen (registering
  counts as "adding data"). Reflects the Main App's last-set theme/accent with no theme picker of
  its own.
- source: docs/HOW_TO_USE_WEB_PORTAL.md — full feature set from a browser tab at
  `http://127.0.0.1:7373`, never listens beyond localhost. Same login/register/shared vault files as
  Main App. One difference: clipboard copy uses the browser's own clipboard permission instead of a
  native OS call. Each browser session is independent; sessions live in memory only (process
  crash/restart logs everyone out, vault data itself untouched).

## Topic: Testing Guides (manual QA checklists)

- source: docs/testing/HOW_TO_TEST_MAIN_APP.md — Part 1 automated (`cargo test --workspace`,
  `npm test`, `npm run build`), Part 2 manual walkthrough (13 numbered sections: registration/login
  security, add-site/search, edit, delete/trash/restore, generator, Health Dashboard, Settings,
  manual lock, backup/restore, CSV export, CSV import, dark theme consistency, keyboard-accessible
  tooltips).
- source: docs/testing/HOW_TO_TEST_VIEWER_APP.md — first-ever written procedure for launching this
  app (deferred through M5 and M6 per `KNOWN_LIMITATIONS.md`). Confirms read-only is structural (only
  8 commands registered, no write command exists in the binary at all).
- source: docs/testing/HOW_TO_TEST_WEB_PORTAL.md — first-ever written procedure for this surface too.
  Covers session cookie behavior (HttpOnly, SameSite=Strict), CSRF header enforcement (`403` without
  `x-pwmgr-csrf`, `200` with any value), multi-session isolation (separate cookie jars never leak
  data), idle sweep (`401 {code:"locked"}`, no auto-redirect since there's no browser
  `vault-locked` event), network-exposure confirmation (`netstat` shows `127.0.0.1:7373` only),
  server crash/restart behavior (in-memory session store, vault data untouched).
- source: docs/HOW_TO_TEST_M6_RESPONSIVE.md — manual-only checklist (no automated viewport tooling,
  per `docs/DEFERRED.md`) for the 640px breakpoint and 3 narrow-nav patterns across both
  `index.html` and `viewer.html`.
- source: docs/HOW_TO_TEST_BIOMETRIC_UNLOCK.md — manual QA for Windows Hello + DPAPI integration,
  added on branch `feature/bitwarden-port-phase1`. Unit-tested only against a fake provider
  (`core/src/biometric.rs`); never exercised against a real Windows Hello prompt before this doc.
  Defaults off (`biometric_enabled: false`). Desktop app only, no web-mode route.
- source: docs/HOW_TO_TEST_SSH_AGENT.md — manual QA for an SSH agent over a Windows named pipe,
  added on branch `feature/bitwarden-port-phase1`. Signing logic (`core/src/ssh_agent.rs`) is
  unit-tested with a real Ed25519 key; the named-pipe server and real OpenSSH/PuTTY interop are not
  automatable and are the subject of this checklist. Defaults off (`ssh_agent_enabled: false`).
  Known v1 gaps (deliberate, not bugs): no in-app key generation, no passphrase-encrypted key
  support, no trash/undo for deleted keys.
- source: docs/testing/PHASE_B_MANUAL_VERIFICATION.md — M1 Phase B's Task 28 manual UI walkthrough:
  registration/generic-error login, add/search, edit, delete/trash/restore, auto-lock takes live
  effect, manual lock, dark theme consistency, keyboard-accessible tooltips (including the
  best-effort disabled-control tooltip case).

## Topic: Known Limitations & Deferred Items

- source: docs/KNOWN_LIMITATIONS.md (last updated 2026-07-19) — Testing gaps: Viewer App and Web
  Portal had never been launched/clicked-through end-to-end before the `testing/` docs above were
  written; no automated UI/visual test tooling exists anywhere (deliberate, per TRD §7); a
  two-browser-session HTTP integration test hasn't been written yet (today: 3 single-session
  integration tests); tooltip-on-disabled-control behavior is best-effort/unconfirmed. Known
  behavioral quirks: Health Dashboard "Weak" always scores with zxcvbn regardless of the Settings
  strength-algorithm toggle (confirmed intentional after review); a Web Portal crash/restart silently
  logs out every session; Web Portal concurrency accepted at "household scale," not load-tested
  beyond it. Deferred code cleanup: no `AppState::open(paths)` production constructor yet (same
  boilerplate duplicated 3×); `SessionStore::remove` has zero production callers. Not yet built: M6
  responsive layout is spec'd/planned but has zero implementation code as of this doc; automated
  viewport/screenshot testing deferred; macOS/Linux builds not started (Windows-only today); Android
  companion + sync is design-only. Deliberate non-goals (restated from PRD/THREAT_MODEL): cloud sync,
  browser extension/autofill, master-password recovery, defense against O1–O4 threat classes.
- source: docs/DEFERRED.md — one tracked item: automated viewport/screenshot testing (e.g.
  Playwright), raised during M6 responsive-layout design, deferred because the project has no
  browser/UI test tooling today and every prior milestone (M4, M5, M6 web-mode) used manual QA
  instead. Revisit trigger: either a real regression slips through manual QA that automated tests
  would have caught, or responsive-layout code accumulates enough screens/breakpoints that manual
  re-verification becomes the bottleneck.

## Topic: Figma UI vs. current feature set (planned new-UI port)

- source: docs/FIGMA_UI_VS_CURRENT_FEATURES.md — feature-by-feature comparison between a Figma
  ("Password manager UI design") mockup and the current app. Gaps to close ("Closing" status): 4 item
  types (Login/Secure Note/Payment Card/Identity — schema has no type discriminator today),
  favorites (no column/command exists), folders (no table/reference exists), per-item password health
  (today's health check is vault-wide only). Deliberately deferred ("Open" status): Sync (no
  server/account system exists; out of scope, button removed rather than shown disabled). Where the
  app is already ahead of the mockup: multi-credential-per-site data model, live TOTP display (vs. a
  static placeholder in the design). Shipping features absent from the Figma design that need a home
  in any new UI: Health dashboard, password generator, Data hub (backup/CSV), Settings (security incl.
  PIN quick-unlock and Windows Hello biometric unlock, SSH agent [desktop-only], appearance, strength
  algorithm), reprompt-on-reveal, custom fields (text/hidden/boolean), clipboard auto-clear, auto-lock
  on OS suspend, multiple credentials per site. Other deferred items: CSV import/export stays
  logins-only for now (new item types won't appear in exports); the read-only Viewer won't display
  the new item types either (separate future work); the 5-color accent picker vs. the design's single
  fixed accent is an open design question.

## Topic: Setup / Environment

- source: docs/FRESH_MACHINE_SETUP.md (classified twice under two classification IDs pointing at the
  same source file — collapsed here; see `INGEST-CONFLICTS.md` [INFO]) — Windows-only setup guide
  (10 2004+/11). All three apps (Main, Viewer, Web Portal) share one `~/.pwmgr` data directory, so a
  user registers once and can log into any of the three. Automated path:
  `scripts/setup-dependencies.ps1` (installs Git, Rust, Strawberry Perl, MSVC Build Tools, Node.js,
  WebView2 via `winget`, then `npm install` + `cargo fetch`). Manual path: a 5-tool table (Git, Rust/
  rustup, MSVC Build Tools C++ workload, Strawberry Perl, Node.js LTS, WebView2 Runtime). Build
  verification: `cargo test --workspace` (first run compiles vendored OpenSSL+SQLCipher from source,
  5-15+ min), `npm test`, `npm run build`. Running each app documented (Main:
  `npm run tauri dev`; Viewer: `cd viewer-src-tauri && npx tauri dev`; Web Portal:
  `npm run build && cargo run -p pwmgr-web` from repo root, binds `127.0.0.1:7373` only).
- source: docs/PATH_AUDIT.md (audit date 2026-07-19) — project-scoped PATH entries required to
  build/run/test this Rust+Node workspace: Cargo bin, Node/npm, Git, OpenSSH (repo's `origin` is an
  SSH URL), Strawberry Perl bin + site\bin (needed by `rusqlite`'s
  `bundled-sqlcipher-vendored-openssl` feature, which compiles OpenSSL from source via a Perl-based
  `Configure` script), Windows core dirs, PowerShell. One "required but not currently on PATH" entry:
  MSVC `link.exe`/`cl.exe` dir — not added permanently because `cargo build` already auto-locates
  MSVC via the registry/vswhere, and adding it would conflict with other `link`/`cl`-named tools.

## Topic: Repo / Branch Hygiene (point-in-time snapshot)

- source: docs/REPO_VS_LOCAL_COMPARISON_2026-07-19.md — snapshot after pushing branch
  `docs/security-and-setup-audit` (PR #7, not yet merged to `main`; 4 files exist only on that
  branch: `SECURITY_CONCERNS.md`, `docs/PATH_AUDIT.md`, `docs/FRESH_MACHINE_SETUP.md`,
  `scripts/setup-dependencies.ps1`). The `.worktrees/m6-responsive-layout` worktree was 2 commits
  behind `origin/m6-responsive-layout` at snapshot time. `origin/m4-health-nav-theming` and
  `origin/m5-readonly-viewer` exist remotely with no local tracking branch (normal, already-merged).
  This is a point-in-time git-state snapshot, not an ongoing constraint — treat as historical context
  only, not current repository state.

## Topic: Bitwarden Comparison Research (docs/bitwarden_compare_ideas/, research-only, no code changed)

- source: docs/bitwarden_compare_ideas/00-summary.md (research date 2026-07-26) — top ranked ideas
  across all three reports: (1) split the data-encryption key from the password-derived key
  (Bitwarden wraps a random `UserKey` under a password-derived key; this project derives `vault_key`
  straight from Argon2id and uses it directly — confirmed via grep that no `change_password`/rewrap
  function exists anywhere in `core/src`, so today a password change would require re-encrypting the
  whole vault); (2) decide and explicitly document unlock-attempt throttling (open gap on both sides);
  (3) scripting-friendly `?raw=true`/`Accept: text/plain` flags on the Web Portal HTTP API; (4)
  generator ambiguous-character avoidance + per-class minimum counts; (5) JSON export alongside CSV
  (to represent multiple credentials per site without flattening); (6) surface "device compromise is
  out of scope" to end users in `USAGE_GUIDE.md`, not just `THREAT_MODEL.md`; (7) a ciphertext
  version/type tag on `seal()`/`open()` (flag only, don't build). Explicitly **not** recommended:
  standalone CLI binary (new attack surface, YAGNI), Card/Identity/SSH-key item types & folders/
  collections & org-sharing (speculative scope, no user story — PRD non-goals), PBKDF2 fallback/
  dual-KDF/RSA org-key wrapping (solve problems this project doesn't have). Where this project already
  does as well or better: compile-time-enforced read-only Viewer, explicit plaintext-CSV confirmation
  gate, AEAD-only ciphertext with no legacy unauthenticated format, tested username-enumeration timing
  defense, concretely specified CSRF double-submit mitigation, structurally zero-knowledge (no server
  exists for Main/Viewer; Web Portal hard-bound to `127.0.0.1`).
- source: docs/bitwarden_compare_ideas/01-crypto-core.md — detailed KDF/key-hierarchy/ciphertext-
  format/zeroization comparison against `bitwarden/sdk-internal`'s crypto crates (fetched 2026-07-26).
  Confirms the split-key gap (ranked #1) in the most detail; also notes this project's ciphertext
  format has no version/type tag (Bitwarden's `EncString` does) and that enforced KDF minimums don't
  apply here (single hardcoded Argon2id param set, no user-facing KDF choice to misconfigure).
- source: docs/bitwarden_compare_ideas/02-vault-and-cli.md — vault data model / CLI/UX comparison
  against Bitwarden's published docs (fetched 2026-07-26). This project's `Site`/`Credential` model
  covers only Bitwarden's "Login" item type (no Card/Identity/Secure-Note/SSH-key, no folders/
  collections/favorites/custom fields) but already exceeds Bitwarden's flat item list for
  multi-credential-per-site. No CLI exists in this project at all (GUI-only, all three surfaces).
- source: docs/bitwarden_compare_ideas/03-server-security-notes.md — Bitwarden's publicly-documented
  security whitepaper/ETH-Zurich-audit model vs. this project's `THREAT_MODEL.md` (fetched
  2026-07-26). Bitwarden's O1-equivalent (client device/keylogger compromise) is explicitly out of
  scope for them too, phrased directly to end users ("you are responsible for...") — recommends this
  project surface a similar one-liner beyond just `THREAT_MODEL.md`. Neither project documents a
  concrete brute-force/lockout algorithm (echoes the summary doc's #2 recommendation). Notes
  Bitwarden's 2025 ETH Zurich audit modeled a fully malicious server and found 10-12 attacks, all
  medium/low severity requiring server-infrastructure control as a precondition — not directly
  comparable since this project has no server for Main/Viewer and a hard-bound localhost server for
  Web Portal.

## Topic: Milestone Implementation Plans (docs/superpowers/plans/ — execution runbooks, not specs)

- source: docs/superpowers/plans/2026-07-01-m1-secure-core.md — M1 Phase A: standalone `pwmgr-core`
  Rust library (crypto/registry/vault/auth/session), TDD, no Tauri/frontend yet. Notes a real
  lesson-learned: declaring all `pub mod` upfront in `lib.rs` before their files exist is a hard
  compile error (`E0583`) that blocks `cargo test` for every module at once — modules must be
  declared incrementally, one per task, to preserve TDD granularity.
- source: docs/superpowers/plans/2026-07-05-m1-phase-b.md — M1 Phase B: Tauri desktop shell + IPC
  command layer + React vault UI on top of `pwmgr-core`. Converts the repo root to a Cargo workspace;
  notes that listing a not-yet-existing member path (`src-tauri`) in `[workspace] members` is a hard
  manifest-load error in modern Cargo, not a warning — the member is added only once its directory
  exists on disk.
- source: docs/superpowers/plans/2026-07-12-m2-password-generator.md — M2: 4-method generator in
  `pwmgr-core::generator` (serde-free, reuses `rand::thread_rng()`), simple entropy estimator lives
  entirely in the frontend TS, advanced estimator is the `zxcvbn` Rust crate in the app crate only
  (kept out of `core` deliberately — precedent later cited by M4's `core::health` network-free
  invariant).
- source: docs/superpowers/plans/2026-07-12-m3-backup-csv.md — M3: encrypted backup/restore (hand-
  rolled versioned binary bundle, no serde) + CSV export (3 modes) + CSV import (2-phase preview/
  commit, transactional rollback). Notes the `csv` crate's dependency on `serde`/`serde_core` turned
  out to be unconditional (not feature-gated as originally assumed) — `serde_core`'s trait
  definitions transitively appear in `pwmgr-core`'s dependency tree, but no `serde_derive` codegen
  runs anywhere in the workspace because of it; the actual invariant ("no serde derive in our own
  types") still holds.
- source: docs/superpowers/plans/2026-07-16-m5-readonly-viewer.md — M5 implementation: Slice 1
  promotes shared `AppState`/DTO/`do_*` glue into `pwmgr-core` with zero behavior change (214-test
  baseline must stay green); Slice 2 adds the `viewer-src-tauri` binary registering only 9 read
  commands; Slice 3 adds the frontend. Documents a judgment call vs. the design spec: the spec said
  the viewer's new command wrappers should get their own unit tests, but tracing actual project
  convention showed thin `#[tauri::command]` wrappers are never unit-tested directly anywhere in this
  codebase — only the `do_*` inner functions are, and those move (with their tests) into `core` in
  Slice 1 — so no redundant wrapper-test file was added, matching real convention over the spec's
  wording.
- source: docs/superpowers/plans/2026-07-18-m6-web-mode.md — M6 web-mode implementation. Baseline
  before this plan: 224 total tests (72 `pwmgr-app` + 150 `pwmgr-core` + 2 e2e). Documents 4 flagged
  deviations from the design spec's literal wording: (1) `do_check_breach` promotion requires
  converting `hibp` to `reqwest::blocking` to drop the `tauri::async_runtime` dependency (run via
  `spawn_blocking` in the web server to avoid panicking inside a tokio runtime); (2) Tauri v2's
  frontend-detection global is actually `__TAURI_INTERNALS__`, not `__TAURI__` as the spec assumed
  (`withGlobalTauri` unset by default); (3) there is no Tauri event bus in the browser, so an
  idle-locked web session surfaces as a `401 {code:"locked"}` rather than an automatic UI redirect;
  (4) each `UserSession` owns a full `AppState` (including its own registry connection) rather than
  the spec's pictured single shared registry connection, to avoid an invasive `AppState` decomposition
  — judged to match the spec's intent with a smaller, safer diff.
- source: docs/superpowers/plans/2026-07-19-m6-responsive-layout.md — M6 responsive-layout
  implementation. Baseline before this plan: 243 total tests. Adds `mobile_nav_style` end-to-end
  through the existing Settings-field pattern; narrow-nav selection is pure CSS breakpoint visibility
  (`hidden sm:flex`/`sm:hidden`), not a JS resize listener. Deliberately no automated viewport tests
  (per `docs/DEFERRED.md`), no changes to the ≥640px desktop experience, no visual/brand redesign.

## Topic: Dependency / npm audit fixes (vitest 2 → 4, GHSA-67mh-4wv8-2f99)

- source: docs/superpowers/plans/2026-07-19-npm-audit-fix.md;
  docs/Plans/npm_vulnerability_fix_plan_19-07-2026.md — **two independent documents describing the
  same fix**, consistent with each other (see `INGEST-CONFLICTS.md` [INFO], not a conflict). Root
  cause: `vitest@2.1.9` nests a vulnerable `vite@5.4.21`→`esbuild@0.21.5` (the top-level
  `vite@7.3.6`→`esbuild@0.28.1` used by the production build is already unaffected). Fix: upgrade
  `vitest` to `^4.1.10` (`npm audit fix --force` or a manual `package.json` edit + `npm install`),
  verify `npm test`/`npm run build`/`npm audit` (expect 0 vulnerabilities) and, as a broader sanity
  check, `cargo test --workspace`. Severity noted as low (dev-only, headless `vitest run`, never
  opens the vulnerable watch/UI dev server) — the fix is done anyway because it's cheap. Explicitly
  not bundled into this fix: unrelated `npm outdated` items (`@vitejs/plugin-react`, `typescript`,
  top-level `vite`).
