---
phase: 08-new-vault-ui-figma-port
plan: 05
subsystem: ui
tags: [react, tailwind, tauri]

# Dependency graph
requires:
  - phase: 08-04
    provides: "src/newui/components/DetailPanel.tsx, CredentialBlock.tsx, CopyButton.tsx, DetailField.tsx (the detail column), and src/newui/NewShell.tsx's screen state slice (initialised to \"vault\", wired to setScreen on all six destinations) this plan dispatches into"
provides:
  - "src/newui/NewShell.tsx: editor-orchestration state (addingSite, editingSite, editingCredential), refreshAfterSave(), goVault(), and the full six-way screen dispatch (DetailPanel + five secondary panes)"
  - "src/newui/components/DetailPanel.tsx: Edit site and Add credential header controls, onEditCredential/onCredentialDeleted/onOpenHealth threaded to CredentialBlock"
  - "src/newui/components/CredentialBlock.tsx: per-credential Edit and Delete controls (api.softDeleteCredential, no confirm dialog) and a \"View full health\" link-out beside the strength meter"
  - "src/newui/screens/GeneratorPane.tsx, HealthPane.tsx, DataPane.tsx, TrashPane.tsx, SettingsPane.tsx: one-line passthrough wrappers over the existing Generator/HealthDashboard/Data/Trash/Settings screens"
affects: [08-06]

# Actuals (#2632)
actuals:
  tokens: 3787
  tasks: 3
  commits: 3
plan_head_before: 50776fffeadb1405f9f683df4788fdae47e8592b

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Editor-overlay orchestration state (addingSite/editingSite/editingCredential) lives at NewShell (shell) level, not inside the leaf components that trigger it - ItemList/DetailPanel/CredentialBlock raise flags via threaded callback props, mirroring the plan's own \"thread callbacks down as props rather than letting leaf components reach into shell state directly\" instruction"
    - "One-line passthrough wrapper per secondary screen (GeneratorPane etc.) - each imports the corresponding src/screens/*.tsx file unchanged and forwards props 1:1, adding no layout of its own, matching the {onBack}-taking wrapper pattern already established by AppShell.tsx's render dispatch"
    - "refreshAfterSave() as the single post-save refresh path: calls vault.refreshSites() and, when a site is selected, vault.refreshCredentials(selectedSiteId) - both SiteEditor's and CredentialEditor's onSaved route through it, and CredentialBlock's onDeleted routes through the same path via onCredentialDeleted"

key-files:
  created:
    - src/newui/screens/GeneratorPane.tsx
    - src/newui/screens/HealthPane.tsx
    - src/newui/screens/DataPane.tsx
    - src/newui/screens/TrashPane.tsx
    - src/newui/screens/SettingsPane.tsx
  modified:
    - src/newui/NewShell.tsx
    - src/newui/components/DetailPanel.tsx
    - src/newui/components/CredentialBlock.tsx

key-decisions:
  - "ItemList.tsx was NOT modified despite being listed in the plan's files_modified frontmatter - its onAddSite prop already raised the shell's addingSite flag as of plan 08-03 (NewShell already passed onAddSite={() => setAddingSite(true)}), so the plan's Task 1 instruction to 'change the existing onAddSite callback to raise the shell's addingSite flag' was already satisfied and required no code change. Confirmed by reading the file before editing (read_first gate) rather than assuming."
  - "CredentialBlock gained a local error state (rendered as the same text-sm text-red-400 convention used elsewhere) to surface a failed softDeleteCredential call, since the component previously had no error-display path of its own - the plan's 'shared catch fallback' instruction meant the same isCommandError/generic-message pattern, not a specific existing state slice to reuse."
  - "CredentialBlock's Edit/Delete/onOpenHealth callbacks are threaded through DetailPanel as onEditCredential(credential)/onCredentialDeleted/onOpenHealth props rather than letting CredentialBlock import CredentialEditor or reach into shell state directly - keeps the same threaded-props boundary DetailPanel already established in plan 08-04 for onToggle/expanded."

requirements-completed: [VAULT-03]

coverage:
  - id: D1
    description: "Every site create/edit/delete path and every credential create/edit/delete path works from the new UI through the existing unmodified SiteEditor/CredentialEditor overlays; site deletion keeps SiteEditor's own confirm copy, credential deletion has no confirm dialog (soft delete, restorable from Trash); saves refresh the list and, when a site is selected, its credentials, without reselecting"
    requirement: VAULT-03
    verification:
      - kind: other
        ref: "grep -c 'SiteEditor' src/newui/NewShell.tsx (3, expects >0)"
        status: pass
      - kind: other
        ref: "grep -c 'CredentialEditor' src/newui/NewShell.tsx (2, expects >0)"
        status: pass
      - kind: other
        ref: "grep -c 'softDeleteCredential' src/newui/components/CredentialBlock.tsx (1, expects >0)"
        status: pass
      - kind: other
        ref: "grep -rn 'window.confirm' src/newui/ (0 matches, expects 0)"
        status: pass
      - kind: other
        ref: "git diff --quiet HEAD -- src/screens (exit 0 - no reused screen edited)"
        status: pass
      - kind: unit
        ref: "npm test (18/18 pass)"
        status: pass
      - kind: other
        ref: "npm run build"
        status: pass
    human_judgment: false
  - id: D2
    description: "All five secondary screens (Generator/Health/Data/Trash/Settings) exist as one-to-one passthrough wrappers under src/newui/screens/ with no added layout, importing the original screens from src/screens/ unchanged; HealthPane threads onNavigateToSite, SettingsPane threads username"
    requirement: VAULT-03
    verification:
      - kind: other
        ref: "ls src/newui/screens/GeneratorPane.tsx src/newui/screens/HealthPane.tsx src/newui/screens/DataPane.tsx src/newui/screens/TrashPane.tsx src/newui/screens/SettingsPane.tsx (exit 0, all 5 present)"
        status: pass
      - kind: other
        ref: "grep -c 'onNavigateToSite' src/newui/screens/HealthPane.tsx (4, expects >0)"
        status: pass
      - kind: other
        ref: "git diff --quiet HEAD -- src/screens (exit 0)"
        status: pass
      - kind: other
        ref: "npm run build"
        status: pass
    human_judgment: false
  - id: D3
    description: "All six sidebar destinations dispatch into NewShell's detail column (vault -> DetailPanel/empty-state, the other five -> their *Pane wrapper), each pane receives goVault as onBack, HealthPane's onNavigateToSite sets the screen back to vault and selects the given site, Sidebar receives activeKey={screen}, collapse persistence (api.updateNavCollapsed) and lock (api.lock) remain wired, and CredentialBlock's strength block links out to Health via a new onOpenHealth prop instead of fabricating a per-item breach/reuse status"
    requirement: VAULT-03
    verification:
      - kind: other
        ref: "grep -c 'HealthPane\\|GeneratorPane\\|TrashPane\\|DataPane\\|SettingsPane' src/newui/NewShell.tsx (11, expects >=5)"
        status: pass
      - kind: other
        ref: "grep -c 'onNavigateToSite' src/newui/NewShell.tsx (1, expects >0)"
        status: pass
      - kind: other
        ref: "grep -c 'View full health' src/newui/components/CredentialBlock.tsx (1, expects >0)"
        status: pass
      - kind: other
        ref: "grep -c 'updateNavCollapsed' src/newui/NewShell.tsx (1, expects >0)"
        status: pass
      - kind: unit
        ref: "npm test (18/18 pass)"
        status: pass
      - kind: other
        ref: "npm run build"
        status: pass
    human_judgment: false
  - id: D4
    description: "Live interactive verification: all six sidebar entries switch the right-hand column correctly with a working Back control; sidebar shows no favourites/folders/item-type/sync/history entry; add/rename a site and see the list update immediately; add/delete a credential and see the panel update immediately with no confirm dialog on delete, then see it in Trash; delete a site and see the exact confirm copy plus the detail column returning to its no-selection state; clicking a weak entry in Health lands on that site; clicking 'View full health' opens the Health pane; sidebar collapse persists across restart; lock button returns to login"
    verification: []
    human_judgment: true
    rationale: "Task 3's <verify> carries a <human-check> requiring a live desktop npm run tauri dev walkthrough (ui_variant: new) that automated build/test/grep checks cannot substitute for - confirming the six destinations actually switch panes correctly, that CRUD round-trips are visible in the running app without reselection, that the site-delete confirm copy is character-exact, that Health navigation and the health link-out actually navigate, and that collapse/lock persist across a real restart and lock cycle. Per workflow.human_verify_mode=end-of-phase (default, no .planning/config.json override present), this is NOT executed as a mid-flight interactive checkpoint - deferred to the end-of-phase UAT harvest, consistent with 08-01 through 08-04's own deferrals in this same phase."

duration: ~8min
completed: 2026-09-14
status: complete
commits: 3
---

# Phase 8 Plan 5: Vault CRUD Wiring and Secondary Panes Summary

**Every site/credential create-edit-delete path now runs through the existing SiteEditor/CredentialEditor overlays, and all five secondary screens (Health/Generator/Trash/Data/Settings) are reachable from the sidebar as thin passthrough wrappers into the detail column — closing both gaps plan 08-04 left open.**

## Performance

- **Duration:** ~8 min
- **Started:** 2026-09-14T16:00:00Z (approximate, immediately after 08-04's completion)
- **Completed:** 2026-09-14T16:08:00Z
- **Tasks:** 3 (all `type="auto"`)
- **Files modified:** 8 (5 created, 3 modified)

## Accomplishments

- `src/newui/NewShell.tsx`: mounts `SiteEditor` (add/edit/delete) and `CredentialEditor` (add/edit) at shell level above the three-pane layout; adds `refreshAfterSave()` (refreshes the site list and, when a site is selected, its credentials); `SiteEditor`'s `onDeleted` clears the selection so the detail column returns to its no-selection empty state; dispatches all six sidebar destinations into the detail column (`DetailPanel`/empty-state for `vault`, `HealthPane`/`GeneratorPane`/`TrashPane`/`DataPane`/`SettingsPane` for the other five), each pane receiving `goVault` as `onBack`; `HealthPane`'s `onNavigateToSite` sets the screen back to `"vault"` and selects the given site
- `src/newui/components/DetailPanel.tsx`: gains `Edit site` and `Add credential` header controls (both `Tooltip`-wrapped) and threads `onEditCredential`/`onCredentialDeleted`/`onOpenHealth` down to each `CredentialBlock`
- `src/newui/components/CredentialBlock.tsx`: gains per-credential `Edit` and `Delete` controls; delete calls `api.softDeleteCredential` with no confirmation dialog (soft delete, restorable from Trash), matching `SiteDetail.tsx`'s existing behaviour exactly; strength block gains a `View full health` link-out beside the meter, calling the new `onOpenHealth` prop instead of fabricating a per-item breach/reuse status (D-17)
- `src/newui/screens/GeneratorPane.tsx`, `HealthPane.tsx`, `DataPane.tsx`, `TrashPane.tsx`, `SettingsPane.tsx`: five one-line passthrough wrappers over the existing `Generator`/`HealthDashboard`/`Data`/`Trash`/`Settings` screens, each importing its screen unchanged from `../../screens/` and forwarding props 1:1 with no added layout

## Task Commits

1. **Task 1: Site and credential CRUD wired to the existing editor overlays** - `3f33bbd` (feat)
2. **Task 2: Five secondary screens re-homed into the detail column** - `501a3c2` (feat)
3. **Task 3: Sidebar nav dispatch and the health link-out** - `ca1e643` (feat)

**Plan metadata:** commit pending (this SUMMARY + STATE.md/ROADMAP.md)

## Files Created/Modified

- `src/newui/screens/GeneratorPane.tsx` - `GeneratorPane`: passthrough to `Generator`
- `src/newui/screens/HealthPane.tsx` - `HealthPane`: passthrough to `HealthDashboard`, threads `onBack`/`onNavigateToSite`
- `src/newui/screens/DataPane.tsx` - `DataPane`: passthrough to `Data`
- `src/newui/screens/TrashPane.tsx` - `TrashPane`: passthrough to `Trash`
- `src/newui/screens/SettingsPane.tsx` - `SettingsPane`: passthrough to `Settings`, threads `onBack`/`username`
- `src/newui/NewShell.tsx` - editor-overlay state, `refreshAfterSave`, `goVault`, full six-way screen dispatch
- `src/newui/components/DetailPanel.tsx` - `Edit site`/`Add credential` header controls, callback threading to `CredentialBlock`
- `src/newui/components/CredentialBlock.tsx` - per-credential `Edit`/`Delete` controls, `View full health` link-out

## Decisions Made

See `key-decisions` in frontmatter. Summary:
1. `ItemList.tsx` needed no code change - its `onAddSite` wiring was already correct as of plan 08-03, confirmed by reading the file before editing rather than assuming the plan's file list implied a required change.
2. `CredentialBlock` gained its own local error state to surface a failed delete, using the same `isCommandError`/generic-fallback convention used everywhere else in this codebase.
3. Callback threading (`onEditCredential`, `onCredentialDeleted`, `onOpenHealth`) follows the existing DetailPanel/CredentialBlock props boundary from plan 08-04 rather than having leaf components import editors or reach into shell state directly.

## Deviations from Plan

None - plan executed exactly as written. The three key-decisions above are resolutions of ambiguity within the plan's own stated contracts (the file already satisfying an instruction, the plan's own "shared catch fallback" and "thread callbacks down as props" instructions), not departures from it.

## Issues Encountered

- Task 3's `<verify>` carries a `<human-check>` (desktop `npm run tauri dev` walkthrough covering all six destinations, CRUD round-trips, the exact site-delete confirm copy, Health navigation, the health link-out, collapse persistence across restart, and lock). Per `workflow.human_verify_mode` defaulting to `end-of-phase` (no `.planning/config.json` override present), this was NOT executed as a mid-flight interactive checkpoint - deferred to the end-of-phase UAT harvest, consistent with 08-01 through 08-04's own deferrals in this same phase.
- No repository-state anomaly recurred this run (branch was `feature/bitwarden-port-phase1` at the correct head throughout, confirmed at session start and again before each commit) - the anomaly flagged in 08-04's SUMMARY did not reproduce.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Plan 08-05 is complete: the new UI is now functionally complete against today's backend for every CRUD path and every sidebar destination - every flow in `docs/UI_REDESIGN_CAPABILITY_CHECKLIST.md` §1 is reachable from it, and every reused screen/editor was reached by wrapping, never by copying (`git diff --quiet HEAD -- src/screens` passed after every task).
- Ready for plan 08-06 (responsive tiers - the two narrower breakpoints `NewShell` still owes per D-18: 640-1023px icon-rail sidebar, <640px single-pane switch via `BottomTabBar`/`NavDrawer`).
- Deferred to end-of-phase UAT: Task 3's `<human-check>` (the full live walkthrough described above), consistent with 08-01 through 08-04's own deferrals under the same `end-of-phase` human-verify mode.
- **Requirement-tracking note carried forward from 08-01 through 08-04's own "Requirement Scope Note":** VAULT-03 as literally worded in REQUIREMENTS.md (item types, favorites, folders) is NOT delivered by this plan or this phase - D-10 cut them for lack of backend persistence. This plan delivers the CRUD-wiring and nav-completeness slice of VAULT-03's real scope (every site/credential create/edit/delete path, all six sidebar destinations reachable). `requirements-completed` above copies the plan's frontmatter `requirements` array verbatim per established precedent; the shared-ID gate (`requirements.ready-ids`) still governs when VAULT-03 actually flips to Complete in REQUIREMENTS.md.

---
*Phase: 08-new-vault-ui-figma-port*
*Completed: 2026-09-14*

## Self-Check: PASSED

All 8 key files verified present on disk (`[ -f ]`), and all 3 task commits (`3f33bbd`, `501a3c2`, `ca1e643`) verified present in `git log --oneline --all`.
