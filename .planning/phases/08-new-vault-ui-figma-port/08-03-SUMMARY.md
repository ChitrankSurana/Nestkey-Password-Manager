---
phase: 08-new-vault-ui-figma-port
plan: 03
subsystem: ui
tags: [react, tailwind, vitest, tdd]

# Dependency graph
requires:
  - phase: 08-01
    provides: "src/newui/NewApp.tsx tracer branch (VaultTracer) replaced by NewShell; the third Vite entry point and real-backend wiring this plan builds on top of"
provides:
  - "src/newui/lib/vaultSelectors.ts: sitesForResults, itemCountLabel, listEmptyState - pure list-pane logic, unit tested"
  - "src/newui/hooks/useVault.ts: useVault() - single owner of vault fetch state, enforcing the lazy-decryption invariant (D-09)"
  - "src/newui/NewShell.tsx: three-pane >=1024px layout (Sidebar + 280px ItemList + flex-1 detail column)"
  - "src/newui/components/Sidebar.tsx, SidebarNavItem.tsx, ItemList.tsx, ItemRow.tsx, PaneSurface.tsx: the new UI's own chrome, built fresh (no prior analog)"
affects: [08-04, 08-05, 08-06]

# Actuals (#2632)
actuals:
  tokens: 6085
  tasks: 3
  commits: 4
plan_head_before: 86223c95cea9b9e9692ea2fbdd7572317d267379

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Pure-logic module (no React, no api import) for list-pane projection/copy, unit tested under vitest's node environment, following src/lib/clipboard.test.ts's describe/it/expect style"
    - "Hook-as-single-owner: useVault() lifts Vault.tsx's refreshSites/runSearch and SiteDetail.tsx's Promise.all dual-fetch logic (not JSX) into one hook that both ItemList and the future DetailPanel (08-04) read from instead of owning their own fetch effects"
    - "Lazy-decryption invariant enforced structurally: exactly one call site for the credential-fetch command in the entire new UI, gated by a repeatable grep pair (hook file count == 1, everywhere else == 0)"
    - "Accent-family ramp (--accent-<step>) used for decorative accent-adjacent chrome (the Sidebar's logo chip) to keep the literal --accent token exclusively reserved for CTAs/selected-state tints, auditable by a single grep with zero false positives"

key-files:
  created:
    - src/newui/lib/vaultSelectors.ts
    - src/newui/lib/vaultSelectors.test.ts
    - src/newui/components/Sidebar.tsx
    - src/newui/components/SidebarNavItem.tsx
    - src/newui/components/PaneSurface.tsx
    - src/newui/NewShell.tsx
    - src/newui/hooks/useVault.ts
    - src/newui/components/ItemList.tsx
    - src/newui/components/ItemRow.tsx
  modified:
    - src/newui/NewApp.tsx

key-decisions:
  - "Sidebar's shield-lock-fill logo chip uses --accent-600 (a step from the existing 5-hue accent ramp App.css already aliases) rather than the literal --accent token, so the header block satisfies the UI-SPEC's 'accent background' instruction for the logo chip while keeping Task 2's acceptance criterion true: zero occurrences of the literal string bg-[var(--accent)] in Sidebar.tsx/SidebarNavItem.tsx, where --accent is reserved exclusively for CTAs and selected-state tints (UI-SPEC Color table)."
  - "NewShell's destinations array is wired with no-op onClick handlers for health/generator/trash/data/settings - this plan's file list (useVault/NewShell/ItemList/ItemRow/NewApp only) does not include the secondary-pane wrapper screens (GeneratorPane etc.) CONTEXT.md's directory layout schedules for a later plan, so there is nothing yet for those five destinations to navigate to. The buttons render, are tooltip-reachable (UX-01), and are part of the required six-entry D-10 set; wiring real navigation behind them is explicitly out of this plan's scope."
  - "ItemList.tsx imports and calls itemCountLabel/listEmptyState directly from ../lib/vaultSelectors, but does not import sitesForResults - visibleSites is read pre-computed from useVault() per the plan's key_links contract ('reads sites, visibleSites, query, error and selection from the hook instead of owning fetch effects'), and sitesForResults is the hook's own internal derivation. Re-importing it into ItemList would mean either an unused import (blocked by tsconfig's noUnusedLocals) or duplicating the hook's projection logic - both worse than the single owner the hook already is."
  - "Task 1 followed the TDD cycle (tdd=\"true\"): RED commit (failing test importing a module that did not yet exist) before GREEN commit (implementation). No REFACTOR commit - the first-pass implementation needed no cleanup."

requirements-completed: [VAULT-03, VAULT-04, VAULT-05]

coverage:
  - id: D1
    description: "Pure list-pane logic (sitesForResults dedup/projection, itemCountLabel pluralization, listEmptyState's four pinned UI-SPEC strings) is unit tested"
    requirement: VAULT-04
    verification:
      - kind: unit
        ref: "src/newui/lib/vaultSelectors.test.ts (6 cases: sitesForResults x3, itemCountLabel x1, listEmptyState x2)"
        status: pass
      - kind: other
        ref: "npm run build"
        status: pass
    human_judgment: false
  - id: D2
    description: "Sidebar/SidebarNavItem structural contract: accent-subtle active tint (never a solid accent fill), shield-lock-fill logo chip, every nav button wrapped in Tooltip+Icon, destinations rendered only from props (D-10)"
    requirement: VAULT-03
    verification:
      - kind: other
        ref: "grep -c 'accent-subtle-bg' src/newui/components/SidebarNavItem.tsx (expects >0)"
        status: pass
      - kind: other
        ref: "grep -c 'shield-lock-fill' src/newui/components/Sidebar.tsx (expects >0)"
        status: pass
      - kind: other
        ref: "grep -rn 'bg-\\[var(--accent)\\]' src/newui/components/Sidebar.tsx src/newui/components/SidebarNavItem.tsx (expects 0)"
        status: pass
      - kind: other
        ref: "npm run build"
        status: pass
    human_judgment: false
  - id: D3
    description: "Lazy-decryption invariant (D-09, threat register T-08-09/T-08-10): exactly one credential-fetch call site in the whole new UI (useVault's refreshCredentials), a vault-locked listener that clears credentials and the selection, and no direct Tauri core IPC import anywhere under src/newui/"
    requirement: VAULT-05
    verification:
      - kind: other
        ref: "grep -rc 'listCredentialsForSite' src/newui/hooks/useVault.ts (expects 1)"
        status: pass
      - kind: other
        ref: "grep -rn 'listCredentialsForSite' src/newui/components/ src/newui/NewShell.tsx (expects 0)"
        status: pass
      - kind: other
        ref: "grep -rn 'from \"@tauri-apps/api/core\"' src/newui/ (expects 0)"
        status: pass
      - kind: other
        ref: "npm test"
        status: pass
      - kind: other
        ref: "npm run build"
        status: pass
    human_judgment: false
  - id: D4
    description: "NewShell wires exactly the six real nav destinations (vault/health/generator/trash/data/settings, copied from AppShell.tsx); NewApp's vault branch renders NewShell and no longer calls listSites itself"
    requirement: VAULT-03
    verification:
      - kind: other
        ref: "grep -oE 'key: \"[a-z]+\"' src/newui/NewShell.tsx (6 matches: vault, health, generator, trash, data, settings)"
        status: pass
      - kind: other
        ref: "grep -c 'NewShell' src/newui/NewApp.tsx (expects >0)"
        status: pass
      - kind: other
        ref: "grep -c 'listSites' src/newui/NewApp.tsx (expects 0)"
        status: pass
      - kind: other
        ref: "npm run build"
        status: pass
    human_judgment: false
  - id: D5
    description: "Live three-pane experience at desktop width: sidebar with shield chip/username/six destinations, 280px list with real sites (globe chip, name, url), live de-duplicated search with no lag, selected-row accent-subtle tint, long name/url truncation, correct light-theme background"
    verification: []
    human_judgment: true
    rationale: "The automated checks (build, unit tests, structural greps) confirm code shape and the security-critical lazy-decryption invariant, but not the live rendered experience - that the three panes actually lay out correctly at >=1024px, live search updates on keystroke with de-duplicated rows for a site with two credentials, a long name/url truncates with an ellipsis instead of widening the 280px pane, the selected row takes the accent-subtle tint, and switching the theme to light in Classic's Settings and restarting into the new UI shows the light background rather than the dark palette. This is the plan's own <verify><human-check>, deferred to end-of-phase UAT per workflow.human_verify_mode=end-of-phase (default, no .planning/config.json override present in this project), matching 08-01's D4/D5 and 08-02's D3 deferral pattern."

duration: ~24min
completed: 2026-09-14
status: complete
commits: 4
---

# Phase 8 Plan 3: Three-Pane Vault Shell Summary

**Sidebar + searchable 280px site list + detail column, backed by a single `useVault()` hook that is the only place in the new UI allowed to decrypt a credential — replacing plan 08-01's tracer view with the real shell every later plan builds into.**

## Performance

- **Duration:** ~24 min
- **Started:** 2026-09-14T15:20:00Z (approximate)
- **Completed:** 2026-09-14T15:44:26Z
- **Tasks:** 3 (Task 1 `tdd="true"`, Tasks 2-3 `type="auto"`)
- **Files modified:** 10 (9 created, 1 modified)

## Accomplishments

- `src/newui/lib/vaultSelectors.ts` + `.test.ts`: pure `sitesForResults`/`itemCountLabel`/`listEmptyState` functions, TDD RED→GREEN, pinning the four UI-SPEC empty-state strings literally and the search-result-to-site dedup/collapse rule (D-08 - one row per site even when a site has two matching credentials)
- `src/newui/hooks/useVault.ts`: single owner of vault fetch state — sites load eagerly with no secrets, credentials load only on selection via the one `refreshCredentials` call site in the whole new UI, and both credentials and the selection clear on deselect and on the `vault-locked` event (D-09 lazy-decryption invariant, structurally enforced and grep-gated)
- `src/newui/components/Sidebar.tsx` / `SidebarNavItem.tsx`: the vault-branded sidebar chrome — shield-lock-fill logo chip, `Vault`/`{username}` header block, the six real destinations mapped from props only (never hardcoded, D-10), accent-subtle selection tint (never a solid accent fill), collapse toggle, lock button
- `src/newui/components/PaneSurface.tsx`: the shared card/surface wrapper this codebase never factored out
- `src/newui/components/ItemList.tsx` / `ItemRow.tsx`: the 280px searchable site list — real-time no-debounce search (carrying forward `Vault.tsx`'s load-bearing rationale comment), globe2 icon chip, truncated name/url, loading indicator, and the pinned browse/search empty states consumed from `vaultSelectors`
- `src/newui/NewShell.tsx`: the three-pane `>=1024px` layout wiring `Sidebar` + `ItemList` + a detail column (no-selection empty state now; plan 08-04 replaces it with `DetailPanel`), config-driven collapse persistence, `handleLock`, and the add-site flow via the existing `SiteEditor` overlay
- `src/newui/NewApp.tsx`: vault branch now renders `<NewShell username={username!} />`, replacing plan 08-01's `VaultTracer`

## Task Commits

1. **Task 1 RED: failing test for vault list-pane selectors** - `465105b` (test)
2. **Task 1 GREEN: implement vault list-pane selectors** - `15e7d35` (feat)
3. **Task 2: sidebar chrome and shared pane surface** - `6da6776` (feat)
4. **Task 3: three-pane shell over a lazy-decryption hook** - `eedd1c1` (feat)

No REFACTOR commit for Task 1 — the first-pass implementation needed no cleanup.

**Plan metadata:** commit pending (this SUMMARY + STATE.md/ROADMAP.md)

## Files Created/Modified

- `src/newui/lib/vaultSelectors.ts` - `sitesForResults`, `itemCountLabel`, `listEmptyState`: pure list-pane logic, no React/no `api` import
- `src/newui/lib/vaultSelectors.test.ts` - 6 unit tests pinning the projection rule and the four empty-state strings
- `src/newui/hooks/useVault.ts` - `useVault()`: sites/results/visibleSites/query/selection/credentials state, the one `listCredentialsForSite` call site, the `vault-locked` clear-on-lock listener
- `src/newui/components/Sidebar.tsx` - vault-branded sidebar: logo chip, header, nav list, collapse toggle, lock button
- `src/newui/components/SidebarNavItem.tsx` - one tooltip-wrapped, accent-subtle-tinted nav button
- `src/newui/components/PaneSurface.tsx` - shared card/surface wrapper (`var(--surface)`/`var(--border)`/`rounded-lg`)
- `src/newui/components/ItemList.tsx` - list pane: header, count, add-site CTA, search box, rows, empty/loading states
- `src/newui/components/ItemRow.tsx` - one site row: globe2 chip, truncated name, truncated url
- `src/newui/NewShell.tsx` - three-pane `>=1024px` layout, destinations array, collapse/lock wiring, add-site flow
- `src/newui/NewApp.tsx` - vault branch renders `NewShell` instead of the plan 08-01 `VaultTracer`

## Decisions Made

See `key-decisions` in frontmatter. Summary:
1. Sidebar's logo chip uses `--accent-600` (accent ramp step) instead of the literal `--accent` token, keeping `--accent` exclusively reserved for CTAs/selected-state tints while still giving the chip an accent-family background per the UI-SPEC.
2. `NewShell`'s five non-vault destinations have no-op `onClick` handlers — their secondary panes are out of this plan's file list and land in a later plan; the buttons still render and are tooltip-reachable per D-10/UX-01.
3. `ItemList.tsx` imports `itemCountLabel`/`listEmptyState` directly but not `sitesForResults`, since `visibleSites` is read pre-computed from `useVault()` per the plan's own key_links contract.
4. Task 1 followed the full TDD RED→GREEN cycle (no REFACTOR needed).

## Deviations from Plan

None — plan executed exactly as written. The three key-decisions above are all resolutions of ambiguity within the plan's own stated contracts (UI-SPEC's "accent background" instruction vs. Task 2's literal-string acceptance criterion; the plan's own key_links contract for which functions ItemList consumes), not departures from it, and are documented above rather than as Rule 1-4 deviations.

## Issues Encountered

- Task 2's initial implementation of the Sidebar logo chip used the literal `bg-[var(--accent)]` class, which passed the build but failed the acceptance criterion's grep (`bg-[var(--accent)]` must not appear in either sidebar file — that token is reserved for CTAs/selected-state tints per UI-SPEC Color). Fixed by switching to `--accent-600` from the existing accent ramp; re-ran all four Task 2 verification checks after the fix, all passed.
- Task 3's `useVault.ts` docstring initially mentioned `api.listCredentialsForSite` in a comment, which made the "exactly one occurrence" grep count 2 instead of 1 (the grep has no comment-line filter, unlike the sibling "zero elsewhere" check). Reworded the comment to describe the invariant without repeating the literal command name; re-ran the check, count is 1.
- Task 3's `<verify>` carries a `<human-check>` (desktop `npm run tauri dev` walkthrough of the three-pane layout, live search, truncation, and light-theme background). Per `workflow.human_verify_mode` defaulting to `end-of-phase` (no `.planning/config.json` override present), this was NOT executed as a mid-flight interactive checkpoint — deferred to the end-of-phase UAT harvest, consistent with 08-01's D4/D5 and 08-02's D3 deferrals.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Plan 08-03 is complete: the new UI's vault screen is a real three-pane shell (sidebar, searchable list, detail column) backed by `useVault()`, with the lazy-decryption invariant structurally enforced and grep-gated.
- Ready for plan 08-04 (detail column: `DetailPanel`/`CredentialBlock`/`DetailField`/`CopyButton`, replacing `NewShell`'s current no-selection-only detail column) and the secondary-pane wrapper screens (`GeneratorPane` etc.) that will give the five non-vault sidebar destinations real content.
- Deferred to end-of-phase UAT: Task 3's `<human-check>` (live three-pane layout, search de-dup, truncation, selected-row tint, light-theme background at `>=1024px`), consistent with 08-01/08-02's own deferrals under the same `end-of-phase` human-verify mode.

---
*Phase: 08-new-vault-ui-figma-port*
*Completed: 2026-09-14*

## Self-Check: PASSED

All 10 key files verified present on disk (`[ -f ]`), and all 4 task commits (`465105b`, `15e7d35`, `6da6776`, `eedd1c1`) verified present in `git log --oneline --all`.
