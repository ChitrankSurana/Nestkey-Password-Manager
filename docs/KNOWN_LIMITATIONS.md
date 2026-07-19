# Known Limitations & Deferred Items

**Last updated:** 2026-07-19

This is a plain list of everything that has been consciously **skipped, deferred, or accepted as a
known gap** so far — because none of it was a show-stopper for shipping the milestone it came up in.
Nothing here is a secret bug; every item below is already on record somewhere in the project's own
docs, plans, or memory. This file just collects them in one place instead of leaving them scattered
across plan files and session notes.

If you're deciding whether something is safe to rely on, or wondering why a feature behaves a
certain way, check here first.

## 1. Testing gaps

- **The Viewer App and Web Portal have never actually been launched and clicked through end-to-end
  before this document set.** Every prior milestone (M5 read-only viewer, M6 web mode) explicitly
  deferred "actually running `cargo tauri dev` against the viewer config" and "actually launching
  `cargo run -p pwmgr-web` and browsing it" to "the manual walkthrough after M6" — and that walkthrough
  still hasn't happened. `docs/testing/HOW_TO_TEST_VIEWER_APP.md` and
  `docs/testing/HOW_TO_TEST_WEB_PORTAL.md` are the first real procedures for this; running them is
  still an open action item, not something already confirmed working.
- **No automated UI/visual test tooling exists anywhere in this repo** (no Playwright, no
  `*.test.tsx`, no visual regression suite). Every screen, theme, tooltip, and resize behavior is
  verified by a human following a manual checklist (`docs/testing/`). This was a deliberate,
  repeated decision (TRD §7 explicitly allows "lighter/manual acceptable" for this class of
  behavior), not an oversight — but it does mean UI regressions are only caught when someone
  actually runs the checklist.
- **A two-browser-session HTTP integration test** (proving the web portal's session isolation holds
  over two real concurrent cookie jars, not just two in-process `SessionStore` entries) hasn't been
  written yet. Today's coverage is 3 integration tests: a register→list roundtrip, a CSRF-header
  rejection check, and a missing-cookie 401 check — all single-session.
- **The tooltip-on-a-disabled-control behavior** (M1 Phase B, Task 27.5) is flagged as best-effort
  only — the timing window to observe it is narrow enough that it's "genuinely difficult to observe
  manually," so it has never been conclusively confirmed either way.

## 2. Known behavioral quirks

- **The Health Dashboard's "Weak" classification always scores with zxcvbn**, regardless of what
  you've picked in Settings → "Password strength algorithm." The Settings toggle only changes the
  live meter shown in the Generator and Credential editor — it was never wired into the Health
  Dashboard's batch scoring. This was confirmed intentional after review, not a bug, but it's easy to
  assume the Settings toggle is global when it isn't.
- **A crash or restart of the Web Portal silently logs out every active browser session.** Nothing
  about "who is logged in right now" is persisted — a server restart just means every open tab gets
  a 401 on its next request and has to log in again. Accepted by design (nothing sensitive survives
  a crash that wasn't already durable in the vault/registry files).
- **Web Portal concurrency is accepted at "household scale," not load-tested beyond it.** Each logged
  -in browser session holds its own in-process vault + registry connections for as long as it lives;
  fine for a handful of family members/tabs, never tested past that.

## 3. Deferred code cleanup

- **`AppState::open(paths)` production constructor doesn't exist yet.** The same boilerplate for
  building a locked `AppState` is duplicated three times — `src-tauri/src/lib.rs`,
  `web-src-tauri/src/handlers.rs` (`build_locked_state`), and `viewer-src-tauri/src/lib.rs`. Bundled
  as a fast-follow once all of M6 (both halves) ships.
- **`SessionStore::remove` (web portal) has zero production callers.** There's no logout route wired
  up yet — a decision is still needed on whether to add one or drop the method.

## 4. Not yet built

- **M6's responsive/mobile layout is spec'd and planned but has zero implementation code.** The
  design spec and 8-task plan are approved and committed
  (`docs/superpowers/specs/2026-07-19-m6-responsive-layout-design.md`), but nothing has been
  implemented — as of today the app is desktop-width-only in practice.
- **Automated viewport/screenshot testing** (e.g. Playwright) was evaluated for the responsive-layout
  work and explicitly deferred in favor of the same manual-QA convention used everywhere else in this
  project — tracked as a cross-milestone deferred item, revisit trigger not yet reached.
- **macOS/Linux builds.** Tauri supports this in principle with modest per-OS packaging work, but none
  of it has been done — the app is Windows-only in practice today.
- **Android companion + sync.** Design-only (PRD §9) — sync transport (Bluetooth vs. LAN-discovery vs.
  manual QR/export-import) is an explicitly open decision, not started.

## 5. Deliberate non-goals (not gaps — never in scope)

These are not deferred; they're intentional exclusions from the PRD, listed here so they aren't
mistaken for missing features:

- Cloud sync or hosted accounts of any kind.
- Browser extension / autofill.
- **Recovering a forgotten master password.** Zero-knowledge means this is impossible by design —
  there is nothing server-side to reset, and the vault cannot be decrypted without it.
- Defense against malware/keyloggers running as your own logged-in OS user, physical coercion or
  shoulder-surfing, a compromised OS/firmware, or a full supply-chain/SBOM program (`THREAT_MODEL.md`
  §3, adversaries O1–O4 — accepted risks of running software on a general-purpose OS at all, not
  something a password manager can fix from inside).
