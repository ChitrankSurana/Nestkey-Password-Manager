# Roadmap: Local-First Zero-Knowledge Password Manager

## Overview

Phases 1-7 are already built and manually tested — this is a brownfield onboarding, not a
greenfield plan. Phases 1-6 map 1:1 onto the original `docs/PRD.md` milestones M1-M6 (secure
vault core → generator → backup/CSV → health/theming → read-only Viewer → localhost web mode),
each verified via its own manual test checklist under `docs/testing/`. Phase 7 covers two
platform-integration features (Windows Hello biometric unlock, an SSH agent over a Windows named
pipe) that shipped afterward on branch `feature/bitwarden-port-phase1`, ahead of a PRD refresh.
Dates below are inferred from the dated execution-plan filenames in
`docs/superpowers/plans/`, not exact ship timestamps.

The remaining journey is two independent, not-yet-started phases: porting the vault UI to a new
Figma design (adding item types, favorites, folders, and per-item health — Phase 8), and closing
three gaps a security audit found in the already-shipped web mode (Phase 9). Neither depends on
the other; both depend only on the shipped base (Phase 6/7).

## Milestones

- ✅ **v1.0 Core Password Manager** — Phases 1-6 (shipped 2026-07-01 to 2026-07-19)
- ✅ **v1.1 Platform Integrations** — Phase 7 (shipped 2026-09, branch `feature/bitwarden-port-phase1`)
- 📋 **v2.0 New Vault UI & Security Hardening** — Phases 8-9 (planned, not started)

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 1: Secure Vault Foundation** - Register/log in with a master password and manage a searchable, encrypted credential vault
- [x] **Phase 2: Password Generator** - Generate strong passwords by 4 methods with live strength/entropy feedback
- [x] **Phase 3: Data Portability** - Encrypted backup/restore and CSV export/import
- [x] **Phase 4: Health Dashboard, Navigation & Theming** - See vault-wide password health and personalize the app's appearance
- [x] **Phase 5: Read-Only Viewer** - A structurally read-only fallback app for when the main app is unavailable
- [x] **Phase 6: Localhost Web Mode** - Full feature set from a browser tab, bound to 127.0.0.1 only, with a responsive layout
- [x] **Phase 7: Platform Integrations** - Windows Hello biometric unlock and an SSH agent over a named pipe
- [ ] **Phase 8: New Vault UI (Figma Port)** - Item types, favorites, folders, and per-item health in a redesigned vault UI
- [ ] **Phase 9: Web Mode Security Hardening** - Close the file-path, rate-limiting, and header gaps a security audit found in Phase 6

## Phase Details

<details>
<summary>✅ v1.0 Core Password Manager (Phases 1-6) — SHIPPED 2026-07-01 to 2026-07-19</summary>

### Phase 1: Secure Vault Foundation
**Goal**: Users can create an account, securely log in, and manage a searchable credential vault
**Depends on**: Nothing (first phase)
**Requirements**: AUTH-01, VAULT-01, VAULT-02, UX-01
**Success Criteria** (what must be TRUE):
  1. A new user can register a master password once, and it becomes the only key to their vault — there is no recovery path if it's forgotten, by design.
  2. A returning user can log in with their master password and see their saved sites/credentials; a wrong password always returns one generic "incorrect" message with no other information leaked, and a vault file opened outside the app (hex viewer / raw SQLite) shows no readable plaintext.
  3. A user can add, edit, and soft-delete (trash/restore) a site with one or more username/password pairs and free-text remarks.
  4. A user can search the vault in real time by site, username, or remarks with no perceptible lag at hundreds of entries.
  5. Every icon/button has a working tooltip, reachable on both mouse hover and keyboard focus.
**Plans**: N/A — implemented pre-GSD (see `docs/superpowers/plans/2026-07-01-m1-secure-core.md` and `2026-07-05-m1-phase-b.md`; manually verified per `docs/testing/PHASE_B_MANUAL_VERIFICATION.md`)

### Phase 2: Password Generator
**Goal**: Users can generate strong, varied passwords and see their strength before saving
**Depends on**: Phase 1
**Requirements**: GEN-01
**Success Criteria** (what must be TRUE):
  1. A user can generate a password using any of 4 methods (random, memorable, pattern, pronounceable).
  2. A user sees real-time strength/entropy feedback (simple estimator or zxcvbn, per their Settings choice) for a generated or typed password before saving it.
**Plans**: N/A — implemented pre-GSD (see `docs/superpowers/plans/2026-07-12-m2-password-generator.md`)

### Phase 3: Data Portability
**Goal**: Users can back up, restore, and move data in and out of the vault safely
**Depends on**: Phase 2
**Requirements**: DATA-01, DATA-02
**Success Criteria** (what must be TRUE):
  1. A user can create an encrypted backup of the entire vault and restore it in one click, on this machine or another.
  2. A user can export the vault to CSV, encrypted by default, with a plaintext option gated behind typing an explicit confirmation phrase.
  3. A user can import credentials from a Chrome/Edge, Firefox, or LastPass CSV export, or map any other CSV's columns themselves, previewing the result before committing.
**Plans**: N/A — implemented pre-GSD (see `docs/superpowers/plans/2026-07-12-m3-backup-csv.md`)

### Phase 4: Health Dashboard, Navigation & Theming
**Goal**: Users can see the health of their vault at a glance and personalize the app's appearance
**Depends on**: Phase 3
**Requirements**: HEALTH-01, UX-02
**Success Criteria** (what must be TRUE):
  1. A user can view a Health Dashboard showing which passwords are weak, reused, or old, with an opt-in (off by default) breach check against Have I Been Pwned that never sends more than a 5-character hash prefix.
  2. A user can pick a light or dark theme and one of five accent colors, and it stays consistent across every screen.
**Plans**: N/A — implemented pre-GSD (see `docs/superpowers/specs/2026-07-14-m4-health-nav-theming-design.md` and `docs/superpowers/plans/2026-07-14-m4-health-nav-theming.md`; known doc-vs-behavior nuance on "Weak" classification tracked as a todo, not a gap in this phase's delivered scope)
**UI hint**: yes

### Phase 5: Read-Only Viewer
**Goal**: Users have a fallback way to view their vault if the main app is broken or unavailable
**Depends on**: Phase 4
**Requirements**: VIEWER-01
**Success Criteria** (what must be TRUE):
  1. A user can open the separate Viewer app, enter their master password, and see their sites/credentials — search, reveal, and copy all work.
  2. The Viewer binary has no add/edit/delete/generator/health/backup commands compiled into it at all — read-only is a structural, compile-time guarantee, not a UI-level restriction.
**Plans**: N/A — implemented pre-GSD (see `docs/superpowers/specs/2026-07-16-m5-readonly-viewer-design.md` and `docs/superpowers/plans/2026-07-16-m5-readonly-viewer.md`)

### Phase 6: Localhost Web Mode
**Goal**: Users can access their vault from a browser tab without exposing it beyond their own machine
**Depends on**: Phase 5
**Requirements**: WEB-01
**Success Criteria** (what must be TRUE):
  1. A user can open `http://127.0.0.1:7373` in a browser, log in (or register), and use the full feature set against the same shared vault files as the desktop app.
  2. The server never listens on any interface other than `127.0.0.1` — confirmed by `netstat` showing only the loopback binding.
  3. Below a 640px window width, the layout switches to one of three narrow-nav patterns (icon rail, bottom tabs, or drawer) on both the main app and the Viewer; the desktop layout at ≥640px is byte-for-byte unchanged.
**Plans**: N/A — implemented pre-GSD (see `docs/superpowers/specs/2026-07-18-m6-web-mode-design.md`, `2026-07-19-m6-responsive-layout-design.md`, and their matching plan docs)
**UI hint**: yes

</details>

### ✅ v1.1 Platform Integrations (Phase 7) — SHIPPED 2026-09

#### Phase 7: Platform Integrations
**Goal**: Users can unlock the vault with Windows Hello and use vault-held SSH keys from other applications, both opt-in and off by default
**Depends on**: Phase 6
**Requirements**: AUTH-02, INTEG-01
**Success Criteria** (what must be TRUE):
  1. A user can enable Windows Hello biometric unlock in Settings and use it to unlock the vault instead of typing the master password; it defaults off.
  2. A user can enable an SSH agent over a Windows named pipe in Settings so other applications (e.g. OpenSSH, PuTTY) can use Ed25519/RSA keys held in the vault; it defaults off and is desktop-only.
**Plans**: N/A — implemented pre-GSD, no design/plan doc exists for this milestone (verify via `docs/HOW_TO_TEST_BIOMETRIC_UNLOCK.md`, `docs/HOW_TO_TEST_SSH_AGENT.md`, and git log on `feature/bitwarden-port-phase1`; both features are explicitly flagged untested against real-world hardware/clients in `.planning/codebase/CONCERNS.md`)

### 📋 v2.0 New Vault UI & Security Hardening (Phases 8-9) — Planned

**Milestone Goal:** Close the two gaps found during this onboarding pass: the vault UI hasn't
been ported to its new Figma design yet, and a security audit of the already-shipped web mode
found unresolved file-path, rate-limiting, and header gaps.

#### Phase 8: New Vault UI (Figma Port)
**Goal**: Users can organize a richer vault (multiple item types, favorites, folders) and check a single item's health without leaving its detail panel, in a UI ported to the new Figma design
**Depends on**: Phase 7
**Requirements**: VAULT-03, VAULT-04, VAULT-05, HEALTH-02
**Success Criteria** (what must be TRUE):
  1. A user can create/view Login, Secure Note, Payment Card, and Identity items, each with its own icon, color, and detail fields, and filter the sidebar by type with a count badge.
  2. A user can star any item as a favorite and filter to a "Favorites" view showing only starred items.
  3. A user can create folders, assign an item to at most one folder, and filter the vault by folder.
  4. A user can view a per-item health card (strength meter, breach status, reuse status) directly in that item's detail panel, without triggering a full vault-wide health sweep.
**Plans**: 6 plans

Plans:
- [ ] 08-01-PLAN.md — Tracer: new UI entry point boots end to end, and the desktop window is built at runtime from `config.ui_variant`
- [ ] 08-02-PLAN.md — Interface toggle in Settings: setter command, web route, API wrapper, persistent restart banner
- [ ] 08-03-PLAN.md — Three-pane shell, sidebar, searchable site list over the lazy-decryption `useVault` hook
- [ ] 08-04-PLAN.md — Detail panel: credential repeater, reprompt-gated secrets, TOTP, custom fields, per-item strength meter
- [ ] 08-05-PLAN.md — CRUD wiring to the existing editors, five secondary panes, sidebar nav dispatch
- [ ] 08-06-PLAN.md — Three responsive tiers and the standing invariant gate over `src/newui/`

**Scope note (2026-09-13)**: the Goal and Success Criteria above describe the ORIGINAL scope and
are superseded for execution by
`.planning/phases/08-new-vault-ui-figma-port/8-CONTEXT.md` `<domain>` + D-10, which the developer
confirmed this session. Item types, favorites and folders are NOT delivered by this phase (no
backend persistence exists for any of them); per-item health ships as a live strength meter only.
VAULT-03/04/05 stay open after this phase; HEALTH-02 is partially satisfied. The roadmap prose was
deliberately left unedited by developer choice — see `08-01-PLAN.md` → `## Requirement Scope Note`.

**Note**: `docs/FIGMA_UI_VS_CURRENT_FEATURES.md` schedules this exact scope as "Closing," but
Bitwarden-comparison research (`docs/bitwarden_compare_ideas/00-summary.md`) explicitly recommends
against adding new item types and folders as speculative scope with no PRD user story. This
roadmap follows the more specific, more recent Figma-comparison doc per the project's own
onboarding instructions — see PROJECT.md Key Decisions. Confirm this is still the intended scope
before planning this phase.
**UI hint**: yes

#### Phase 9: Web Mode Security Hardening
**Goal**: The localhost web server can no longer be used to read/write arbitrary files, brute-force logins, or exhaust memory, and ships baseline security headers — closing the gaps `SECURITY_CONCERNS.md` found in the already-shipped Phase 6 web mode
**Depends on**: Phase 6 (independent of Phase 8 — can be planned/executed in either order or in parallel)
**Requirements**: SEC-01, SEC-02, SEC-03
**Success Criteria** (what must be TRUE):
  1. Web-mode export/import/restore endpoints reject any client-supplied path that isn't a filename inside the app's own backups/exports directory — no path traversal, no arbitrary file read or write (closes `SECURITY_CONCERNS.md` H1).
  2. Repeated failed login/register attempts against the web server are throttled with backoff/lockout, and Argon2 derivations are bounded so a burst of requests can't exhaust server memory (closes M1/M2).
  3. The web server sends a strict Content-Security-Policy and `X-Content-Type-Options: nosniff` on every response, and the desktop Tauri shell no longer runs with `csp: null` (closes M3).
**Plans**: TBD

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9 (8 and 9 may run in either order once Phase 7 is done)

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|-----------------|--------|-----------|
| 1. Secure Vault Foundation | v1.0 | N/A (pre-GSD) | Complete | 2026-07-05 |
| 2. Password Generator | v1.0 | N/A (pre-GSD) | Complete | 2026-07-12 |
| 3. Data Portability | v1.0 | N/A (pre-GSD) | Complete | 2026-07-12 |
| 4. Health Dashboard, Navigation & Theming | v1.0 | N/A (pre-GSD) | Complete | 2026-07-14 |
| 5. Read-Only Viewer | v1.0 | N/A (pre-GSD) | Complete | 2026-07-16 |
| 6. Localhost Web Mode | v1.0 | N/A (pre-GSD) | Complete | 2026-07-19 |
| 7. Platform Integrations | v1.1 | N/A (pre-GSD) | Complete | 2026-09 (approx.) |
| 8. New Vault UI (Figma Port) | v2.0 | 0/TBD | Not started | - |
| 9. Web Mode Security Hardening | v2.0 | 0/TBD | Not started | - |
