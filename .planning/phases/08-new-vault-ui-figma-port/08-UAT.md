---
status: diagnosed
phase: 08-new-vault-ui-figma-port
source: [08-VERIFICATION.md]
started: 2026-09-14T17:17:38Z
updated: 2026-09-16T00:00:00Z
---

## Current Test

[testing complete]

## Tests

### 1. Full desktop launch and Classic/New UI switch parity
expected: With ~/.pwmgr/config.json absent or ui_variant=classic, the Classic UI opens in an 800x600 window titled tauri-app unchanged (login screen, clipboard copy with 30s-clear toast, Data->Backup native file dialog). Set ui_variant=new, relaunch, confirm the new entry opens and shows the real site list. Set back to classic, relaunch, confirm Classic returns exactly as before.
result: issue
reported: "Config had no ui_variant before opening — opened old UI (expected default, not a bug). No master password prompt when viewing a credential's password. Need option to choose backup location when clicking 'create backup'. UI variant toggle itself works."
severity: major

### 2. Settings "Interface" section toggle and persistence
expected: Switch to New, confirm a persistent (non-fading) banner appears and survives 10+ seconds and a collapse/reopen of the section; restart and confirm New UI opens and banner is gone. In web mode, change the select and confirm the banner instead names /newui.html and the current tab does not switch UIs.
result: pass
note: "Desktop steps (1-4) confirmed. Web-mode banner check postponed by user until desktop is considered complete — see Deferred Follow-Ups."

### 3. Three-pane desktop view rendering (>=1024px)
expected: Sidebar (shield chip, "Vault", username, six destinations), 280px live-search list (no debounce, no duplicate rows for a two-credential site), selected-row accent-subtle tint, long name/url truncation with ellipsis, light-mode background correctness after a Classic theme change.
result: pass

### 4. Detail column walkthrough — reprompt gate, repeater, auto-lock
expected: Select a site with one credential (masked secret, live strength meter); reveal/copy a reprompt-flagged credential and confirm the master-password prompt fires on both reveal AND copy, for the main secret and for a hidden custom field; select a 3-credential site and confirm one expanded + "Other logins (2)" toggle; select a 0-credential site and confirm the empty state and its CTA; confirm notes/remarks render with line breaks; leave the app idle until auto-lock and confirm the panel empties and returns to login.
result: issue
reported: "Adding a custom field shows a 'Something went wrong. Please Try Again' error. All else (main-secret reprompt on reveal+copy, 3-credential repeater/toggle, 0-credential empty state, notes line breaks, idle auto-lock) works correctly."
severity: blocker

### 5. CRUD round-trip and navigation
expected: Add/edit/delete a site (confirm exact prompt copy 'Delete "{name}"? This cannot be undone.'), add/edit/delete a credential (no confirm dialog, appears in Trash), confirm list/detail refresh without reselecting. Navigate all six sidebar destinations (Health, Generator, Trash, Data, Settings, Vault) and confirm each renders its real screen with a working Back control. Click a weak entry in Health and confirm it lands on that site's detail view; click "View full health" on a credential and confirm it opens the Health pane. Collapse the sidebar, restart, confirm it stays collapsed. Click Lock and confirm return to login.
result: issue
reported: "Delete control only reachable via edit-site, should be exposed directly. No confirm dialog shown when deleting a site with no credentials. Deleting a site that already has credentials is blocked entirely (message says credentials exist, must delete them one by one first) instead of offering a confirmed cascade delete. Login and reprompt password-check both feel slow; audited (see Audit Notes) — decision: add loading animations now, defer any KDF-cost change to a later decision. All other points (edit, credential CRUD to Trash, list/detail auto-refresh, all six nav destinations + Back, Health->detail linking, View-full-health, sidebar-collapse persistence across restart, Lock) work correctly."
severity: major

### 6. Responsive tiers, full capability checklist, and Classic regression check
expected: Resize the window through all three D-18 tiers (>=1024px full sidebar honoring navCollapsed; 640-1023px icon rail with navCollapsed untouched; <640px mobile nav swap with single visible pane and a working Back). Walk all 18 flows in docs/UI_REDESIGN_CAPABILITY_CHECKLIST.md §1 in the New UI, and separately confirm all six §4 cross-cutting behaviors (30s clipboard auto-clear, reprompt on reveal+copy for both secret types, idle/sleep auto-lock returns to login, theme+all 5 accents load flash-free, all three mobileNavStyle values render correctly below 640px, no credential fetched before a site is opened). Finally set ui_variant back to classic and walk the same 18 flows in Classic to confirm nothing changed.
result: skipped
reason: "User will complete this test later"

## Summary

total: 6
passed: 2
issues: 3
pending: 0
skipped: 1
blocked: 0

## Deferred Follow-Ups

- test: 2
  idea: "Web-mode 'Interface' select banner check (naming /newui.html, no in-tab switch) — postponed until the desktop New UI is considered complete."
  deferred_at: 2026-09-15

## Gaps

- gap_id: G-08-1
  truth: "Clicking Data -> Backup opens a native OS dialog letting the user pick the backup file's save location"
  status: failed
  reason: "User reported: Need option to choose backup location when clicking \"create backup\" — no location picker appeared."
  severity: major
  test: 1
  root_cause: "Not a regression — create_backup has never had a save-location picker, by explicit M3 design (writes always to hardcoded AppPaths::backups_dir()). UAT's expectation was seeded by a false claim in 08-01-PLAN.md's own verification text asserting a native dialog was pre-existing Classic behavior; it isn't. Restore and CSV export DO use native dialogs (open()/save() via tauri-plugin-dialog) — backup-create never did."
  artifacts:
    - path: "src/screens/Backup.tsx"
      issue: "handleCreate never invokes a save() dialog; imports open() for restore only"
    - path: "src-tauri/src/commands.rs"
      issue: "create_backup command (lines 361-364) takes no destination-path argument"
    - path: "core/src/commands.rs"
      issue: "do_create_backup (lines 561-589) always writes to fixed backups_dir()"
    - path: "core/src/paths.rs"
      issue: "backups_dir() (lines 22-23) is hardcoded, non-configurable"
  missing:
    - "save() dialog call in Backup.tsx's handleCreate, mirroring CsvExport.tsx's existing pattern"
    - "optional destination-path parameter threaded through create_backup / do_create_backup (mirroring restore_backup's existing path param)"
    - "correction of the stale 'native file dialog' claim in 08-01-PLAN.md so it doesn't reseed this expectation"
  debug_session: ".planning/debug/backup-location-picker-missing.md"
- gap_id: G-08-2
  truth: "A new Settings field 'Reprompt mode' (Global | Per-credential) controls the reprompt gate. Global: every credential's password reveal/copy requires the master password; a correct entry opens a grace window (default 30s, configurable in Settings) during which further reveals/copies for ANY credential do not re-prompt; after the window expires, the next reveal/copy re-prompts. Per-credential: today's existing behavior — only credentials with 'Require reprompt' ON ask for the master password, and they ask every single time (no grace window)."
  status: failed
  reason: "User reported: no master password was asked to view a credential's password. Locked decision (this session): add the Global/Per-credential Settings toggle described above rather than changing per-credential default behavior."
  severity: major
  test: 1
  root_cause: "Not a bug — the tested credential had its per-credential reprompt DB flag set to false (schema default), so SecretField.requestAction()'s existing else-branch revealed immediately by design; Test 4's credential had the flag true and correctly prompted. Both results are the same, unmodified, correct gate. The Global/grace-window mode itself is genuinely unimplemented: no repo-wide scaffolding exists (no ConfigDto field, no 'grace' identifier anywhere)."
  artifacts:
    - path: "src/components/SecretField.tsx"
      issue: "requestAction (lines 35-41) only implements per-credential gating — needs to branch on a Global mode + grace-window check"
    - path: "core/src/dto.rs"
      issue: "ConfigDto (lines 122-134) has no repromptMode or repromptGraceSeconds field"
  missing:
    - "repromptMode: 'global' | 'perCredential' field on ConfigDto/Settings (Rust + frontend getConfig/updateConfig, mirroring ui_variant/auto_lock_minutes wiring)"
    - "repromptGraceSeconds: u32 (default 30) config field"
    - "session-scoped 'last successful reprompt' timestamp, checked first in SecretField.requestAction() when mode is global"
  debug_session: ".planning/debug/reprompt-not-firing-on-reveal.md"
- gap_id: G-08-3
  truth: "Adding a custom field to a credential in the New UI succeeds without error"
  status: failed
  reason: "User reported: Adding a custom field shows a 'Something went wrong. Please Try Again' error. Blocks verifying the hidden-custom-field reprompt path (Test 4 sub-check) since a hidden field can't be added to test against."
  severity: blocker
  test: 4
  root_cause: "pwmgr_core::vault::CustomField (core/src/vault.rs:121-126) is missing #[serde(rename_all = \"camelCase\")]. Frontend sends fieldType (camelCase); backend deserializes field_type — every add_credential/update_credential call with a non-empty custom field fails IPC/HTTP deserialization before business logic runs, on both desktop and web paths. Pre-existing latent bug (introduced pre-Phase-08, commit aa0dcad), first surfaced by this UAT. The generic error message is because the transport-level failure doesn't match the app's {code,message} error shape."
  artifacts:
    - path: "core/src/vault.rs"
      issue: "CustomField struct (lines 121-126) missing #[serde(rename_all = \"camelCase\")] — CustomFieldType right above it has the attribute, CustomField doesn't"
  missing:
    - "#[serde(rename_all = \"camelCase\")] on CustomField struct"
    - "regression test that round-trips CustomField through actual JSON, not just Rust-native construction"
  debug_session: ".planning/debug/custom-field-add-error.md"
- gap_id: G-08-4
  truth: "A delete control for a site is reachable directly, without first opening edit mode"
  status: failed
  reason: "User reported: I shouldn't need to click on edit site to see the delete button, it should be available outside somewhere."
  severity: minor
  test: 5
  root_cause: "Pre-existing Classic UI limitation (SiteDetail.tsx has the same edit-gated delete-site pattern) carried forward unexamined into the Figma port. docs/FIGMA_UI_VS_CURRENT_FEATURES.md mischaracterized the Figma design's direct header delete as already 'equivalent' to the app's edit-modal-gated behavior, so 8-UI-SPEC.md (D-11/D-12) locked in unchanged reuse of SiteEditor.tsx as the sole delete trigger. DetailPanel.tsx was built faithfully to that spec — not an implementation defect against Phase 8's own spec, but a spec-review miss."
  artifacts:
    - path: "src/newui/components/DetailPanel.tsx"
      issue: "header (lines 136-157) has Edit/Add-credential buttons but no delete affordance"
    - path: "src/screens/SiteEditor.tsx"
      issue: "sole owner of softDeleteSite, only reachable once already in edit mode"
  missing:
    - "direct 'Delete site' icon button in DetailPanel.tsx header, calling api.softDeleteSite with the same confirm copy SiteEditor.tsx uses"
  debug_session: ".planning/debug/delete-control-not-direct.md"
- gap_id: G-08-5
  truth: "Deleting a site (with no credentials) shows a confirm dialog reading 'Delete \"{name}\"? This cannot be undone.' before deleting"
  status: failed
  reason: "User reported: No popup shown asking to confirm deletion."
  severity: major
  test: 5
  root_cause: "window.confirm() (SiteEditor.tsx:43) is not reliably supported inside Tauri v2's WRY-based webview (WebView2/WKWebView/WebKitGTK) — it silently produces no visible dialog. The confirm-gate code itself is present, correctly worded, and correctly ordered (fires first in handleDelete for every site regardless of credential count) — this is a runtime/API-support bug, not an application logic bug. A second identical latent occurrence exists at Backup.tsx:38 (restore-cascade confirm), untested this round. @tauri-apps/plugin-dialog is already installed and already proven working for native file pickers (open()/save() in Backup.tsx, CsvExport.tsx, CsvImport.tsx) — just not used for these two confirms."
  artifacts:
    - path: "src/screens/SiteEditor.tsx"
      issue: "line 43: window.confirm() produces no visible dialog in Tauri's webview"
    - path: "src/screens/Backup.tsx"
      issue: "line 38: same anti-pattern, restore-cascade confirm — separate latent occurrence"
  missing:
    - "replace window.confirm() with @tauri-apps/plugin-dialog's confirm()/ask() in SiteEditor.tsx"
    - "same replacement in Backup.tsx's restore-cascade confirm"
  debug_session: ".planning/debug/no-confirm-dialog-delete-site-no-creds.md"
- gap_id: G-08-6
  truth: "A site that already has saved credentials can still be deleted (cascading the delete to its credentials), gated behind a confirmation popup — not blocked outright"
  status: failed
  reason: "User reported: it says I have a credential saved for that site and just stops deletion altogether until all credentials for that site are deleted one by one. User wants: I should be able to delete site (with a confirmation popup) even if it has credentials already saved for it."
  severity: major
  test: 5
  root_cause: "Deliberate backend guard, not a bug: do_soft_delete_site (core/src/commands.rs:260-275) refuses outright when count_live_credentials_for_site > 0, because the lower-level soft_delete_site (core/src/vault.rs:218-227) has zero cascade logic — it's a bare single-table UPDATE that never touches credentials, and list_credentials_for_site filters out any credential whose site is soft-deleted. Removing the guard without adding cascade logic would permanently orphan credentials (live but unreachable, no restore-site UI exists). Pre-Phase-8 behavior, explicitly preserved (not introduced) by 8-UI-SPEC.md. Requires a genuine pwmgr-core behavior change — product decision on cascade semantics needed before implementation."
  artifacts:
    - path: "core/src/commands.rs"
      issue: "do_soft_delete_site (lines 260-275): refuse-outright gate, no cascade path"
    - path: "core/src/vault.rs"
      issue: "soft_delete_site (lines 218-227): single-table UPDATE, no cascade to credentials"
  missing:
    - "cascade logic in soft_delete_site (or do_soft_delete_site) to soft-delete the site's live credentials in the same operation"
    - "frontend confirm-gated cascade UX in SiteEditor.tsx (entangled with G-08-5's window.confirm fix)"
    - "product decision: cascade semantics, whether restoring a site later also restores its credentials"
  debug_session: ".planning/debug/delete-blocked-when-credentials-exist.md"
- gap_id: G-08-7
  truth: "A loading animation shows during any operation with a perceptible wait (login, master-password reprompt check, and similar)"
  status: failed
  reason: "User reported: login and password check (reprompt popup) both take too much time with no feedback; add a loading animation wherever loading the next thing takes time. Decision (this session): add the loading animation now; the underlying KDF cost itself is NOT being changed as part of this — see Audit Notes."
  severity: minor
  test: 5
  root_cause: "Pure rendering/UX omission, not missing state plumbing. Both Login.tsx (submitting) and SecretField.tsx (verifying) already track in-flight state correctly (set before the single await, cleared in finally) and already gate disabled — but neither renders an actual animation: Login.tsx only dims buttons to 50% opacity, SecretField.tsx only swaps text to 'Checking…'. No spinner/loading-animation component exists anywhere in the codebase to wire up."
  artifacts:
    - path: "src/screens/Login.tsx"
      issue: "submitting state wired to disabled but only dims opacity — no animation"
    - path: "src/components/SecretField.tsx"
      issue: "verifying state wired to disabled, text swaps to 'Checking…' — no animation"
  missing:
    - "reusable spinner component composed from existing Icon.tsx (bootstrap-icons arrow-repeat/arrow-clockwise glyph) + Tailwind's animate-spin utility (both already in use elsewhere — no new dependency needed)"
    - "spinner wired into Login.tsx submit/biometric buttons and SecretField.tsx Confirm button when submitting/verifying is true"
  debug_session: ".planning/debug/no-loading-animation-login-reprompt.md"

## Audit Notes

- **Login / reprompt delay investigation (2026-09-15):** Measured `core::crypto::derive_key` (Argon2id, m=64MiB t=3 p=4, `core/src/crypto.rs:48`) directly in a debug build (the profile `npm run tauri dev` always uses): **1961ms per call**. Login calls it twice (`core/src/auth.rs:40-41`, vault key + verifier key) → ~4s wall time; reprompt calls it once → ~2s. This matches the user's "takes too much time" report.
  - The KDF parameters are intentional and already reviewed as correct in `SECURITY_CONCERNS.md` (flagged there only for missing rate-limiting in web mode, M2 — not for being mistuned). Reducing them would weaken brute-force resistance.
  - Attempted to measure release-build timing to confirm the debug-only-artifact theory empirically; release build fails in this environment for an unrelated pre-existing reason — `rusqlite`'s `bundled-sqlcipher-vendored-openssl` feature needs to vendor-build OpenSSL, which needs a Perl module (`Locale::Maketext::Simple`) not installed here. Not fixed as part of this audit (out of scope, pre-existing environment gap, unrelated to phase 8 or the KDF).
  - Unoptimized-vs-optimized Rust for tight crypto loops (Argon2, AES) is a well-documented 5-20x gap, so the real (release) delay is expected to be a few hundred ms, not multiple seconds — i.e. likely a debug-build artifact, not a real production bug.
  - **Decision:** add a loading animation now (G-08-7). Do NOT change the KDF cost as part of this phase's gap closure. Re-measure on an actual release build later, then decide if the KDF cost needs reducing.
