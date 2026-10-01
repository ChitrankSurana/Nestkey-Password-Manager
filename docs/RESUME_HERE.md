# Resume Notes — Password Manager — Make UI + UAT remediation in progress

**Updated:** 2026-09-30. **Branch:** `feature/modern-ui-overhaul` (HEAD `23f6fc3`). Always
`git fetch` and compare against `origin/main` before trusting this snapshot.

## Where everything stands

- **`main`:** M1-M6 (web mode + responsive layout), Phase 7 (biometric/SSH agent), Phase 8 (New
  vault UI) and Phase 9 (web-mode security hardening) are complete.
- **`feature/modern-ui-overhaul`:** adds the **Make UI** (`ui_variant: "make"`, `src/makeui/`),
  item metadata, secret-content search, password history, master-password change, and the UAT
  remediation waves:
  - Waves 1-3 (blockers, majors, minors) fixed and committed.
  - Wave 4: full-fidelity JSON export/import, per-credential change log, MakeShell restructure
    (B1 Option B), Viewer UI port (N16) — all committed.
  - Also: single-instance guard + WebView2 black-screen mitigation, Make UI copy-button
    unification, devtools no longer auto-open.
- **Not yet merged to `main`.** Working tree: `core/src/session.rs` modified (uncommitted),
  `docs/Plans/a` untracked.

## Still open

- Wave 0 live retests (R1-R5, vault-data-loss, blank-window launch, N15) — need a running app.
- Deferred web-mode pass (B2, J7, N11, N14 + a full web re-run) and a fresh UAT script.
- Phase 9 human UAT items (production-CSP browser sweep) from `09-VERIFICATION.md`.

## Read first

1. `docs/Plans/UAT_REMEDIATION_PLAN_2026-09-27.md` — triage, rulings, Wave 4 notes.
2. `.planning/.continue-here.md` — blocking constraints (e.g. `reprompt_exempt`; never reuse the old
   `reprompt` column).
3. `docs/guides/HOW_TO_USE_APP.md` — current feature reference.
