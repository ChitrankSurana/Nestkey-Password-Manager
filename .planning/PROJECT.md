# Local-First Zero-Knowledge Password Manager

## What This Is

A local-first, zero-knowledge password manager for Windows: a Tauri v2 + React desktop app backed
by a shared Rust core (`pwmgr-core`), storing an AES-256-GCM + SQLCipher encrypted vault unlockable
by master password, PIN, or Windows Hello. It ships alongside a compile-time read-only Viewer app
(for when the main app is broken) and an optional web mode hard-bound to `127.0.0.1`. Built for one
developer who wants Bitwarden-grade vault mechanics with nothing ever leaving their own machine.

## Core Value

The vault must stay unreadable to anyone without the master password — zero-knowledge and
local-only by default, with no path to recover a forgotten master password because there is
deliberately nothing server-side to reset it from.

## Requirements

### Validated

<!-- Shipped and confirmed working, per git history, manual test checklists, and the codebase map
dated 2026-09-13. See REQUIREMENTS.md for the full checklist with REQ-IDs. -->

- ✓ Master password registration/login, encrypted SQLCipher + AES-256-GCM vault, CRUD, soft-delete
  trash, live search, keyboard-accessible tooltips — Phase 1 (M1)
- ✓ Password generator (4 methods) with real-time strength/entropy — Phase 2 (M2)
- ✓ Encrypted backup/restore, CSV export (encrypted default / confirmed-plaintext) and CSV import
  (Chromium/Firefox/LastPass/generic mapper) — Phase 3 (M3)
- ✓ Health Dashboard (weak/reused/old + opt-in HIBP breach check), light/dark theme + 5 accent
  colors — Phase 4 (M4)
- ✓ Read-only Viewer app, structurally read-only (no write commands compiled in) — Phase 5 (M5)
- ✓ Localhost-only web mode (`127.0.0.1:7373`) with full command parity + 640px responsive layout —
  Phase 6 (M6)
- ✓ Windows Hello biometric unlock (opt-in) and an SSH agent over a Windows named pipe (opt-in,
  desktop-only) — Phase 7 (shipped on `feature/bitwarden-port-phase1`, post-dates the original PRD)

### Active

<!-- Current scope. Not yet started as of this writing. -->

- [ ] New vault UI port (Figma design): item types (Login/Secure Note/Payment Card/Identity),
  favorites, folders, per-item health card — Phase 8
- [ ] Security hardening of the already-shipped web mode: file-path confinement, login
  rate-limiting + bounded Argon2 concurrency, CSP/security headers — Phase 9

### Out of Scope

- Cloud sync or hosted accounts of any kind — breaks the zero-knowledge, local-only core value
  (PRD §4 Non-Goal)
- Browser extension / autofill — not requested; may be reconsidered later (PRD §4 Non-Goal)
- Recovering a forgotten master password — zero-knowledge means there is nothing to reset, by
  design (PRD §4 Non-Goal)
- CSV import/export for Secure Note/Payment Card/Identity items — deferred until the new item
  types settle (FIGMA_UI_VS_CURRENT_FEATURES.md, deferred item #2)
- Read-only Viewer support for the new item types — separate future work
  (FIGMA_UI_VS_CURRENT_FEATURES.md, deferred item #3)
- Standalone CLI binary — new attack surface, no user story (Bitwarden-comparison research,
  explicitly not recommended)
- Folders/collections/org-sharing beyond the single-folder-per-item model Phase 8 schedules —
  speculative scope (Bitwarden-comparison research, explicitly not recommended)
- PBKDF2 fallback / dual-KDF / RSA org-key wrapping — solves problems this project doesn't have
  (Bitwarden-comparison research, explicitly not recommended)
- Android companion app build, macOS/Linux builds — designed/scoped but not started; tracked as
  v2 requirements (FUTURE-01, FUTURE-02) in REQUIREMENTS.md, not in the active roadmap

## Context

**Brownfield onboarding (2026-09-13):** this project was NOT scaffolded via GSD from scratch. M1-M6
of `docs/PRD.md` (Draft v1.0, 2026-07-01) were already implemented and manually tested before this
`.planning/` setup existed, per `docs/superpowers/plans/*` execution runbooks and
`docs/testing/*` checklists. Two more features — Windows Hello biometric unlock and an SSH agent
over a named pipe — shipped afterward on branch `feature/bitwarden-port-phase1`, ahead of the PRD
being updated to mention them. This ROADMAP.md and REQUIREMENTS.md therefore document Phases 1-7
as already complete (see codebase map at `.planning/codebase/`, especially `ARCHITECTURE.md` and
`STRUCTURE.md`, for what actually exists on disk today) rather than re-planning shipped work.

**Developer-facing success metric (as originally scoped):** all PRD milestones M1-M6 shipped and
manually tested. This is already true. Remaining active scope (Phases 8-9) extends past the
original PRD into a Figma-driven UI port and a security-hardening pass, both surfaced during this
onboarding rather than in the original PRD.

**Where the forward scope came from:**
- Phase 8 traces to `docs/FIGMA_UI_VS_CURRENT_FEATURES.md`, which compares a Figma mockup against
  the shipped app and marks item types/favorites/folders/per-item health as "Closing — scheduled."
- Phase 9 traces to `SECURITY_CONCERNS.md` (audit dated 2026-07-19), which found that M6 web mode
  exposes desktop-oriented file-path handling over HTTP unvalidated (H1, HIGH severity), plus
  missing rate-limiting (M1) and security headers (M3).

**Known limitations surfaced during onboarding (not blockers):**
- No CI pipeline, no automated browser/UI test tooling (`docs/DEFERRED.md`), no database migration
  system (`.planning/codebase/CONCERNS.md`).
- The Health Dashboard's "Weak" classification always uses zxcvbn and does not respect the user's
  `strength_algorithm` Settings toggle, contradicting one design doc's wording but confirmed
  intentional per `docs/KNOWN_LIMITATIONS.md`. Tracked as a doc-only cleanup todo, not a defect:
  `.planning/todos/pending/2026-09-13-resolve-m4-health-dashboard-weak-classification-doc-conflict.md`.
- Rust-side (`cargo test --workspace`) test counts reported in the ingested M1-M6 plan docs (up to
  243 tests) could not be cross-checked against the fresh codebase map, whose `CONCERNS.md` and
  `TESTING.md` describe only the frontend TypeScript/Vitest suite (2 test files) and flag auth,
  crypto, and vault operations as untested. These two views were generated from different sources
  (doc ingest vs. static code scan) and were not reconciled by this roadmapping pass — worth a
  direct `cargo test --workspace` run before Phase 9 is planned.

## Constraints

- **Security**: Zero-knowledge — master password and derived vault key are never persisted,
  logged, or transmitted (`docs/TRD.md` NFR-2).
- **Security**: Local-only by default — no network calls except the explicit opt-in HIBP breach
  check, which only ever sends a 5-character SHA-1 hash prefix (`docs/TRD.md` NFR-1;
  `docs/THREAT_MODEL.md`).
- **Performance**: Vault unlock ≤ 1s on typical hardware; search results update within one UI
  frame of each keystroke for vaults up to ~5,000 entries (`docs/TRD.md` NFR-3).
- **Platform**: Windows 10/11 is the primary and only currently-built target; core Rust modules
  must stay portable (no Windows-only assumptions) so macOS/Linux/Android doors stay open
  (`docs/TRD.md` NFR-5).
- **Tech stack**: Tauri v2 + React 19 + TypeScript frontend; Rust 2021 core (`pwmgr-core`) shared
  across three binaries (`src-tauri`, `viewer-src-tauri`, `web-src-tauri`); SQLCipher (AES-256) +
  per-field AES-256-GCM + Argon2id.
- **Testing bar** (target, per `docs/TRD.md` Testing Requirements Bar — see Context note above on
  unreconciled current coverage): crypto round-trip, wrong-password rejection, tamper detection,
  zeroization, CRUD, search, and CSV round-trip must be automated; UI/theme/tooltip/responsive
  checks stay manual-checklist by deliberate choice (no browser/UI test tooling in this project).

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Zero-knowledge, master-password-only vault key, no recovery path | Core security promise since M1 (PRD §4 Non-Goal) | ✓ Good |
| One shared Rust core (`pwmgr-core`) reused by desktop, Viewer, and web binaries instead of 3 separate implementations | Keeps crypto/vault logic in exactly one place; Viewer's read-only guarantee becomes structural (compile-time), not a UI restriction | ✓ Good |
| Web mode hard-bound to `127.0.0.1`, not a config option | Prevents accidentally exposing a zero-knowledge vault to the LAN | ✓ Good |
| M4 Health Dashboard "Weak" always classifies via zxcvbn, ignoring the `strength_algorithm` Settings toggle | No Rust port of the "simple" entropy formula exists; corroborated as intentional by `KNOWN_LIMITATIONS.md`, `HOW_TO_USE_MAIN_APP.md`, and the M4 test checklist | ⚠️ Revisit — doc-vs-shipped-behavior mismatch tracked as a todo, not a functional defect |
| Phase 8 (new-UI port) keeps item types + folders in scope, despite Bitwarden-comparison research explicitly recommending against both (speculative scope, no PRD user story) | `docs/FIGMA_UI_VS_CURRENT_FEATURES.md` already scheduled this exact scope as "Closing"; the Bitwarden-comparison docs are explicitly research-only/advisory, not a locked project decision | ⚠️ Revisit — worth an explicit go/no-go before Phase 8 planning starts |
| No ADR-type documents exist for this project; architectural decisions live narratively inside `docs/PRD.md` / `docs/TRD.md` / `docs/THREAT_MODEL.md` / the M4-M6 design specs | 0 of 39 ingested docs were classified `type: ADR` (doc-ingest run, 2026-09-13) | — Pending — formalize as ADRs later if desired |

---
*Last updated: 2026-09-13 after brownfield doc-ingest and initial roadmap creation*
