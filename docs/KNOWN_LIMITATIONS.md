# Known Limitations & Deferred Items

**Last updated:** 2026-09-30

This is a plain list of everything that has been consciously **skipped, deferred, or accepted as a
known gap** so far — because none of it was a show-stopper for shipping the milestone it came up in.
Nothing here is a secret bug; every item below is already on record somewhere in the project's own
docs, plans, or memory. This file just collects them in one place instead of leaving them scattered
across plan files and session notes.

If you're deciding whether something is safe to rely on, or wondering why a feature behaves a
certain way, check here first.

## 1. Testing gaps

- **The web-mode pass of the UAT is still deferred.** A desktop UAT (Phases A and B) ran in
  September 2026 and drove the remediation work, but the deferred web-mode pass (B2, J7, N11, N14
  plus a full web re-run) is still open — see `docs/Plans/UAT_REMEDIATION_PLAN_2026-09-27.md`.
- **No automated browser/UI/visual test tooling exists anywhere in this repo** (no Playwright, no
  visual regression suite). Frontend unit tests do exist (`vitest`, `src/**/*.test.ts`, covering
  helpers such as card, clipboard, mask, totp and reprompt policy), but no component/screen tests.
  Every screen, theme, tooltip, and resize behavior is verified by a human following a manual
  checklist (`docs/testing/`). This was a deliberate,
  repeated decision (TRD §7 explicitly allows "lighter/manual acceptable" for this class of
  behavior), not an oversight — but it does mean UI regressions are only caught when someone
  actually runs the checklist.
- *(Resolved)* A two-browser-session HTTP integration test now exists
  (`two_browser_sessions_over_real_connections_stay_isolated`).
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

- *(Resolved)* The shared `AppState::open(paths)` constructor now exists in `core/src/state.rs`, and
  web mode has a `POST /api/logout` route that removes the session (no visible "Log out" button
  in the UI yet).

## 4. Not yet built

- **Automated viewport/screenshot testing** (e.g. Playwright) remains deferred in favor of manual QA.
  Rust (`cargo test --workspace`) and frontend Vitest suites exist and are run before each commit.
- **Open UAT items:** Wave 0 live retests and the deferred web-mode pass (B2, J7, N11, N14) — see
  `docs/Plans/UAT_REMEDIATION_PLAN_2026-09-27.md`.
- **CSV/Viewer support for new item types** (secure note, card, identity): CSV does not carry them
  (use JSON export/import instead); Viewer support for them is deferred.
- **macOS/Linux builds.** Windows-only in practice today.
- **Android companion + sync.** Design-only (PRD §9).

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
