---
status: diagnosed
trigger: "UAT gap G-08-1 (phase 08-new-vault-ui-figma-port, Test 1): 'Need option to choose backup location when clicking create backup' — no native save-location dialog appeared when creating a backup from Data -> Backup in the New UI."
created: 2026-09-16T15:43:18Z
updated: 2026-09-16T15:43:18Z
---

## Current Focus

hypothesis: CONFIRMED — see Resolution below.
test: n/a (diagnose-only session, goal: find_root_cause_only)
expecting: n/a
next_action: none — root cause confirmed with direct evidence; return diagnosis to caller. Do NOT proceed to fix_and_verify (out of scope for this session).

## Symptoms

expected: "Clicking Data -> Backup opens a native OS dialog letting the user pick the backup file's save location" (from UAT Test 1's expected text and Gap G-08-1's `truth` field, which frames this as matching pre-existing Classic UI behavior).
actual: User reported: "Need option to choose backup location when clicking 'create backup'" — no location picker appeared; backup is saved to a fixed default location instead (`~/.pwmgr/backups/{username}-{timestamp}.bak`).
errors: None reported.
reproduction: New UI (or Classic UI — behavior is identical), Data screen, click "Create Backup" under Backup & Restore.
started: Not a regression — this has been the behavior since the backup feature's original implementation (M3, pre-GSD phase, June/July 2026), unchanged through Phase 08's New UI port.

## Eliminated

- hypothesis: "New UI's Data/Backup screen was built without wiring the save-location picker, diverging from a working Classic UI flow that already has one."
  evidence: "`src/newui/screens/DataPane.tsx` is a literal one-to-one passthrough to the Classic `Data` component (`return <Data onBack={onBack} />;`, with a comment confirming '(D-11) - it already renders flex-1 p-4 internally, so it drops straight into NewShell's detail column with no added layout of its own'). `Data.tsx` in turn renders the same `Backup.tsx` component for both UIs. There is no New-UI-specific backup code path at all — New UI and Classic UI share the identical component and the identical `handleCreate` function. No divergence is possible because there is only one implementation."
  timestamp: 2026-09-16T15:43:18Z

- hypothesis: "The backend `create_backup` Tauri command or `do_create_backup` core function was changed/regressed during Phase 08 to drop dialog invocation or a path parameter it previously had."
  evidence: "The original M3 implementation plan (`docs/superpowers/plans/2026-07-12-m3-backup-csv.md`, lines 103-109) explicitly documents this as a deliberate day-one design decision, not something later removed: 'restoring a backup needs to pick an arbitrary [file]... The official tauri-plugin-dialog is the platform-blessed way to do this... create_backup does **not** use it: per the design, backups always write under the already-existing AppPaths::backups_dir(), so there is nothing to pick for that half.' The `create_backup` Tauri command signature (`src-tauri/src/commands.rs:361-364`) has never accepted a path argument; `do_create_backup` (`core/src/commands.rs:561-589`) has always computed the destination via `paths.backups_dir().join(...)`, a fixed non-configurable directory (`core/src/paths.rs:22-23`: `backups_dir() = root.join(\"backups\")`). Phase 08 touched none of these files for backup logic."
  timestamp: 2026-09-16T15:43:18Z

## Evidence

- timestamp: 2026-09-16T15:43:18Z
  checked: src/screens/Backup.tsx (the actual Backup screen rendered by both Classic and New UI)
  found: "`handleCreate()` (lines 12-24) calls `api.createBackup()` with zero arguments and never imports or calls `save()` from `@tauri-apps/plugin-dialog` — only `open` is imported (line 2), and it's used exclusively in `handleRestore()` (line 27) to pick an *existing* .bak file to restore FROM, not a save destination. The 'Create Backup' button's tooltip (line 63) reads: 'Write an encrypted backup of this vault to ~/.pwmgr/backups' — explicitly documenting the fixed-location behavior to the user in the UI itself."
  implication: "The frontend never had a save-location picker wired to backup creation; this is by design, not an oversight in the New UI port."

- timestamp: 2026-09-16T15:43:18Z
  checked: src/newui/screens/DataPane.tsx
  found: "Entire file is a 7-line passthrough: `export function DataPane({ onBack }) { return <Data onBack={onBack} />; }` with a comment explicitly noting it is a one-to-one passthrough to the existing Classic `Data` screen (D-11)."
  implication: "Confirms zero New-UI-specific implementation exists for Data/Backup — ruling out any New-UI-vs-Classic-UI divergence as the cause."

- timestamp: 2026-09-16T15:43:18Z
  checked: src-tauri/src/commands.rs:361-364 and core/src/commands.rs:561-589 (do_create_backup)
  found: "`create_backup` Tauri command takes only `state: State<AppState>, paths: State<AppPaths>` — no path/location parameter. `do_create_backup` builds the destination as `paths.backups_dir().join(format!(\"{}-{}.bak\", username, timestamp))` (core/src/commands.rs:588) and writes directly via `std::fs::write` (line 589). No dialog, no user-supplied path, anywhere in the Rust backend's backup-creation path. Contrast: `restore_backup` (src-tauri/src/commands.rs:366-374) DOES take a `path: String` argument — supplied by the frontend's native `open()` dialog — confirming the asymmetry is intentional (restore needs to locate an arbitrary existing file; create does not, by design)."
  implication: "Backend has no location-picker capability for create_backup to plumb through to the frontend even if the frontend were changed."

- timestamp: 2026-09-16T15:43:18Z
  checked: core/src/paths.rs:22-23
  found: "`backups_dir()` returns `self.root.join(\"backups\")` — a fixed subdirectory under the app's data root, not configurable per-call or via settings."
  implication: "There is no existing 'default location, user can override' mechanism — the destination is fully hardcoded."

- timestamp: 2026-09-16T15:43:18Z
  checked: docs/superpowers/plans/2026-07-12-m3-backup-csv.md (original M3 feature design/implementation plan, 2026-07-12, pre-dates Phase 08 by ~2 months)
  found: "Lines 103-109, under 'New dependency justification (ponytail ladder)': explicitly states create_backup intentionally does NOT use tauri-plugin-dialog, 'per the design, backups always write under the already-existing AppPaths::backups_dir(), so there is nothing to pick for that half.'"
  implication: "This is a deliberate, documented product/architecture decision made when the backup feature was first built (M3), not a bug introduced anywhere later, and certainly not by Phase 08."

- timestamp: 2026-09-16T15:43:18Z
  checked: src/screens/CsvExport.tsx (sibling feature under the same Data hub)
  found: "`handleExport()` (lines 22-25) DOES call `save({ defaultPath: defaultName })` from `@tauri-apps/plugin-dialog` before exporting — a native save-location picker IS used here, in the same Data screen, for CSV export."
  implication: "The native-save-dialog pattern is established and readily available in this codebase (proven working, low-risk to reuse) — it was simply never applied to backup creation, by the M3-era design choice above. This is useful context for a future fix but does not change the root-cause diagnosis: the omission predates Phase 08."

- timestamp: 2026-09-16T15:43:18Z
  checked: .planning/phases/08-new-vault-ui-figma-port/08-01-PLAN.md, lines 319-327 (Task's human-check verify step)
  found: "The Phase 08 Plan 01 human-check text, which was written to verify 'Classic UI unchanged' after adding the ui_variant switch, asserts: 'after logging in, revealing a secret and clicking copy still works ... and Data -> Backup still opens a native file dialog.' This claim is factually incorrect per all the evidence above — create_backup has never opened a native file dialog in this codebase."
  implication: "This is the likely origin of the false expectation. The incorrect 'native file dialog' claim in the Phase 08 plan's own verification checklist appears to have been carried forward, worded near-identically, into 08-UAT.md Test 1's 'expected' text ('Data->Backup native file dialog (matching Classic UI behavior)') and then into Gap G-08-1's `truth` field. The UAT is therefore flagging the absence of a feature that was mis-described as pre-existing Classic UI behavior in the phase's own planning documentation — most likely conflated with CsvExport's genuine native save dialog (same Data hub, visually adjacent) or with restore_backup's native open dialog (same screen, same 'native dialog' shape)."

## Resolution

root_cause: "The backup-creation flow (`src/screens/Backup.tsx`'s `handleCreate`, the `create_backup` Tauri command in `src-tauri/src/commands.rs`, and `do_create_backup` in `core/src/commands.rs`) has never invoked a native save-location dialog and has no code path capable of accepting a user-chosen destination — it always writes to the fixed, hardcoded `AppPaths::backups_dir()` directory (`core/src/paths.rs`). This was an explicit, documented design decision made in the original M3 feature implementation (docs/superpowers/plans/2026-07-12-m3-backup-csv.md), long before Phase 08 existed. Phase 08's New UI Data/Backup screen (`src/newui/screens/DataPane.tsx`) is a literal passthrough to this same, unchanged Classic-UI component, so there is no New-UI-specific regression or divergence — both UIs behave identically and always have. The UAT gap exists because Phase 08 Plan 01's own human-check verification text incorrectly asserted 'Data -> Backup still opens a native file dialog' as pre-existing Classic UI behavior to be confirmed unchanged; that false claim propagated into 08-UAT.md Test 1's expected description and into Gap G-08-1's truth statement, so the UAT surfaced a documentation error / mis-set expectation as if it were a regression, when it is actually a longstanding feature gap present equally in both UIs since before Phase 08."
fix: ""
verification: ""
files_changed: []
