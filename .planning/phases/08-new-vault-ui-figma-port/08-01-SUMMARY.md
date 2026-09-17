---
phase: 08-new-vault-ui-figma-port
plan: 01
subsystem: ui
tags: [tauri, react, vite, tailwind, rust, config, window-management]

# Dependency graph
requires: []
provides:
  - Third Vite entry point (newui.html -> src/newui-main.tsx -> src/newui/NewApp.tsx) that boots
    and reaches the real backend through src/lib/api.ts
  - "Persisted config.ui_variant field (classic | new, default classic) with allowlist validation
    and snap-to-default on load"
  - Runtime desktop window construction from ui_variant, replacing the static tauri.conf.json
    windows array, window label fixed at "main"
  - Dual recovery paths so no New UI failure mode requires manual config-file editing (asset-guard
    fallback in Rust + a vanilla-DOM runtime-failure escape hatch in the frontend)
affects: [08-02, 08-03, 08-04, 08-05, 08-06]

# Actuals (#2632)
actuals:
  tokens: 5284
  tasks: 3
  commits: 2

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Third Vite rollup entry point mirroring the existing main/viewer two-entry pattern"
    - "Config field add: VALID_X const -> default_x() fn -> #[serde(default)] field -> Default impl -> snap-to-default in load() -> both test struct literals -> 3 analog tests"
    - "Runtime WebviewWindowBuilder construction from a validated config value, replacing a static tauri.conf.json windows array"
    - "AssetResolver::get() as a pre-flight existence check before pointing a window at a bundled entry asset"
    - "Vanilla-DOM (non-React) top-level error boundary registered before the app mounts, independent of the mounted tree's own state"

key-files:
  created:
    - newui.html
    - src/newui-main.tsx
    - src/newui/NewApp.tsx
  modified:
    - vite.config.ts
    - src/App.css
    - core/src/config.rs
    - core/src/dto.rs
    - core/src/commands.rs
    - src-tauri/src/commands.rs
    - src-tauri/src/lib.rs
    - src-tauri/tauri.conf.json

key-decisions:
  - "Checkpoint decision (Task 2): 'asset-guard, extended' - implement both the Rust-side asset-guard (missing/stale bundled asset falls back to Classic) AND a new in-app runtime-failure escape hatch (JS error after the asset loads falls back to Classic via a 'Switch to Classic UI' button), so no recovery path requires manual config-file editing. keybind-escape (global OS shortcut) explicitly not implemented - out of scope per the decision."
  - "Exposed a minimal update_ui_variant Tauri command in src-tauri/src/commands.rs one plan-cycle ahead of its full Settings-UI wiring (08-02, D-05/D-07), scoped narrowly to give the runtime-failure fallback something to call."
  - "The runtime-failure escape hatch lives in src/newui-main.tsx (not under src/newui/) as vanilla DOM, not React, and is registered before NewApp mounts - a deliberate exception to D-12's 'no direct IPC import under src/newui/' since the whole point is it must keep working even if the rest of the New UI's own code (including src/lib/api.ts call sites) is what's broken."
  - "Task 1's tracer feedback gate resolved via precedence-chain row 3 (interactive, end-of-phase mode, automated-only <verify>): re-ran all 5 automated checks, all passed, continued to Task 2 without a checkpoint."
  - "core/src/config.rs's VALID_UI_VARIANTS/default_ui_variant() were written fresh, not assumed pre-existing - the prior session's partial scaffolding for these lives in git stash@{0}, deliberately left untouched (out of scope for this plan, belongs to 08-02) rather than popped."

requirements-completed: [VAULT-03]

coverage:
  - id: D1
    description: "New UI entry point (newui.html/newui-main.tsx/NewApp.tsx) boots and reaches the real backend through src/lib/api.ts, with the static dark/indigo anti-flash attributes preserved"
    requirement: VAULT-03
    verification:
      - kind: other
        ref: "npm run build (tsc + vite build)"
        status: pass
      - kind: other
        ref: "grep -c 'data-accent=\"indigo\"' newui.html"
        status: pass
      - kind: other
        ref: "grep -c '@source \"../newui.html\";' src/App.css"
        status: pass
      - kind: other
        ref: "grep -rn 'from \"@tauri-apps/api/core\"' src/newui/ (expects 0)"
        status: pass
    human_judgment: false
  - id: D2
    description: "config.ui_variant field persists classic|new with allowlist validation and snap-to-default on load, mirroring the mobile_nav_style pattern"
    verification:
      - kind: unit
        ref: "core/src/config.rs#load_defaults_ui_variant_to_classic"
        status: pass
      - kind: unit
        ref: "core/src/config.rs#save_then_load_roundtrips_ui_variant"
        status: pass
      - kind: unit
        ref: "core/src/config.rs#load_rejects_invalid_ui_variant_by_falling_back_to_default"
        status: pass
    human_judgment: false
  - id: D3
    description: "do_update_ui_variant setter (core) and its update_ui_variant Tauri command wrapper reject out-of-allowlist values and persist valid ones without mutating state on save failure"
    verification:
      - kind: unit
        ref: "core/src/commands.rs#update_ui_variant_persists_and_rejects_invalid_value"
        status: pass
      - kind: unit
        ref: "core/src/commands.rs#get_config_includes_ui_variant_default"
        status: pass
      - kind: unit
        ref: "core/src/commands.rs#update_ui_variant_does_not_mutate_state_when_save_fails"
        status: pass
      - kind: other
        ref: "cargo build -p pwmgr-app"
        status: pass
    human_judgment: false
  - id: D4
    description: "Desktop window is built at runtime from ui_variant (WebviewWindowBuilder in .setup()) instead of tauri.conf.json's static windows array; window label stays the literal \"main\""
    verification:
      - kind: other
        ref: "grep -c 'WebviewWindowBuilder::new' src-tauri/src/lib.rs (expects 1)"
        status: pass
      - kind: other
        ref: "grep -c '\"main\"' src-tauri/src/lib.rs (expects >0)"
        status: pass
      - kind: other
        ref: "node check that src-tauri/tauri.conf.json app.windows is []"
        status: pass
    human_judgment: true
    rationale: "The automated checks confirm the code shape (one window builder, fixed label, empty static array) but not the actual launched behavior - that Classic opens unchanged at ui_variant classic, New opens at ui_variant new and shows the real site list, an out-of-allowlist value loads as classic, and clipboard/file-dialog access still works on the main-labelled window. This plan's Task 3 <verify> carries a <human-check> for exactly this, deferred to end-of-phase UAT per workflow.human_verify_mode=end-of-phase (default, no config.json override present in this project)."
  - id: D5
    description: "Dual recovery paths so a broken New UI never requires manual config-file editing: Rust-side asset-guard (missing bundled asset falls back to Classic before the window is even built) plus a frontend runtime-failure escape hatch (window error/unhandledrejection listeners + a try/catch around the initial render, independent of NewApp's own tree, rendering a 'Switch to Classic UI' button)"
    verification:
      - kind: other
        ref: "npm run build (tsc compiles src/newui-main.tsx's listeners/fallback UI without errors)"
        status: pass
    human_judgment: true
    rationale: "Confirming the fallback actually renders and its button actually recovers the session requires deliberately triggering a New UI failure inside a running app (npm run tauri dev) and clicking through it - not something a build/grep check can prove. Deferred to end-of-phase UAT alongside D4's human-check."

duration: 47min
completed: 2026-09-14
status: complete
---

# Phase 8 Plan 1: New Vault UI Entry Point Summary

**Third Vite entry point (`newui.html`) reaching the real backend via a tracer vault view, plus a runtime-selectable `ui_variant` config field that builds the desktop window at boot with two independent recovery paths — no failure mode requires manual config-file editing.**

## Performance

- **Duration:** 47 min
- **Started:** 2026-09-14T14:33:00Z (approximate — resumed from a prior paused executor; not separately timestamped at kickoff)
- **Completed:** 2026-09-14T15:20:34Z
- **Tasks:** 3 (Task 1 tracer, Task 2 checkpoint:decision, Task 3 auto)
- **Files modified:** 11 (3 created, 8 modified)

## Accomplishments

- Stood up `newui.html` / `src/newui-main.tsx` / `src/newui/NewApp.tsx` as a genuine third React root that boots, bootstraps theme from real config, listens for `vault-locked`, and — once logged in — fetches and lists the user's real sites through `src/lib/api.ts` (no direct Tauri IPC import anywhere under `src/newui/`)
- Added `config.ui_variant` (`"classic" | "new"`, default `"classic"`) to the Rust core following the exact `mobile_nav_style` pattern: allowlist const, default fn, serde field, `Default` impl, snap-to-default on load, both exhaustive test struct literals, and 3 analog tests
- Added the validated `do_update_ui_variant` setter (core) and a `update_ui_variant` Tauri command wrapper, rejecting any value outside the allowlist
- Replaced `tauri.conf.json`'s static `windows` array with an explicit `WebviewWindowBuilder` in `.setup()`, keyed on `ui_variant`, window label fixed at the literal `"main"` (preserves clipboard-manager/dialog capability grants)
- Resolved the Task 2 checkpoint decision ("asset-guard, extended") by implementing both failure-mode recovery paths: a Rust-side `AssetResolver::get()` pre-flight check that falls back to Classic if the bundled New UI asset is missing, and a frontend runtime-failure escape hatch (window `error`/`unhandledrejection` listeners plus a try/catch around the initial render) that shows a vanilla-DOM "Switch to Classic UI" button independent of the New UI's own React tree

## Task Commits

1. **Task 1: New UI entry point boots and reads the real vault end to end** - `ffa3da9` (feat)
2. **Task 3: Desktop window is built at runtime from the ui_variant preference (extended per Task 2 checkpoint decision)** - `31b6096` (feat)

Task 2 was a `checkpoint:decision` (`gate="blocking-human"`) — no commit; the coordinator's resolution ("asset-guard, extended") is implemented in the Task 3 commit above.

**Plan metadata:** commit pending (this SUMMARY + STATE.md/ROADMAP.md)

## Files Created/Modified

- `newui.html` - Third Vite HTML entry, copy of `index.html` with title "Vault" and script src pointed at `newui-main.tsx`; anti-flash `class="dark" data-accent="indigo"` preserved
- `src/newui-main.tsx` - React root mount for the New UI, plus the runtime-failure escape hatch (global error/rejection listeners, try/catch around initial render, vanilla-DOM fallback UI)
- `src/newui/NewApp.tsx` - Auth gate, theme bootstrap, `vault-locked` listener, single `ToastHost`, activity watcher, and a tracer vault view (real `listSites()` call) standing in for `NewShell` until plan 08-03
- `vite.config.ts` - Added `newui` as a third `build.rollupOptions.input` key
- `src/App.css` - Added `@source "../newui.html";` so Tailwind scans the new HTML entry
- `core/src/config.rs` - `ui_variant` field: `VALID_UI_VARIANTS` const, `default_ui_variant()` fn, serde field, `Default` impl entry, snap-to-default in `load()`, both test struct literals, 3 new tests
- `core/src/dto.rs` - `ui_variant` added to `ConfigDto`
- `core/src/commands.rs` - `ui_variant` added to `do_get_config`'s `ConfigDto` literal; new `do_update_ui_variant` setter; 3 new tests
- `src-tauri/src/commands.rs` - New `update_ui_variant` Tauri command wrapper (coordinator-directed, ahead of its 08-02 Settings-UI wiring)
- `src-tauri/src/lib.rs` - Boot-time `ui_variant` read; runtime `WebviewWindowBuilder` construction with the asset-guard match arm; `commands::update_ui_variant` registered in `invoke_handler`
- `src-tauri/tauri.conf.json` - `app.windows` set to `[]`

## Decisions Made

See `key-decisions` in frontmatter. Summary: the Task 2 checkpoint was resolved by the coordinator as "asset-guard, extended" — both the originally-scoped asset-guard AND a new in-app runtime-failure escape hatch, explicitly ruling out keybind-escape. The escape hatch required exposing `update_ui_variant` as a Tauri command a plan-cycle ahead of its planned 08-02 wiring; this was scoped narrowly (command wrapper only, no Settings UI, no web-portal route) per the coordinator's explicit "keep this scoped to Task 3 only" instruction.

## Deviations from Plan

### Auto-fixed Issues

**1. [Checkpoint-directed scope addition] Runtime-failure escape hatch beyond the original Task 3 file list**
- **Found during:** Task 2 checkpoint resolution
- **Issue:** The plan's own Task 2 options (`doc-only`/`asset-guard`/`keybind-escape`) only covered a missing bundled asset. The coordinator's resolution explicitly asked for a second, independent recovery path for a JS error *after* the asset loads (a gap none of the three original options closed), which the original `files_modified` list (`src-tauri/src/commands.rs` was not listed) didn't anticipate.
- **Fix:** Added a minimal `update_ui_variant` Tauri command wrapper in `src-tauri/src/commands.rs` (thin pass-through to the already-planned `do_update_ui_variant`) and a vanilla-DOM error boundary in `src/newui-main.tsx` with a "Switch to Classic UI" button that best-effort persists the choice via that command and always navigates to `/index.html`.
- **Files modified:** `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs` (invoke_handler registration), `src/newui-main.tsx`
- **Verification:** `cargo build -p pwmgr-app` succeeds; `npm run build` compiles the new listeners/fallback UI cleanly
- **Committed in:** `31b6096` (Task 3 commit)

---

**Total deviations:** 1 (coordinator-directed scope addition, not a Rules 1-3 auto-fix — explicitly authorized via the Task 2 checkpoint resolution)
**Impact on plan:** Closes a real gap (asset-guard alone cannot recover from a post-load JS error) that the plan's own checkpoint discussion surfaced. No unrelated scope creep — the addition is narrowly the command wrapper + frontend listener needed for the button to function, nothing from 08-02's Settings-UI/web-portal wiring was pulled forward.

## Issues Encountered

- The prior paused session's `.continue-here.md` assumed `core/src/config.rs`'s `VALID_UI_VARIANTS`/`default_ui_variant()` were already present on disk from earlier uncommitted work. That work is actually in `git stash@{0}` (correctly left untouched per the resume instructions — it belongs to plan 08-02), so `core/src/config.rs` on disk was clean; I wrote the full 6-step field-add pattern fresh rather than assuming a partial baseline. No functional impact — `cargo test -p pwmgr-core` passes with all 308 tests including the 3 new `ui_variant` tests.
- Task 1 is `type="tracer"` with an automated-only `<verify>` block; per the tracer feedback gate (interactive, `end-of-phase` mode, no `<human-check>` present), re-ran all 5 automated checks — all passed — and continued directly to Task 2 with no checkpoint synthesized.
- Task 3's `<verify>` carries a `<human-check>` (launch `npm run tauri dev`, confirm Classic/New/invalid-value window behavior and clipboard/file-dialog access). Per `workflow.human_verify_mode` defaulting to `end-of-phase` (no `.planning/config.json` override present in this project), this was NOT executed as a mid-flight interactive checkpoint — it is deferred for the end-of-phase UAT harvest, consistent with D4/D5's `coverage` entries above.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Plan 08-01 is complete: the third Vite entry point exists and is styled, `ui_variant` persists and validates, the desktop window builds at runtime from it, and both recovery paths (missing asset, post-load JS error) are in place with no manual-file-edit requirement.
- Ready for wave 2: plans 08-02 (frontend selector toggle / Settings UI wiring, including the full `update_ui_variant` web-portal route this plan intentionally left out) and 08-03 (three-pane vault shell replacing this plan's tracer view), both depending on 08-01.
- Deferred to end-of-phase UAT: the Task 3 human-check (Classic/New/invalid-value desktop launch behavior, clipboard/file-dialog regression) and confirming the runtime-failure escape hatch's fallback UI and button actually work when triggered live.
- Git stash `stash@{0}` (pre-existing uncommitted `ui_variant` scaffolding) remains untouched and available for whoever executes 08-02, per the resume instructions.

---
*Phase: 08-new-vault-ui-figma-port*
*Completed: 2026-09-14*

## Self-Check: PASSED

All 11 key files verified present on disk (`[ -f ]`), and both task commits (`ffa3da9`, `31b6096`) verified present in `git log --oneline --all`.
