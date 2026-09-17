---
phase: 08-new-vault-ui-figma-port
plan: 02
subsystem: ui
tags: [tauri, react, axum, config, settings]

# Dependency graph
requires:
  - phase: 08-01
    provides: "config.ui_variant field, do_update_ui_variant core setter, desktop update_ui_variant Tauri command + invoke_handler registration"
provides:
  - "/api/update_ui_variant POST route on the web portal (UiVariantRequest + handler), completing the setter trail 08-01 left open"
  - "api.updateUiVariant wrapper and ConfigDto.uiVariant field in src/lib/api.ts"
  - "Settings 'Interface' section: select control + persistent (non-toast) restart/open-URL banner"
affects: [08-03, 08-04, 08-05, 08-06]

# Actuals (#2632)
actuals:
  tokens: 1632
  tasks: 2
  commits: 2

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Web-portal setter route add: request struct -> async handler delegating to the core do_* function -> route registration in the same session/CSRF-protected router chain as its siblings (mirrors update_mobile_nav_style exactly)"
    - "Settings config-select pattern: OPTIONS const -> useState synced from getConfig() under an editedRef guard -> handleXChange (editedRef=true, submitting, api call, catch, finally) -> <details><label><Tooltip><select> markup"
    - "Persistent (non-dismissing) inline banner distinct from ToastHost: a dedicated boot-value ref (not the edited-vs-not editedRef) compared against current state to decide visibility, styled with the existing inline-message convention (text-sm rounded border px-2 py-1 border-[var(--border)]), platform-branched copy via isTauri"

key-files:
  created: []
  modified:
    - web-src-tauri/src/handlers.rs
    - src/lib/api.ts
    - src/screens/Settings.tsx

key-decisions:
  - "git stash@{0} ('wip: pre-existing uncommitted ui_variant scaffolding before phase 8 execution') was inspected via `git stash show -p` and found fully superseded: it contained only VALID_UI_VARIANTS + default_ui_variant() in core/src/config.rs, a strict subset of what 08-01 already implemented fresh on disk (full 6-step field-add pattern: const, default fn, struct field, Default impl, load() snap-to-default, both test literals, 3 tests, all confirmed present via grep before dropping). Dropped with `git stash drop stash@{0}` - nothing from it needed merging."
  - "Task 1's desktop half (src-tauri/src/commands.rs update_ui_variant command, src-tauri/src/lib.rs invoke_handler registration) was already completed in 08-01 as part of its checkpoint-directed runtime-failure escape hatch. Verified via grep before starting rather than re-implementing; this plan's Task 1 work was narrowed to only the web-src-tauri/src/handlers.rs route (UiVariantRequest struct, handler, /api/update_ui_variant registration) that 08-01 explicitly left out of scope."
  - "Human-check verification for Task 2 (interactive select + banner persistence + desktop-restart vs. web-/newui.html copy, live in npm run tauri dev and web mode) deferred to end-of-phase UAT, consistent with workflow.human_verify_mode defaulting to end-of-phase (no .planning/config.json override present in this project) and matching how 08-01's own human-check items (D4/D5) were deferred."

requirements-completed: [VAULT-03]

coverage:
  - id: D1
    description: "update_ui_variant is registered as a POST route on the web portal (web-src-tauri/src/handlers.rs), in the same session-and-CSRF-protected router chain as its sibling setters, delegating to the core do_update_ui_variant validator"
    requirement: VAULT-03
    verification:
      - kind: unit
        ref: "cargo test --workspace (pwmgr_web unit + http_integration suites, 22 tests, all pass)"
        status: pass
      - kind: other
        ref: "grep -c '/api/update_ui_variant' web-src-tauri/src/handlers.rs (expects >0)"
        status: pass
      - kind: other
        ref: "grep -c 'struct UiVariantRequest' web-src-tauri/src/handlers.rs (expects >0)"
        status: pass
    human_judgment: false
  - id: D2
    description: "src/lib/api.ts exposes ConfigDto.uiVariant and the api.updateUiVariant(variant) wrapper, reusing call<T>()'s existing dual IPC/HTTP transport"
    requirement: VAULT-03
    verification:
      - kind: other
        ref: "npm run build (tsc + vite build), exit 0"
        status: pass
      - kind: other
        ref: "grep -c 'updateUiVariant' src/lib/api.ts (expects >=1)"
        status: pass
      - kind: other
        ref: "grep -c 'uiVariant: string;' src/lib/api.ts (expects >0)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Settings gains an 'Interface' <details> section (select bound to UI_VARIANT_OPTIONS) between Appearance and Vault, with a persistent (non-toast) banner telling desktop users to restart and web-portal users to open /newui.html, driven by a dedicated bootUiVariantRef distinct from editedRef"
    requirement: VAULT-03
    verification:
      - kind: other
        ref: "npm run build (tsc compiles the new section/state/handler cleanly), exit 0"
        status: pass
      - kind: other
        ref: "grep -c 'bootUiVariantRef' src/screens/Settings.tsx (expects >0)"
        status: pass
      - kind: other
        ref: "grep -c 'Restart the app to see it.' src/screens/Settings.tsx (expects >0)"
        status: pass
      - kind: other
        ref: "grep -c 'Open /newui.html in this browser to use it.' src/screens/Settings.tsx (expects >0)"
        status: pass
      - kind: other
        ref: "grep -c 'toast(' src/screens/Settings.tsx Interface section (expects 0 — confirmed via full-file grep returning 0 matches)"
        status: pass
    human_judgment: true
    rationale: "The automated checks confirm the code shape (select renders, banner strings exist, no toast() call) but not the live interactive behavior — that changing the select actually persists via the setter, the banner appears immediately and survives well past ToastHost's 2000ms auto-dismiss window, collapsing/reopening the Settings section keeps it, restarting the desktop app opens the chosen UI and clears the banner, and the web-portal path names /newui.html without switching the browser tab's own UI. Deferred to end-of-phase UAT per workflow.human_verify_mode=end-of-phase (default, no config.json override present), matching 08-01's own D4/D5 deferral pattern."

duration: ~20min
completed: 2026-09-14
status: complete
commits: 2
plan_head_before: 4319ac813ca9f84a1a1fd37baf878db41a3b2a93
---

# Phase 8 Plan 2: Interface Setter Wiring & Settings UI Summary

**Completed the `update_ui_variant` setter trail 08-01 left open — a web-portal POST route, an `api.ts` wrapper, and a Settings "Interface" select with a persistent (non-toast) restart/open-URL banner — turning the config field 08-01 built into a real, discoverable Settings control.**

## Performance

- **Duration:** ~20 min
- **Started:** 2026-09-14T20:40:00+05:30 (approximate)
- **Completed:** 2026-09-14T21:02:00+05:30
- **Tasks:** 2 (both `type="auto"`)
- **Files modified:** 3 (`web-src-tauri/src/handlers.rs`, `src/lib/api.ts`, `src/screens/Settings.tsx`)

## Accomplishments

- Registered `UiVariantRequest`, an `update_ui_variant` handler, and the `/api/update_ui_variant` POST route in `web-src-tauri/src/handlers.rs`, adjacent to `/api/update_mobile_nav_style` in the same session-and-CSRF-protected router chain — the desktop IPC command and its `generate_handler!` registration were already done in 08-01, so this task's actual diff was narrower than the plan anticipated.
- Added `uiVariant: string;` to `ConfigDto` and `updateUiVariant: (variant: string) => call<void>("update_ui_variant", { variant })` to the `api` object in `src/lib/api.ts`.
- Added a new "Interface" `<details>` section to `src/screens/Settings.tsx` (between Appearance and Vault) with a `UI_VARIANT_OPTIONS` (classic/new) select, `handleUiVariantChange` mirroring `handleMobileNavStyleChange`'s shape, and a persistent inline banner — driven by a dedicated `bootUiVariantRef` (deliberately not `editedRef`) — that tells desktop users to restart and web-portal users to open `/newui.html` in that browser, per D-05/D-07.
- Resolved the outstanding `git stash@{0}` from before phase 8 execution began: inspected via `git stash show -p`, confirmed fully superseded by 08-01's complete field-add implementation already on disk, and dropped.

## Task Commits

1. **Task 1: Register the interface setter on the desktop and web binaries** - `1f22407` (feat) — web-portal route only; desktop half was already committed in 08-01 (`31b6096`).
2. **Task 2: Settings gains an Interface section with a persistent restart banner** - `39a6ce9` (feat)

**Plan metadata:** commit pending (this SUMMARY + STATE.md/ROADMAP.md)

## Files Created/Modified

- `web-src-tauri/src/handlers.rs` - `UiVariantRequest` struct, `update_ui_variant` async handler delegating to `do_update_ui_variant`, `/api/update_ui_variant` POST route registered next to `update_mobile_nav_style`
- `src/lib/api.ts` - `ConfigDto.uiVariant` field, `api.updateUiVariant(variant)` wrapper
- `src/screens/Settings.tsx` - `UI_VARIANT_OPTIONS` const, `uiVariant` state + `bootUiVariantRef`, `handleUiVariantChange`, new "Interface" `<details>` section with select and persistent banner

## Decisions Made

See `key-decisions` in frontmatter. Summary:
1. `git stash@{0}` was superseded by 08-01's own fresh implementation and dropped (nothing to merge).
2. Task 1's scope was narrowed at execution time to only the web-portal route, since 08-01 had already shipped the desktop IPC command and its registration ahead of schedule (documented in `08-01-SUMMARY.md`'s key-decisions). No plan deviation — this is exactly the "extend/wire to it instead of duplicating" instruction from the upstream context.
3. The Task 2 human-check (live interactive verification of the select + banner + restart/URL behavior) is deferred to end-of-phase UAT, matching `workflow.human_verify_mode=end-of-phase` (default) and 08-01's own precedent for its D4/D5 human-check items.

## Deviations from Plan

None — plan executed exactly as written, once accounting for the upstream context's explicit instruction that Task 1's desktop-command portion was already built in 08-01. This isn't a Rules 1-3 auto-fix or a Rule 4 architectural question; it's the plan's own acceptance criteria being satisfied by prior work, verified via `grep` before starting rather than re-implemented.

## Issues Encountered

None. `cargo test --workspace` (all crates, 22+ tests in `pwmgr_web` alone plus the full `pwmgr_core`/`tauri_app_lib` suites) and `npm run build` both passed clean on the first attempt for both tasks.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Plan 08-02 is complete: the interface preference is now a validated, discoverable Settings control on both desktop and web portal, with the setter trail (core → Tauri command → web route → `api.ts` → Settings UI) fully wired.
- The `git stash@{0}` scaffolding referenced in 08-01's resume instructions is resolved (dropped as superseded) — no dangling stash remains for future plans to reconcile.
- Deferred to end-of-phase UAT: Task 2's human-check (live select/banner behavior on both desktop and web, plus restart round-trip), consistent with 08-01's D4/D5 deferrals under the same `end-of-phase` human-verify mode.
- Ready for wave 2 continuation / wave 3: 08-03 (three-pane vault shell) can proceed independently; nothing in this plan blocks it.

---
*Phase: 08-new-vault-ui-figma-port*
*Completed: 2026-09-14*

## Self-Check: PASSED

All 4 key files verified present on disk (`[ -f ]`), and both task commits (`1f22407`, `39a6ce9`) verified present in `git log --oneline --all`.
