---
status: testing
phase: 08-new-vault-ui-figma-port
source: [08-VERIFICATION.md]
started: 2026-09-14T17:17:38Z
updated: 2026-09-14T17:17:38Z
---

## Current Test

number: 1
name: Full desktop launch and Classic/New UI switch parity
expected: |
  Classic is byte-identical in behavior pre/post phase; New UI opens and reaches the real backend when selected.
awaiting: user response

## Tests

### 1. Full desktop launch and Classic/New UI switch parity
expected: With ~/.pwmgr/config.json absent or ui_variant=classic, the Classic UI opens in an 800x600 window titled tauri-app unchanged (login screen, clipboard copy with 30s-clear toast, Data->Backup native file dialog). Set ui_variant=new, relaunch, confirm the new entry opens and shows the real site list. Set back to classic, relaunch, confirm Classic returns exactly as before.
result: [pending]

### 2. Settings "Interface" section toggle and persistence
expected: Switch to New, confirm a persistent (non-fading) banner appears and survives 10+ seconds and a collapse/reopen of the section; restart and confirm New UI opens and banner is gone. In web mode, change the select and confirm the banner instead names /newui.html and the current tab does not switch UIs.
result: [pending]

### 3. Three-pane desktop view rendering (>=1024px)
expected: Sidebar (shield chip, "Vault", username, six destinations), 280px live-search list (no debounce, no duplicate rows for a two-credential site), selected-row accent-subtle tint, long name/url truncation with ellipsis, light-mode background correctness after a Classic theme change.
result: [pending]

### 4. Detail column walkthrough — reprompt gate, repeater, auto-lock
expected: Select a site with one credential (masked secret, live strength meter); reveal/copy a reprompt-flagged credential and confirm the master-password prompt fires on both reveal AND copy, for the main secret and for a hidden custom field; select a 3-credential site and confirm one expanded + "Other logins (2)" toggle; select a 0-credential site and confirm the empty state and its CTA; confirm notes/remarks render with line breaks; leave the app idle until auto-lock and confirm the panel empties and returns to login.
result: [pending]

### 5. CRUD round-trip and navigation
expected: Add/edit/delete a site (confirm exact prompt copy 'Delete "{name}"? This cannot be undone.'), add/edit/delete a credential (no confirm dialog, appears in Trash), confirm list/detail refresh without reselecting. Navigate all six sidebar destinations (Health, Generator, Trash, Data, Settings, Vault) and confirm each renders its real screen with a working Back control. Click a weak entry in Health and confirm it lands on that site's detail view; click "View full health" on a credential and confirm it opens the Health pane. Collapse the sidebar, restart, confirm it stays collapsed. Click Lock and confirm return to login.
result: [pending]

### 6. Responsive tiers, full capability checklist, and Classic regression check
expected: Resize the window through all three D-18 tiers (>=1024px full sidebar honoring navCollapsed; 640-1023px icon rail with navCollapsed untouched; <640px mobile nav swap with single visible pane and a working Back). Walk all 18 flows in docs/UI_REDESIGN_CAPABILITY_CHECKLIST.md §1 in the New UI, and separately confirm all six §4 cross-cutting behaviors (30s clipboard auto-clear, reprompt on reveal+copy for both secret types, idle/sleep auto-lock returns to login, theme+all 5 accents load flash-free, all three mobileNavStyle values render correctly below 640px, no credential fetched before a site is opened). Finally set ui_variant back to classic and walk the same 18 flows in Classic to confirm nothing changed.
result: [pending]

## Summary

total: 6
passed: 0
issues: 0
pending: 6
skipped: 0
blocked: 0

## Gaps
