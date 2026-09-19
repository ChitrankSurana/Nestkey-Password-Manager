---
status: complete
phase: 09-web-mode-security-hardening
source: 09-01-SUMMARY.md, 09-02-SUMMARY.md, 09-03-SUMMARY.md, 09-04-SUMMARY.md, 09-05-SUMMARY.md
started: 2026-09-19T00:00:00Z
updated: 2026-09-19T00:00:00Z
---

## Current Test

[testing complete]

## Tests

### 1. Web mode CSP sweep (browser)
expected: Zero CSP violations across all screens; icons, meter bar, theme work.
result: pass

### 2. Response headers present
expected: DevTools Network, any response shows content-security-policy, x-content-type-options: nosniff, referrer-policy, x-frame-options: DENY.
result: skipped
reason: "User could not locate headers in DevTools; deferred. Server-side check via curl showed all 4 headers on GET /, POST /api/get_config, 401 and 403 responses (2026-09-19). Revisit if a real issue appears."

### 3. Web export download and import/restore upload
expected: Export CSV downloads a file; CSV import via file picker previews and commits; backup restore via file picker works (with overwrite confirm).
result: pass
note: "Passed after in-session fixes (master password on export, preview toggle/commit/scroll/live mapping, restore stays on screen, save-location picker, Back/Close styling). Final two fixes accepted by user without re-check."

### 4. Desktop shell under strict CSP
expected: `npm run tauri:dev` window loads, hot reload works, no console CSP violations; the `npm run tauri:build` binary launches and every screen works.
result: pass
note: "User passed 'for now'; no CSP violations reported."

## Summary

total: 4
passed: 3
issues: 0
pending: 0
skipped: 1
blocked: 0

## Gaps

[none yet]

## Deferred Follow-Ups

- test: 1
  idea: "Backup screen should let the user choose the backup location when taking a backup"
  deferred_at: 2026-09-19
- test: 1
  idea: "Independent scrollbars for each pane (credential list, detail, settings/dashboard). Implemented same day in NewShell.tsx, AppShell.tsx, ItemList.tsx (h-screen root, min-h-0 + overflow-y-auto panes); needs visual confirm."
  deferred_at: 2026-09-19
- test: 2
  idea: "Web: on 401 locked from a protected call, go back to login screen instead of showing 'Vault is locked' message. Implemented same day (api.ts SESSION_LOCKED_EVENT + App.tsx listener); needs confirm."
  deferred_at: 2026-09-19
- test: 3
  idea: "Implemented same day, need re-check: (1) Export CSV requires master password in all modes, web and desktop (CsvExport.tsx, client-side verify via verify_master_password); (2) Preview button toggles Preview/Hide Preview; (3) Commit Import button beside Preview, visible once a file is uploaded (and, for Other, a column mapped); (4) Restore stays on Backup screen showing success and a Go to login button - the lock itself is by design because restore replaces the open vault file and key, so the redirect is held, not removed."
  deferred_at: 2026-09-19
- test: 3
  idea: "Implemented same day, need re-check: (a) Vault list header+search sticky (old UI Vault.tsx); (b) web Create Backup now downloads via new /api/create_backup_download with a Save-as picker (Chromium showSaveFilePicker; other browsers fall back to plain download); desktop already used native save dialog; (c) CSV import preview in its own max-h scroll box with sticky header; (d) with preview open, changing a column mapping re-parses live and keeps preview open."
  deferred_at: 2026-09-19
- test: 3
  idea: "Save-as picker now opens first (before the backup is produced); status reports saved vs browser-download; Back/Close buttons restyled to the accent button style."
  deferred_at: 2026-09-19
