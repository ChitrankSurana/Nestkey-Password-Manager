---
phase: 08-new-vault-ui-figma-port
plan: 06
subsystem: ui
tags: [react, tailwind, vitest, responsive, tauri]

requires:
  - phase: 08-new-vault-ui-figma-port
    provides: 08-05's full CRUD wiring and secondary panes over NewShell/Sidebar/ItemList/DetailPanel
provides:
  - Mid-tier (640-1023px) icon-rail sidebar in NewShell, sole breakpoint owner per D-18
  - Narrow-tier (<640px) mobile nav swap (BottomTabBar/NavDrawer) with single-visible-pane and DetailPanel back control
  - src/newui/lib/parity.test.ts standing invariant gate (7 assertions) over the whole new UI
affects: [any future new-UI phase touching src/newui — the parity gate must keep passing]

actuals:
  tokens: 9000
  tasks: 3
  commits: 3

tech-stack:
  added: []
  patterns:
    - "NewShell as sole Tailwind sm:/lg: breakpoint owner; no other src/newui file branches on viewport width"
    - "Source-invariant gate: a vitest suite reading a directory tree from disk with node:fs/readdirSync and regex-asserting cross-cutting invariants against comment-stripped source text"

key-files:
  created:
    - src/newui/lib/parity.test.ts
  modified:
    - src/newui/NewShell.tsx
    - src/newui/components/Sidebar.tsx
    - src/newui/components/ItemList.tsx
    - src/newui/components/DetailPanel.tsx

key-decisions:
  - "Mid tier forces Sidebar into icon-rail form via a showToggle=false prop rather than a width-aware Sidebar, keeping Sidebar itself width-agnostic and NewShell the only breakpoint owner (D-18)."
  - "Mid tier never calls api.updateNavCollapsed, so a narrow window is a pure layout consequence and never overwrites the user's saved navCollapsed preference."
  - "Narrow tier reuses BottomTabBar/NavDrawer from src/screens/ unmodified (D-11), copying AppShell's three-way mobileNavStyle branch verbatim including its icon-rail default case."
  - "Parity gate matches @tauri-apps/api/core specifically (not the whole @tauri-apps/api scope) so the allowed event-listener import used by NewApp/useVault doesn't self-trip the Tauri-IPC-isolation assertion."

patterns-established:
  - "Pattern: standing source-invariant gates — for behaviors that regress silently in a rewrite (clipboard seam, reprompt gate, lazy-decryption call count, scope cuts), assert against the actual source tree on every test run instead of relying on convention."

requirements-completed: []

coverage:
  - id: D1
    description: "Mid tier (640-1023px): Sidebar renders as an icon rail, ItemList/detail unchanged, saved navCollapsed preference untouched"
    requirement: "VAULT-03"
    verification:
      - kind: unit
        ref: "grep showToggle prop in Sidebar.tsx / lg: prefix in NewShell.tsx"
        status: pass
    human_judgment: true
    rationale: "Grep checks prove the prop and breakpoint exist; actual visual behavior across a resized window requires the deferred end-of-phase UAT walkthrough."
  - id: D2
    description: "Narrow tier (<640px): mobileNavStyle-selected BottomTabBar/NavDrawer swap-in, single visible pane switched on selectedSiteId, DetailPanel back control"
    requirement: "VAULT-04"
    verification:
      - kind: unit
        ref: "npm test (25 passed) and npm run build"
        status: pass
    human_judgment: true
    rationale: "Automated checks confirm the components are wired and src/screens is untouched; the interactive nav-swap and back-navigation experience is deferred to end-of-phase UAT per workflow.human_verify_mode=end-of-phase."
  - id: D3
    description: "src/newui/lib/parity.test.ts: seven-invariant standing gate (Tauri IPC isolation, clipboard seam, lazy-decryption call count, hidden-field reprompt, no length-proportional mask, six-entry nav, D-10 scope-cut absence)"
    verification:
      - kind: unit
        ref: "npx vitest run src/newui/lib/parity.test.ts (7 passed)"
        status: pass
      - kind: unit
        ref: "npm test (25 passed), npm run build (exit 0), cargo test --workspace (all passed)"
        status: pass
    human_judgment: false
  - id: D4
    description: "Full 18-flow capability walkthrough (both New and Classic UI) and the six §4 cross-cutting behaviours across all three D-18 tiers"
    verification: []
    human_judgment: true
    rationale: "Interactive human-check block in the plan's Task 3 <verify> — requires a live npm run tauri dev session, deferred to end-of-phase UAT (workflow.human_verify_mode=end-of-phase, consistent with 08-01 through 08-05)."

duration: resumed from paused session
completed: 2026-09-14
status: complete
---

# Phase 8: New Vault UI (Figma Port) — Plan 06 Summary

**Completes the Figma-ported vault UI: NewShell now owns all three D-18 responsive tiers (full sidebar, icon-rail, mobile-nav-swap-with-single-pane), and a standing parity.test.ts gate asserts the phase's seven security/scope invariants against the live source tree.**

## Performance

- **Tasks:** 3/3 complete
- **Files modified:** 5 (1 created, 4 modified)

## Accomplishments
- Mid tier (640-1023px): `Sidebar` gained a `showToggle` prop; `NewShell` renders the icon-rail form via Tailwind `sm:`/`lg:` prefixes without ever calling `api.updateNavCollapsed`.
- Narrow tier (<640px): `NewShell` swaps in `BottomTabBar`/`NavDrawer` per the user's `mobileNavStyle`, collapses `ItemList`/`DetailPanel` to a single visible pane on `selectedSiteId`, and `DetailPanel` gained a `sm:hidden` back control.
- `src/newui/lib/parity.test.ts` created: 7 `it` blocks reading `src/newui/` from disk and asserting the Tauri-IPC boundary, clipboard seam, lazy-decryption call count, hidden-field reprompt coverage, mask construction, six-entry nav, and D-10 scope-cut absence — all passing.

## Task Commits

1. **Task 1: Mid tier — icon-rail sidebar between 640px and 1023px** - `a1cb36c` (feat)
2. **Task 2: Narrow tier — mobile nav swap and single visible pane below 640px** - `904e428` (feat)
3. **Task 3: Standing invariant gate over the new UI's sources** - `0dd2a6a` (feat)

**Plan metadata:** this file's commit (docs: complete plan)

_Note: this plan's execution spanned a session pause/resume (`.continue-here.md`); Task 3's test file was written before the pause and verified/committed on resume rather than rewritten._

## Files Created/Modified
- `src/newui/lib/parity.test.ts` - standing 7-invariant source gate over `src/newui/`
- `src/newui/NewShell.tsx` - sole breakpoint owner; all three D-18 tiers, mobileNavStyle branch, drawerOpen state
- `src/newui/components/Sidebar.tsx` - `showToggle` prop for the icon-rail tier
- `src/newui/components/ItemList.tsx` - width now supplied by NewShell's wrapper
- `src/newui/components/DetailPanel.tsx` - `sm:hidden` back control with `onBack` prop

## Decisions Made
- Mid tier forces `Sidebar` collapse via a prop rather than giving `Sidebar` its own width awareness, keeping every viewport decision inside `NewShell` (D-18's single-owner requirement).
- Parity gate's Tauri-IPC check matches `@tauri-apps/api/core` specifically so the legitimate `@tauri-apps/api/event` import isn't a false positive.

## Deviations from Plan
None - plan executed as written. Execution spanned a pause/resume; on resume, Task 3's already-written `parity.test.ts` was verified against the plan spec rather than rewritten, then committed.

## Issues Encountered
None. All automated verification passed on resume: `npx vitest run src/newui/lib/parity.test.ts` (7/7), `npm test` (25/25), `npm run build` (exit 0), `cargo test --workspace` (all passed), and both diff guards (`package.json`/`vitest.config.ts` unchanged, `src/screens` unchanged).

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Both interfaces (Classic and New) build and pass their automated suites. The new UI's security/scope invariants are now enforced by `parity.test.ts` on every `npm test`.
- Outstanding: the end-of-phase UAT walkthrough (18 capability flows + 6 cross-cutting behaviours + 3 responsive tiers + Classic-unchanged confirmation) is deferred per `workflow.human_verify_mode=end-of-phase`, consistent with plans 08-01 through 08-05. This must run before phase 8 is considered fully done by a human, though phase-goal verification and roadmap completion can proceed per the workflow's human_needed routing.
- VAULT-03/04/05 and HEALTH-02 remain open in REQUIREMENTS.md per the scope note in `08-01-PLAN.md` (item types/favorites/folders have no backend persistence; health per-item breach/reuse status is a full-vault sweep not yet exposed per-item) — not oversights, a recorded and enforced scope reduction.

---
*Phase: 08-new-vault-ui-figma-port*
*Completed: 2026-09-14*
