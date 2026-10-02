# Docs index

Map of everything under `docs/`. Start with the guides if you want to *use* the app, `design/` if
you want to understand it, and `testing/` if you want to verify it.

## Guides — using the app (`guides/`)

| Doc | What's in it |
|---|---|
| [`HOW_TO_USE_APP.md`](guides/HOW_TO_USE_APP.md) | **Current**, complete reference — all features, all three interfaces (Classic / New / Make) |
| [`HOW_TO_USE_MAIN_APP.md`](guides/HOW_TO_USE_MAIN_APP.md) | Legacy guide — Classic interface only |
| [`HOW_TO_USE_VIEWER_APP.md`](guides/HOW_TO_USE_VIEWER_APP.md) | Read-only emergency-access Viewer app |
| [`HOW_TO_USE_WEB_PORTAL.md`](guides/HOW_TO_USE_WEB_PORTAL.md) | Browser access on `127.0.0.1` |
| [`USAGE_GUIDE.md`](USAGE_GUIDE.md) | Short pointer page to the guides above |
| [`FRESH_MACHINE_SETUP.md`](FRESH_MACHINE_SETUP.md) | New Windows PC to all three apps running |
| [`PATH_AUDIT.md`](PATH_AUDIT.md) | PATH entries the toolchain needs, and why |

## Design — how it's built (`design/`)

| Doc | What's in it |
|---|---|
| [`PRD.md`](design/PRD.md) | Product requirements (original v1.0) |
| [`TRD.md`](design/TRD.md) | Technical requirements: stack, crypto design, data model |
| [`ARCHITECTURE.md`](design/ARCHITECTURE.md) | System architecture and key data flows |
| [`THREAT_MODEL.md`](design/THREAT_MODEL.md) | Assets, adversaries, mitigations, accepted risks |
| [`SECURITY_CONCERNS.md`](design/SECURITY_CONCERNS.md) | 2026-07-19 security audit; H1 and M1–M3 since closed in Phase 9 |

## Testing — verifying it (`testing/`)

- **How-to-test guides:** `HOW_TO_TEST_MAIN_APP.md`, `HOW_TO_TEST_VIEWER_APP.md`,
  `HOW_TO_TEST_WEB_PORTAL.md`, `HOW_TO_TEST_BIOMETRIC_UNLOCK.md`, `HOW_TO_TEST_SSH_AGENT.md`,
  `HOW_TO_TEST_M6_RESPONSIVE.md`, `PHASE_B_MANUAL_VERIFICATION.md`.
- **Modern UI overhaul UAT:** `UAT_MODERN_UI_OVERHAUL.md` (script), `UAT_USER_TESTING_GUIDE.md`
  (independent run), `UAT_FINDINGS_2026-09-23.md` (findings ledger).

## Plans and specs

- [`Plans/`](Plans/) — current work: Modern UI overhaul plan and progress, UAT plan, UAT remediation
  plan, Wave 0 fix plan, npm vulnerability fix plan.
- [`superpowers/plans/`](superpowers/plans/) and [`superpowers/specs/`](superpowers/specs/) —
  per-milestone implementation plans (M1–M6, npm audit fix) and design specs (M4–M6). Frozen
  records of shipped work.

## Status and limits

| Doc | What's in it |
|---|---|
| [`KNOWN_LIMITATIONS.md`](KNOWN_LIMITATIONS.md) | Deferred items, quirks and accepted gaps |
| [`DEFERRED.md`](DEFERRED.md) | Cross-milestone follow-ups deliberately postponed |
| [`RESUME_HERE.md`](RESUME_HERE.md) | Current session handoff: branch state and what's open |

## Design inputs and research (`research/`)

- [`FIGMA_DESIGN_BRIEF.md`](research/FIGMA_DESIGN_BRIEF.md), [`FIGMA_UI_VS_CURRENT_FEATURES.md`](research/FIGMA_UI_VS_CURRENT_FEATURES.md),
  [`UI_REDESIGN_CAPABILITY_CHECKLIST.md`](research/UI_REDESIGN_CAPABILITY_CHECKLIST.md) — Figma brief and gap analysis.
- [`research/bitwarden/compare-ideas/`](research/bitwarden/compare-ideas/) — Bitwarden comparison research.
- [`research/bitwarden/gui-survey/`](research/bitwarden/gui-survey/) — Bitwarden GUI feature survey and port plans.
- [`make-ui-capture/`](research/make-ui-capture/) — Make UI screenshots (dark, light, mobile).
- [`REPO_VS_LOCAL_COMPARISON_2026-07-19.md`](research/REPO_VS_LOCAL_COMPARISON_2026-07-19.md) — dated snapshot.

## Elsewhere

- `.planning/` — GSD project state, roadmap, requirements, per-phase artifacts.
