# Doc Ingestion Synthesis Summary

Mode: `new`. Classifications consumed: 39 (from `.planning/intel/classifications/*.json`).

## Doc counts by type
- DOC: 31 classifications / 30 unique source paths (`docs/FRESH_MACHINE_SETUP.md` classified twice — see conflicts report INFO)
- SPEC: 7
- PRD: 1
- ADR: 0
- UNKNOWN: 0

## Decisions (decisions.md)
0 locked or proposed decisions recorded — no ADR-type documents were present in this batch. See `decisions.md` for candidate future-ADR content (KDF/encryption choice, 127.0.0.1-only web-mode bind, structurally-enforced read-only viewer) currently living only in SPEC/DOC tier.

## Requirements (requirements.md)
12 requirements extracted, one per PRD user story, all sourced from the single PRD (`docs/PRD.md`):
REQ-master-password-registration, REQ-login-unlock, REQ-vault-crud, REQ-password-generator, REQ-live-search, REQ-health-dashboard, REQ-backup-restore, REQ-csv-export-import, REQ-readonly-viewer, REQ-theming, REQ-tooltips, REQ-web-mode-localhost. No competing-variant conflicts were possible (only one PRD in the batch). PRD §4 Non-Goals recorded separately at the end of the file.

## Constraints (constraints.md)
25 constraint entries extracted from the 7 SPEC documents (`docs/TRD.md`, `docs/THREAT_MODEL.md`, M4 design + M4 plan, M5 design, M6 web-mode design, M6 responsive-layout design). Type breakdown: nfr 12, protocol 4, schema 5, api-contract 4. All 7 SPECs were cross-checked pairwise for contradictory technical assertions (crypto facts, M4 design-vs-plan overlap) — none found.

## Context (context.md)
23 of 30 unique DOC-type documents extracted, topic-keyed. 7 were excluded because they form two cross-reference cycles (see below) — extraction did not proceed on those to avoid synthesis-loop garbage, per the doc-synthesizer's cycle-detection rule.

## Conflicts: 2 blockers, 1 competing variant, 5 auto-resolved/info
- **2 BLOCKERS**: two mutual cross-reference cycles detected in the `cross_refs` graph — Cycle A (`HOW_TO_USE_MAIN_APP.md` / `HOW_TO_USE_VIEWER_APP.md` / `HOW_TO_USE_WEB_PORTAL.md`) and Cycle B (`docs/testing/HOW_TO_TEST_MAIN_APP.md` / `docs/testing/HOW_TO_TEST_VIEWER_APP.md` / `docs/testing/HOW_TO_TEST_WEB_PORTAL.md` / `docs/KNOWN_LIMITATIONS.md`). These 7 documents were withheld from `context.md`. Full detail and remediation path in `.planning/INGEST-CONFLICTS.md`.
- **1 WARNING**: competing DOC-tier guidance on whether to build additional vault item types (Card/Secure Note/Identity) — one source ("Closing," scheduled) vs. two sources (explicitly recommend skipping).
- **5 INFO**: no ADRs present in batch; duplicate classification of FRESH_MACHINE_SETUP.md; redundant-but-consistent npm-audit-fix planning docs; PRD/TRD appear stale relative to later-shipped features (biometric unlock, SSH agent, PIN unlock, TOTP, custom fields, reprompt-on-reveal); M4 design-vs-plan cross-check found consistent.

See `.planning/INGEST-CONFLICTS.md` for full detail on every entry above.

## Pointers
- Decisions: `.planning/intel/decisions.md`
- Requirements: `.planning/intel/requirements.md`
- Constraints: `.planning/intel/constraints.md`
- Context: `.planning/intel/context.md`
- Conflicts report: `.planning/INGEST-CONFLICTS.md`
