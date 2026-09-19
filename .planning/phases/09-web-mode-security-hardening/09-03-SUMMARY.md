---
phase: 09-web-mode-security-hardening
plan: 03
subsystem: api
tags: [axum, rust, typescript, backup-restore, upload-cache, sec-01]
requires:
  - "UploadCache Backup variant + set_upload/backup_upload/clear_upload (09-02)"
  - "httpUpload transport helper (09-02)"
provides:
  - "do_restore_backup_from_bytes path-free core restore; do_restore_backup is a thin wrapper"
  - "POST /api/upload_backup and restore_backup reading the session cache"
  - "api.uploadBackup and isTauri-branched api.restoreBackup; Backup.tsx web file-input branch"
  - "Standing SEC-01 closure test across all four H1 handlers"
affects: [09-04, 09-05]
status: complete
commits: 1
plan_head_before: 8ca32e6353b86cd1e4c7c8992f122dbadda5c88e
actuals:
  tokens: 5500
  tasks: 2
  commits: 1
key-files:
  modified:
    - core/src/commands.rs
    - web-src-tauri/src/handlers.rs
    - web-src-tauri/tests/http_integration.rs
    - src/lib/api.ts
    - src/screens/Backup.tsx
decisions:
  - "Upload cache survives confirm_required and is cleared only on restore success, so the overwrite confirmation needs one upload"
metrics:
  completed: 2026-09-19
---

# Phase 9 Plan 03: Web-mode backup restore from browser upload Summary

Web-mode `restore_backup` no longer accepts a client path: the browser uploads the `.bak` via `/api/upload_backup` and the server restores from the session-cached bytes through the unchanged atomic restore sequence, closing SEC-01 for all four H1 handlers.

## Tasks

1. Task 1 (tracer) and Task 2 landed in one commit `f55e27b`. Both tasks edit `http_integration.rs`, and the closure and confirm-round-trip tests were written together with the Task 1 tests, so they were not split.
2. Verification: `cargo test --workspace` (core 329, web 17 unit + 32 integration), `npm run build`, `npm test` (40) all green. `src-tauri/src/commands.rs` untouched (D-02).

## Deviations from Plan

**[Rule 1 - Test design] Integration tests adjusted for real behavior.** On Windows an open vault file cannot be deleted or renamed over, and a successful restore over the acting user's own open vault locks that session (existing `do_restore_backup` behavior), so a follow-up call returns 401. Tests log alice out first and restore as a second logged-in user (bob), which keeps the acting session unlocked so the cache-cleared assertion (400 `invalid_input`) is observable. No production code impact.

Line endings: touched files are CRLF; edits kept the repo convention (git normalizes).

## Known Stubs

None.

## Threat Flags

None. New `/api/upload_backup` route is inside the authenticated router with the same body limit as `/api/upload_csv` (T-09-16).

## Self-Check: PASSED
