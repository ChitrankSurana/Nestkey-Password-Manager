---
status: diagnosed
trigger: "UAT gap G-08-5 (phase 08 New Vault UI): Deleting a site with no credentials should show a confirm dialog reading 'Delete \"{name}\"? This cannot be undone.' before deleting. User reported: No popup shown asking to confirm deletion."
created: 2026-09-16T00:00:00Z
updated: 2026-09-16T00:00:00Z
---

## Current Focus

hypothesis: CONFIRMED — window.confirm() is not a reliably supported native dialog in Tauri v2's WRY-based webview runtime; the call in SiteEditor.tsx's handleDelete never surfaces a visible modal.
test: Read SiteEditor.tsx handleDelete implementation, traced its wiring through NewShell.tsx, confirmed it is the only delete path, ruled out New-UI-specific bypass, confirmed shared usage with Classic UI, and corroborated with documented Tauri v2 ecosystem behavior (GitHub issues/PRs) plus a second occurrence of the exact same anti-pattern already latent in this codebase (Backup.tsx:38).
expecting: N/A — root cause confirmed, diagnose-only mode (goal: find_root_cause_only). Not fixing.
next_action: none — return ROOT CAUSE FOUND to caller.

## Symptoms

expected: From UAT Test 5 — confirm the exact prompt copy 'Delete "{name}"? This cannot be undone.' fires before a site delete (truth per gap G-08-5: "Deleting a site (with no credentials) shows a confirm dialog reading 'Delete \"{name}\"? This cannot be undone.' before deleting")
actual: User reported: "No popup shown asking to confirm deletion."
errors: None reported
reproduction: Test 5 in UAT (08-UAT.md) — New UI, open a site with zero credentials, click "Edit site" (only reachable path per sibling gap G-08-4), click "Delete" in the edit-site modal
started: Discovered during UAT (phase 08, New Vault UI Figma port), 2026-09-15

## Eliminated

- hypothesis: An early-return branch in handleDelete skips window.confirm specifically when the site has zero credentials.
  evidence: Read src/screens/SiteEditor.tsx:41-54 in full. handleDelete has exactly one guard (`if (!existing) return;`) before the window.confirm call at line 43 — no credential-count check exists anywhere in this function or component. The zero-credential case takes the identical code path as any other site.
  timestamp: 2026-09-16T00:00:00Z

- hypothesis: New UI has its own delete-site path that bypasses SiteEditor.tsx's handleDelete entirely (e.g. a direct delete affordance in DetailPanel's empty-credentials state).
  evidence: Grepped entire src/ tree for softDeleteSite/handleDelete/window.confirm — only 6 files match, and site-deletion (softDeleteSite) appears solely in src/screens/SiteEditor.tsx. Read src/newui/components/DetailPanel.tsx in full: the zero-credential empty state (lines 60-77) renders only an "Add credential" CTA, no delete affordance. The only site-delete entry point in the New UI is NewShell.tsx's `editingSite && vault.selectedSite` block (NewShell.tsx:230-244), which renders the identical shared `<SiteEditor existing=... onDeleted=.../>` component also used by Classic UI's Vault.tsx:113. Confirms sibling gap G-08-4's finding that delete is edit-site-only, and rules out a New-UI-specific bypass.
  timestamp: 2026-09-16T00:00:00Z

- hypothesis: NewShell.tsx wires SiteEditor's onDeleted/onClose callbacks incorrectly, causing the modal to close/unmount before window.confirm can render.
  evidence: Read NewShell.tsx:230-244 in full. onDeleted only fires *after* handleDelete's `await api.softDeleteSite(...)` succeeds (SiteEditor.tsx:47-48), which itself only runs after window.confirm returns truthy. No early unmount, no conditional short-circuit before the confirm call. Wiring is correct and inert with respect to this bug.
  timestamp: 2026-09-16T00:00:00Z

- hypothesis: A CSP (Content-Security-Policy) setting in tauri.conf.json blocks the confirm() call or its rendering.
  evidence: src-tauri/tauri.conf.json:15 — `"csp": null`. No CSP is configured at all, so it cannot be blocking anything.
  timestamp: 2026-09-16T00:00:00Z

- hypothesis: Rust-side window creation (WebviewWindowBuilder in src-tauri/src/lib.rs) uses settings (invisible, unfocused, decorations off, etc.) that would prevent a native dialog from being visible/focused.
  evidence: Read src-tauri/src/lib.rs in full. The window builder (lines 52-55) only sets `.title("tauri-app")` and `.inner_size(800.0, 600.0)` — no visibility/focus/decoration overrides for either "new" or "classic" entry points. Both UI variants share the exact same window builder call; nothing UI-variant-specific here.
  timestamp: 2026-09-16T00:00:00Z

- hypothesis: Tooltip.tsx (wrapping the Delete button) intercepts or swallows the click event before handleDelete's onClick fires.
  evidence: Read src/components/Tooltip.tsx in full. It only clones the child with an added `aria-describedby` prop and attaches hover/focus listeners to an outer wrapper `<span>` for its own tooltip-visibility state — it does not touch, wrap, or stop-propagate the child's onClick handler. Click reaches handleDelete unmodified.
  timestamp: 2026-09-16T00:00:00Z

- hypothesis: The G-08-1 "no backup location picker" symptom and this gap share a root cause (both are "no dialog appeared" reports, suggesting a broader Tauri dialog-rendering failure in this window rather than a window.confirm-specific issue).
  evidence: Read src/screens/Backup.tsx in full. `handleCreate` (the "create backup" button, the one G-08-1 is about) never calls any dialog API at all — `api.createBackup()` is invoked directly with a fixed backend-side save location; a save-location picker was simply never implemented for that flow. This is an unrelated missing-feature gap (G-08-1), not a dialog-rendering failure, and does not corroborate or contradict the window.confirm hypothesis for G-08-5.
  timestamp: 2026-09-16T00:00:00Z

## Evidence

- timestamp: 2026-09-16T00:00:00Z
  checked: src/screens/SiteEditor.tsx (full file, 124 lines)
  found: handleDelete (lines 41-54) is exactly the flow the sibling gap G-08-4 lead described — `if (!window.confirm(\`Delete "${existing.name}"? This cannot be undone.\`)) return;` fires as literally the first statement, synchronously, directly inside the onClick-triggered async function (SiteEditor.tsx:43), with no await before it (so it stays inside the same call stack/user-gesture context as the click). Copy text matches the expected truth exactly. This same handler and JSX is used regardless of the site's credential count.
  implication: The confirm-gate code is present, correctly placed, and correctly worded. If window.confirm() itself does not render/behave as a blocking native dialog in this runtime, the user would see exactly "no popup" while the code otherwise "looks right" — matching the reported symptom precisely.

- timestamp: 2026-09-16T00:00:00Z
  checked: src/newui/NewShell.tsx (delete-site wiring, lines 220-244) and src/screens/Vault.tsx (SiteEditor import/usage, lines 3, 113)
  found: SiteEditor.tsx is imported and rendered identically by both the New UI (NewShell.tsx) and the Classic UI (Vault.tsx) — it is one shared component, not duplicated or New-UI-specific.
  implication: Any defect in SiteEditor.tsx's window.confirm usage is NOT a New-UI regression — it is a pre-existing, UI-variant-agnostic bug that this UAT round happened to be the first to exercise and report. (Classic UI's own regression walk, Test 6, was skipped in this UAT round, so Classic UI's delete-site confirm has never actually been independently verified working either — its "unchanged" status in Test 1 covered login/clipboard/backup-dialog only, not site deletion.)

- timestamp: 2026-09-16T00:00:00Z
  checked: src/screens/Backup.tsx (full file, 77 lines), specifically handleRestore's cascade-confirm at line 38
  found: A second, independent occurrence of the exact same pattern exists in this codebase: `if (window.confirm(err.message)) { ... }` guards the "confirm_required" cascading backup-restore action. This flow was not exercised by this UAT round (Test 1's backup check only covered create-backup's missing location picker, G-08-1), so this second latent instance has not yet been separately reported as broken — but it is architecturally identical to the SiteEditor.tsx bug and would be expected to fail the same way if triggered.
  implication: This is a systemic pattern (raw window.confirm() used for destructive-action confirmation), not an isolated one-off mistake in SiteEditor.tsx. Any fix decision should account for both occurrences.

- timestamp: 2026-09-16T00:00:00Z
  checked: package.json (line 15) and src/screens/Backup.tsx / CsvExport.tsx / CsvImport.tsx (import lines)
  found: `@tauri-apps/plugin-dialog` (^2.7.1) is already an installed dependency, and is already correctly used elsewhere in the frontend for `open()`/`save()` native file-picker dialogs (Backup.tsx:2, CsvExport.tsx:2, CsvImport.tsx:2). The same package also exports `confirm()`/`ask()` functions for exactly this kind of native yes/no dialog, but neither SiteEditor.tsx's delete-site flow nor Backup.tsx's restore-cascade flow uses them — both use the raw Web API `window.confirm()` instead.
  implication: The project already has the correct, working replacement mechanism installed and proven functional (via the file-picker usages) — it is simply not being used for the two destructive-confirmation flows. This is strong supporting evidence for what the actual fix direction should be, without prescribing it (diagnose-only mode).

- timestamp: 2026-09-16T00:00:00Z
  checked: src-tauri/Cargo.toml (tauri = "2") and package.json (@tauri-apps/api ^2) — confirms Tauri v2 throughout, both Rust and JS sides
  found: Project is on Tauri v2 (WRY-based webview runtime: WebView2 on Windows, the platform this UAT was run on per environment info).
  implication: Version context matches the documented Tauri ecosystem behavior below.

- timestamp: 2026-09-16T00:00:00Z
  checked: Web search for Tauri v2 window.confirm() behavior (external research, since no other code-level cause was found after ruling out every code-path hypothesis above)
  found: This is a well-documented, known Tauri limitation/anti-pattern, not specific to this codebase. Found: (1) a GitHub PR literally titled "fix(desktop): window cannot be closed, and confirm() guards never fire" describing the same class of bug in another Tauri app; (2) a GitHub issue titled (translated from Chinese) "window.confirm silently fails under Tauri: replace with a native ask() dialog for batch-translation confirmation"; (3) general community guidance (e.g. "Tauri dialog instead of window.confirm" on DEV Community) recommending `@tauri-apps/plugin-dialog`'s `ask()`/`confirm()` as the correct replacement specifically because Tauri's WRY-based webviews (WKWebView on macOS explicitly, and WebView2/WebKitGTK inconsistently depending on version/platform) do not reliably implement the browser-standard blocking `window.confirm()`/`alert()`/`prompt()` dialogs the way a normal browser tab does.
  implication: Combined with every code-path hypothesis above being eliminated (the confirm call is correctly placed, correctly worded, correctly gated, not bypassed, not intercepted, not blocked by CSP or window settings, and identical between UI variants), the remaining and best-supported explanation is that `window.confirm()` itself is the point of failure at the platform/runtime layer, not any application logic bug.

## Resolution

root_cause: "window.confirm() — used at src/screens/SiteEditor.tsx:43 inside handleDelete, the sole site-delete entry point shared by both Classic and New UI — is not a reliably supported native dialog primitive inside Tauri v2's WRY-based webview runtime. The confirm-gate code itself is present, correctly worded ('Delete \"{name}\"? This cannot be undone.'), and correctly ordered (fires synchronously before the softDeleteSite call, for every site regardless of credential count) — every application-level hypothesis (credential-count-specific early return, New-UI-specific bypass path, broken NewShell wiring, CSP block, window-visibility/focus settings, Tooltip event interception) was checked directly against the code and eliminated. What remains, and is corroborated by documented Tauri-ecosystem behavior plus a second identical latent occurrence already in this codebase (Backup.tsx:38's restore-cascade confirm), is that the raw browser Web API window.confirm() does not reliably render as a blocking OS dialog inside Tauri's webview (WebView2 on Windows / WKWebView on macOS / WebKitGTK on Linux) — so the call executes but produces no visible prompt, matching the user's exact report of 'No popup shown asking to confirm deletion.'"
fix: ""
verification: ""
files_changed: []
