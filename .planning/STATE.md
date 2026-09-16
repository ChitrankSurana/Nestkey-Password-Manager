---
gsd_state_version: "1.0"
milestone: v2.0
milestone_name: New Vault UI & Security Hardening (Phases 8-9) — Planned
current_phase: 08
current_phase_name: New Vault UI (Figma Port)
status: executing
stopped_at: Completed 08-10-PLAN.md
last_updated: "2026-09-16T17:42:43.557Z"
last_activity: 2026-09-16
last_activity_desc: Phase 08 execution started
state_head: 826ae4b1828871482691ef05eef94188c9368c68
progress:
  total_phases: 2
  completed_phases: 7
  total_plans: 12
  completed_plans: 10
  percent: 83
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-09-13)

**Core value:** The vault must stay unreadable to anyone without the master password —
zero-knowledge and local-only by default, with no path to recover a forgotten master password.
**Current focus:** Phase 08 — New Vault UI (Figma Port)

## Current Position

Phase: 08 (New Vault UI (Figma Port)) — EXECUTING
Plan: 5 of 12
Status: Ready to execute
Last activity: 2026-09-16 — Phase 08 execution started
PROJECT.md/REQUIREMENTS.md/ROADMAP.md/STATE.md. Phases 1-7 (PRD milestones M1-M6 plus biometric
unlock/SSH agent) retroactively marked Complete based on shipped code, git history, and manual
test checklists — none were executed through GSD plan-phase/execute-phase.

Progress: [████████░░] 83% (7/9 phases complete; plan-level tracking not applicable to
pre-GSD phases)

## Performance Metrics

**Velocity:**

- Total plans completed: 0 (no phase has been executed through GSD's plan-phase/execute-phase yet)
- Average duration: N/A
- Total execution time: N/A

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 1-7 (pre-GSD) | N/A | N/A | N/A |
| 8-9 | 0 | - | - |

**Recent Trend:**

- Last 5 plans: N/A — no GSD execution history yet
- Trend: N/A

*Updated after each plan completion*
**Per-Plan Metrics:**

| Plan | Duration | Tasks | Files |
|------|----------|-------|-------|
| Phase 08 P01 | 47min | 3 tasks | 11 files |
| Phase 08 P02 | 20min | 2 tasks | 3 files |
| Phase 08 P03 | ~24min | 3 tasks | 10 files |
| Phase 08 P04 | ~20min | 3 tasks | 7 files |
| Phase 08 P05 | ~8min | 3 tasks | 8 files |
| Phase 08 P07 | ~15min | 2 tasks | 1 files |
| Phase 08 P08 | 12min | 2 tasks | 5 files |
| Phase 08 P09 | ~25min | 4 tasks | 9 files |
| Phase 08 P10 | ~15min | 3 tasks | 7 files |

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- [Phase 8]: Figma-scheduled item-type/folder expansion (`docs/FIGMA_UI_VS_CURRENT_FEATURES.md`)
  kept in v1 scope despite Bitwarden-comparison research recommending against it — flagged
  "⚠️ Revisit" in PROJECT.md, confirm before planning Phase 8.
- [Phase 4, historical]: M4 Health Dashboard "Weak" classification always uses zxcvbn regardless
  of the `strength_algorithm` Settings toggle — confirmed intentional; doc-vs-behavior mismatch
  tracked as a todo, not a functional defect.
- [Phase 08]: Checkpoint decision: window-failure recovery is 'asset-guard, extended' (Rust asset-guard + frontend runtime-failure escape hatch), keybind-escape ruled out — Coordinator resolved 08-01 Task 2 checkpoint mid-execution: no recovery path may require manual config-file editing
- [Phase 08]: Phase 08 Plan 02: dropped git stash@{0} (pre-existing ui_variant scaffolding) as fully superseded by 08-01's own implementation
- [Phase 08]: Phase 08 Plan 02: Task 1 scoped to only the web-portal /api/update_ui_variant route, since 08-01 already shipped the desktop Tauri command ahead of schedule
- [Phase 08]: Phase 08 Plan 03: Sidebar logo chip uses --accent-600 ramp step instead of the literal --accent token, keeping --accent exclusively reserved for CTAs/selected-state tints
- [Phase 08]: Phase 08 Plan 03: NewShell's five non-vault sidebar destinations (health/generator/trash/data/settings) have no-op onClick handlers - their secondary panes land in a later plan, out of 08-03's file list
- [Phase 08]: Phase 08 Plan 04: DetailPanel/CredentialBlock's per-credential expand state and the group-level Other logins toggle are two independent, unpersisted pieces of local UI state, so collapsed credentials stay fully unmounted even after the group opens
- [Phase 08]: Phase 08 Plan 04: NewShell's screen state slice is wired to real setScreen calls on all six nav destinations (not left as no-ops) since strict noUnusedLocals would flag an unused setter - only the vault branch renders content until plan 08-05
- [Phase 08]: Phase 08 Plan 05: ItemList.tsx needed no code change - its onAddSite wiring already raised the shell's addingSite flag as of plan 08-03
- [Phase 08]: Phase 08 Plan 05: CredentialBlock gained its own local error state to surface a failed softDeleteCredential call, using the same isCommandError/generic-fallback convention used elsewhere
- [Phase 08]: Phase 08 Plan 05: Edit/Delete/onOpenHealth callbacks threaded through DetailPanel as props rather than CredentialBlock reaching into shell state or importing editors directly
- [Phase 08]: Phase 08 Plan 07: Used #[serde(alias = "field_type")] rather than a one-time data migration - the encrypted blob is opaque at rest, so accepting both spellings on read is simpler and safer than trying to migrate ciphertext
- [Phase 08]: Phase 08 Plan 08: Spinner routes the bootstrap-icons glyph through the existing Icon component (not an inline <i class="bi bi-...">), preserving the project's single-choke-point rule for the icon font
- [Phase 08]: [Phase 08]: Phase 08 Plan 09: Task 1 checkpoint:decision auto-resolved to Option A (restoring a cascade-deleted credential also revives its site) per auto-mode and the already-locked 8-CONTEXT.md product decision - no live checkpoint stop occurred
- [Phase 08]: [Phase 08]: Phase 08 Plan 09: soft_delete_site cascades to live credentials inside one unchecked_transaction and do_soft_delete_site's refuse-outright guard was removed - a site with credentials now deletes behind a frontend confirmation instead of a hard backend refusal
- [Phase 08]: [Phase 08]: Phase 08 Plan 09: new src/lib/confirm.ts is the sanctioned cross-platform confirm seam (native plugin-dialog on desktop, window.confirm in web mode) for any future destructive-action confirmation in either UI
- [Phase 08]: [Phase 08]: Phase 08 Plan 10: Task 1 (tracer, tdd=true) executed as real production-quality code from the start, per plan convention - nothing throwaway written or discarded
- [Phase 08]: [Phase 08]: Phase 08 Plan 10: web-src-tauri create_backup handler stays request-body-free rather than adding an ignored destinationPath field, to avoid misleadingly suggesting the web route accepts a destination
- [Phase 08]: [Phase 08]: Phase 08 Plan 10: G-08-1 closed - destination path threaded through do_create_backup/src-tauri create_backup/Backup.tsx save() dialog on desktop only; web create_backup provably still passes None (grep gate + HTTP integration test)

### Pending Todos

1 pending todo — see `.planning/todos/pending/`:

- `2026-09-13-resolve-m4-health-dashboard-weak-classification-doc-conflict.md` (cosmetic,
  doc-vs-shipped-behavior mismatch, not blocking any phase)

### Blockers/Concerns

- [Phase 9 candidate]: `SECURITY_CONCERNS.md` (audit dated 2026-07-19) flags H1 — web-mode
  arbitrary file read/write via client-supplied paths — as the highest-severity open item, and it
  guards the project's own zero-knowledge/local-only promise. Not yet fixed.
- [Pre-Phase-9]: Test-coverage claims are unreconciled between sources. Ingested M1-M6 plan docs
  report an escalating Rust `cargo test --workspace` count (up to 243 tests, TDD-driven).
  `.planning/codebase/CONCERNS.md` / `TESTING.md` (fresh static scan, 2026-09-13) describe only
  the frontend TypeScript/Vitest suite (2 test files) and flag auth/crypto/vault as untested —
  they do not appear to have audited `core/src/*.rs` test modules. Run `cargo test --workspace`
  directly to confirm actual coverage before planning Phase 9.
- Phase 08 Plan 04 execution: repo HEAD was found checked out to main (eaa441b) mid-session instead of feature/bitwarden-port-phase1 (git reflog confirmed an external checkout, cause unknown/outside executor visibility). Working tree had no uncommitted tracked changes at the time, so recovery via 'git checkout feature/bitwarden-port-phase1' was safe and lossless. Worth investigating what triggered the checkout if it recurs.

## Deferred Items

Items acknowledged and deferred at milestone close, most recent first:

| Category | Item | Status | Deferred At | Milestone |
|----------|------|--------|-------------|-----------|
| Scope | CSV import/export for new item types (Secure Note/Payment Card/Identity) | Deferred | 2026-09-13 | v2.0 (post-Phase 8) |
| Scope | Read-only Viewer support for new item types | Deferred | 2026-09-13 | v2.0 (post-Phase 8) |
| Scope | Android companion app + sync design | Deferred | 2026-09-13 | Future (FUTURE-01) |
| Scope | macOS/Linux builds | Deferred | 2026-09-13 | Future (FUTURE-02) |
| Design | Single fixed accent color (Figma) vs. existing 5-color picker | Open question | 2026-09-13 | v2.0 (Phase 8) |

## Session Continuity

Last session: 2026-09-16T17:42:43.504Z
Stopped at: Completed 08-10-PLAN.md
(`.planning/intel/SYNTHESIS.md`) plus the existing codebase map (`.planning/codebase/`); ready for
the orchestrator to present the roadmap for approval and finalize the ingest workflow.
Resume file: None
workflow, not phase execution; safe to remove once this handoff is reviewed)
