---
phase: 08-new-vault-ui-figma-port
plan: 10
subsystem: vault-core-and-ui
tags: [rust, tauri, dialog-plugin, react, http-integration-test]

# Dependency graph
requires:
  - phase: 08-new-vault-ui-figma-port (plans 01-09)
    provides: the existing @tauri-apps/plugin-dialog dependency proven for file pickers
      (CsvExport.tsx's save() pattern), Backup.tsx's existing restore-dialog structure,
      do_restore_backup's existing caller-supplied-path precedent at the core layer
provides:
  - "core::commands::do_create_backup(state, paths, destination_path: Option<&str>) - None keeps
    the historical backups_dir()/timestamp default; Some(p) writes to p verbatim; Some(\"\") is
    rejected with invalid_input"
  - "src-tauri create_backup Tauri command accepting an optional destination_path, forwarded from
    a native save() dialog the user drove"
  - "web-src-tauri create_backup handler proven (grep gate + HTTP integration test) to always pass
    None regardless of any client-supplied destinationPath in the request body"
  - "Backup.tsx's Create Backup button drives a native save dialog on desktop; web mode unchanged"
affects: [08-11, 08-12, Phase 9 SEC-01 (do NOT extend web create_backup with a path parameter -
  this plan's T-08-10-01 mitigation depends on that staying true)]

# Actuals (#2632)
actuals:
  tokens: 3422
  tasks: 3
  commits: 3
plan_head_before: 513b5bd8d7b7ac4f5f53ef45c9f1bc6db4e81ddf

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Optional destination-path threading mirrors do_restore_backup's existing precedent: the
      core function takes Option<&str>, None means the app-owned default directory, Some is only
      ever reached from the desktop transport where the path came from a native OS dialog the
      user personally drove - the web transport is a structurally separate call site that always
      passes None, so there's no runtime toggle to accidentally flip."
    - "Backup.tsx's handleCreate now mirrors CsvExport.tsx's proven save()-dialog shape exactly:
      call save() before setting busy, treat a falsy return as a clean cancel (no busy, no error,
      no status change), then proceed with the chosen (or null) path."

key-files:
  created: []
  modified:
    - core/src/commands.rs
    - src-tauri/src/commands.rs
    - web-src-tauri/src/handlers.rs
    - web-src-tauri/tests/http_integration.rs
    - src/lib/api.ts
    - src/screens/Backup.tsx
    - .planning/phases/08-new-vault-ui-figma-port/08-01-PLAN.md

key-decisions:
  - "Task 1 (tracer, tdd=true) executed as production-quality code exactly like a type=\"auto\"
    task, per plan convention - no throwaway scaffolding was written and none was discarded."
  - "Tracer feedback gate after Task 1: re-ran cargo test -p pwmgr-core create_backup and the full
    cargo test -p pwmgr-core suite (320 passed) before expanding into Task 2, per
    workflow.human_verify_mode=end-of-phase and Task 1's automated-only <verify> - no live
    checkpoint stop occurred, consistent with the tracer feedback gate's row-3 behavior."
  - "web-src-tauri's create_backup handler stays request-body-free (no destinationPath field, no
    RequestStruct) rather than adding a field that's merely ignored - a present-but-ignored field
    would misleadingly suggest the web route accepts a destination, undermining the very
    regression gate (T-08-10-01) this plan exists to add."
  - "08-01-PLAN.md's stale human-check claim was corrected via a scoped Edit on the single sentence
    only, with a bracketed provenance note, per the plan's explicit instruction to change nothing
    else in a completed plan's historical record."

requirements-completed: [DATA-01]

coverage:
  - id: D1
    description: "do_create_backup honours an explicit destination, defaults to the backups
      directory when given None, rejects an empty destination with invalid_input, and keeps
      failing closed (locked error) when the vault is locked, regardless of the destination
      argument"
    requirement: DATA-01
    verification:
      - kind: unit
        ref: "cargo test -p pwmgr-core create_backup (5 passed)"
        status: pass
      - kind: unit
        ref: "cargo test -p pwmgr-core (320 passed)"
        status: pass
    human_judgment: false
  - id: D2
    description: "The desktop create_backup command accepts an optional destination_path and
      forwards it; the web create_backup handler unconditionally passes None and provably ignores
      any client-supplied destinationPath in the request body, with no file ever created at the
      attacker-chosen location"
    requirement: DATA-01
    verification:
      - kind: other
        ref: "cargo build -p pwmgr-app (exit 0)"
        status: pass
      - kind: integration
        ref: "cargo test -p pwmgr-web create_backup_route_ignores_a_client_supplied_destination (pass)"
        status: pass
      - kind: unit
        ref: "cargo test --workspace (320 core + 2 e2e + 17 web + 6 http-integration, all passed)"
        status: pass
      - kind: other
        ref: "grep -c 'do_create_backup(&s.app_state, &ctx.paths, None)' web-src-tauri/src/handlers.rs (1)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Backup.tsx's Create Backup button drives a native save() dialog on desktop
      mirroring CsvExport.tsx; cancelling is a clean no-op; web mode calls createBackup() with no
      argument unchanged; api.ts's createBackup default keeps every existing call site valid;
      08-01-PLAN.md no longer claims a backup-create native dialog was pre-existing Classic
      behavior"
    requirement: DATA-01
    verification:
      - kind: other
        ref: "npm run build (exit 0)"
        status: pass
      - kind: unit
        ref: "npm test (6 files, 29 passed)"
        status: pass
      - kind: other
        ref: "grep -c 'from \"@tauri-apps/plugin-dialog\"' src/screens/Backup.tsx (1)"
        status: pass
      - kind: other
        ref: "grep -c 'destinationPath' src/lib/api.ts (2)"
        status: pass
      - kind: other
        ref: "! grep -q 'Data → Backup still opens a native file' 08-01-PLAN.md (0 matches, exit 0)"
        status: pass
    human_judgment: false
  - id: D4
    description: "Manual end-to-end confirmation: click Create Backup on the desktop build,
      confirm a native save dialog appears with a .bak default; cancel and confirm no file was
      written and no error shown; repeat, choose a folder outside ~/.pwmgr, confirm the file lands
      there and the status message names that exact path; restore from it to prove the bundle is
      valid"
    verification: []
    human_judgment: true
    rationale: "Plan's own <verification> section explicitly defers this to the next UAT pass
      (desktop build) per workflow.human_verify_mode=end-of-phase, consistent with every prior
      plan in this phase (08-01 through 08-09)."

duration: ~15min
completed: 2026-09-16
status: complete
commits: 3
---

# Phase 8 Plan 10: Native Save-Location Picker for Create Backup Summary

**Create Backup now drives a native OS save dialog on desktop (destination threaded through `do_create_backup` -> the Tauri `create_backup` command -> `Backup.tsx`'s `save()` call, mirroring the existing `CsvExport.tsx` pattern) while the web transport is proven — by grep gate and a new HTTP integration test — to still write only to its own `backups_dir()`, closing gap G-08-1 without growing `SECURITY_CONCERNS.md` H1's arbitrary-file-write surface.**

## Performance

- **Duration:** ~15 min
- **Tasks:** 3 (1 `type="tracer" tdd="true"`, 1 `type="auto" tdd="true"`, 1 `type="auto"`)
- **Files modified:** 7
- **Commits:** 3

## Accomplishments

- **Task 1 (tracer, TDD):** `core::commands::do_create_backup` gained a `destination_path:
  Option<&str>` parameter. `None` keeps the exact historical default (millisecond-timestamped
  filename under `paths.backups_dir()`), `Some(p)` with a non-empty `p` writes there verbatim and
  returns it, `Some("")` is rejected with `invalid_input` before any write, and the locked check
  still fires first regardless of the destination argument. Five tests (2 updated existing, 3 new:
  explicit destination, empty rejection, locked+destination) all pass; the tracer feedback gate
  re-ran both the targeted and full `pwmgr-core` suites (320 tests) before expanding.
- **Task 2 (TDD):** `src-tauri/src/commands.rs`'s `create_backup` Tauri command now takes
  `destination_path: Option<String>` and forwards `.as_deref()`. `web-src-tauri/src/handlers.rs`'s
  `create_backup` handler stayed request-body-free and now passes `None` explicitly, with a
  comment recording why the asymmetry is deliberate (desktop path comes from a user-driven native
  dialog; web mode would be an arbitrary client string; `SECURITY_CONCERNS.md` H1 already flags
  four other handlers with this exact problem, Phase 9 SEC-01 is chartered to close them - this
  plan must not add a fifth). A new HTTP integration test,
  `create_backup_route_ignores_a_client_supplied_destination`, POSTs a `destinationPath` pointing
  outside the app directory and asserts the response succeeds, the returned path is inside the
  server's own `backups_dir()`, and no file was created at the attacker-chosen location.
- **Task 3:** `src/lib/api.ts`'s `createBackup` now takes an optional `destinationPath` (default
  `null`), keeping the existing (only) call site and web mode's `null` send valid without any
  change at that call site. `src/screens/Backup.tsx`'s `handleCreate` now calls `save({
  defaultPath: "vault-backup.bak", filters: [...] })` before setting busy when `isTauri` is true -
  a falsy return (cancel) exits immediately with no busy state, no error, and no status change,
  matching `CsvExport.tsx`'s proven shape exactly. The button's tooltip no longer promises a fixed
  location. `08-01-PLAN.md`'s stale `<human-check>` claim (that Data → Backup "still opens a
  native file dialog") was corrected via a scoped edit to the truthful original state, with a
  bracketed provenance note naming this plan as the correction's origin.

## Task Commits

1. **Task 1: Give core do_create_backup an optional destination path** - `955ff7f` (feat, TDD)
2. **Task 2: Thread the destination through the desktop transport only, not the web route** - `80909d9` (feat, TDD)
3. **Task 3: Drive a native save dialog from the Backup screen, and correct the stale plan claim** - `fb5d425` (feat)

**Plan metadata:** this file's commit (docs: complete plan)

_Note: TDD tasks landed as single atomic commits each (test + implementation together), matching
this plan's `<behavior>`/`<action>` structure rather than separate RED/GREEN commits._

## Files Created/Modified

- `core/src/commands.rs` - `do_create_backup` gains `destination_path: Option<&str>`; 5 tests total (2 updated, 3 new)
- `src-tauri/src/commands.rs` - `create_backup` Tauri command takes and forwards `destination_path: Option<String>`
- `web-src-tauri/src/handlers.rs` - `create_backup` handler passes `None` unconditionally, with a deliberate-asymmetry comment
- `web-src-tauri/tests/http_integration.rs` - new `create_backup_route_ignores_a_client_supplied_destination` regression test
- `src/lib/api.ts` - `createBackup(destinationPath = null)` sends `destinationPath`
- `src/screens/Backup.tsx` - `handleCreate` drives a `save()` dialog on desktop; tooltip updated
- `.planning/phases/08-new-vault-ui-figma-port/08-01-PLAN.md` - scoped correction of the stale native-dialog claim

## Decisions Made

See `key-decisions` in frontmatter. Summary:
1. Task 1 (tracer) was executed as real, production-quality code from the start - nothing
   throwaway was written or later discarded.
2. The tracer feedback gate re-ran Task 1's automated `<verify>` end-to-end before expanding into
   Task 2, per `workflow.human_verify_mode=end-of-phase`; no live checkpoint stop occurred since
   verification passed cleanly.
3. The web `create_backup` handler deliberately stays request-body-free rather than adding an
   ignored `destinationPath` field - an ignored-but-present field would misleadingly suggest the
   route accepts a destination, undermining the regression gate this plan adds.
4. The stale `08-01-PLAN.md` claim was corrected with a minimal scoped edit plus a bracketed
   provenance note, per the plan's explicit "change nothing else" instruction for a completed
   plan's historical record.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None. All automated verification passed on the first attempt: `cargo test -p pwmgr-core
create_backup` (5/5), `cargo test -p pwmgr-core` (320/320), `cargo build -p pwmgr-app` (exit 0),
`cargo test -p pwmgr-web` (17 unit + 6 integration, all passed), `cargo test --workspace` (345
total tests passed), `npm run build` (exit 0), `npm test` (29/29), and all four grep-based
verification gates.

## User Setup Required

None - no external service configuration required. `@tauri-apps/plugin-dialog`'s `save()` was
already a direct dependency and already permitted by `dialog:allow-save` in
`src-tauri/capabilities/default.json`; no capability change was needed.

## Next Phase Readiness

- G-08-1 is closed pending the deferred end-of-phase UAT walkthrough (D4 above): the desktop
  Create Backup flow drives a real native save dialog, cancel is a clean no-op, and the returned
  path is authoritative for the success message.
- Web mode's file-write surface is unchanged - `create_backup` still accepts no client-supplied
  path, provably so via both a grep gate and an HTTP integration test - so Phase 9 SEC-01's scope
  does not grow.
- Ready for 08-11 (G-08-2 backend: `repromptMode` / `repromptGraceSeconds` config, setters, both
  transports).

---
*Phase: 08-new-vault-ui-figma-port*
*Completed: 2026-09-16*

## Self-Check: PASSED

All 8 key files verified present on disk (`[ -f ]`), and all 4 commits (`955ff7f`, `80909d9`,
`fb5d425`, `826ae4b`) verified present in `git log --oneline --all`.
