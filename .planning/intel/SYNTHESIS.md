# Doc Ingest Synthesis — Summary

Mode: `new` (no existing `.planning/` context to merge against).
Classifications consumed: 40 JSON files in `.planning/intel/classifications/` → 39 unique source
documents (one duplicate pair for `docs/FRESH_MACHINE_SETUP.md` collapsed — see
`INGEST-CONFLICTS.md`).

## Doc counts by type

| Type | Count |
|---|---|
| DOC  | 31 |
| SPEC | 7  |
| PRD  | 1  |
| ADR  | 0  |
| **Total** | **39** |

## Decisions (decisions.md)

**0 locked decisions, 0 ADR entries.** No ADR-type documents exist anywhere in this ingest set.
`decisions.md` explains this explicitly and points to where the equivalent narrative decisions
actually live (PRD.md, TRD.md, THREAT_MODEL.md, the four milestone `*-design.md` SPECs,
ARCHITECTURE.md) so the roadmapper isn't left guessing whether this is an omission.

## Requirements (requirements.md)

**13 requirements** extracted from the single PRD source (`docs/PRD.md`), one per PRD user story
plus the two "Future Scope" items (Android companion, macOS/Linux builds): REQ-master-password-
registration-login, REQ-vault-crud, REQ-password-generator, REQ-live-search, REQ-health-dashboard,
REQ-backup-restore, REQ-csv-export-import, REQ-readonly-viewer, REQ-theming, REQ-tooltips,
REQ-localhost-web-mode, REQ-android-companion-future, REQ-macos-linux-builds-future. PRD §4
Non-Goals are recorded separately at the bottom of the file for traceability (not as requirements).
No competing PRD acceptance-criteria variants — only one PRD document was in this ingest.

## Constraints (constraints.md)

**20 constraint entries** extracted from the 7 SPEC-type documents (`docs/TRD.md`,
`docs/THREAT_MODEL.md`, and the M4/M5/M6 `*-design.md` specs plus the M4 implementation plan, which
the upstream classifier tagged SPEC rather than DOC). Breakdown by type: 9 `nfr`, 6 `api-contract`,
4 `protocol`, 1 `schema`. One entry pair carries a flagged WARNING (see Conflicts below) — both
variants are preserved verbatim rather than merged.

## Context (context.md)

**31 DOC-type documents**, organized into 9 topic groups: Architecture; Usage Guides (3 apps);
Testing Guides (7 manual QA checklists); Known Limitations & Deferred Items; Figma UI vs. current
feature set (planned new-UI port); Setup/Environment (fresh-machine setup, PATH audit); Repo/Branch
Hygiene (point-in-time git snapshot, historical only); Bitwarden Comparison Research (4 docs,
research-only, no code changed — ranked adoption ideas, explicitly-not-recommended items, and
"already better" comparisons); Milestone Implementation Plans (7 runbooks for M1-M6, each with
source-attributed judgment-call/deviation notes where the plan diverged from its design spec); and
the vitest/npm audit fix (2 redundant, consistent documents, captured once).

## Conflicts

- **0 BLOCKERS** — no LOCKED-vs-LOCKED contradictions (no ADRs exist), no cross-reference cycles
  (graph walked, clean), no UNKNOWN/low-confidence classifications.
- **1 WARNING** — competing SPEC-vs-SPEC(-plan) variants on whether the Health Dashboard's "Weak"
  classification respects the user's `strength_algorithm` Settings toggle. Both variants preserved
  in `constraints.md`; needs an explicit user decision (amend the design doc, or change the shipped
  behavior) before downstream planning treats either as settled.
- **6 INFO** — transparency notes: the duplicate-classification collapse; the "0 ADRs" observation;
  PRD.md being stale relative to shipped/planned features (biometric unlock, SSH agent, new item
  types from the Figma comparison); a clean cross-reference cycle check; redundant (non-conflicting)
  npm-audit-fix coverage; and the open (non-contradictory) unlock-throttling gap flagged by the
  Bitwarden comparison research.

Full detail: `.planning/INGEST-CONFLICTS.md`.

## For the roadmapper

Read, in order: this file → `decisions.md` (empty, explained) → `requirements.md` →
`constraints.md` → `context.md` → `.planning/INGEST-CONFLICTS.md` for the one WARNING needing a
decision before it's baked into a roadmap. Recommended first action given the INFO notes: consider
whether `docs/PRD.md` needs a refresh pass (new item types, biometric unlock, SSH agent) before using
it as the sole source of truth for feature scope — none of this blocks planning, but planning on
PRD.md alone will under-represent the project's actual current surface.
