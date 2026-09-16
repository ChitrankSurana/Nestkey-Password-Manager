---
phase: 08-new-vault-ui-figma-port
plan: 11
subsystem: auth
tags: [rust, serde, tauri, axum, config, settings, typescript]

# Dependency graph
requires:
  - phase: 08-02
    provides: "config setter wiring pattern (core -> Tauri command -> web route -> api.ts), the exact shape this plan copies for ui_variant's reprompt-mode sibling"
provides:
  - "Config.reprompt_mode / Config.reprompt_grace_seconds fields, defaulted to perCredential/30, snapped to their defaults on an out-of-allowlist load"
  - "do_update_reprompt_mode / do_update_reprompt_grace_seconds core setters, validated against VALID_REPROMPT_MODES / VALID_REPROMPT_GRACE_SECONDS, save-then-commit ordered"
  - "update_reprompt_mode / update_reprompt_grace_seconds reachable over Tauri IPC (invoke_handler) and over /api/update_reprompt_mode, /api/update_reprompt_grace_seconds (session+CSRF protected)"
  - "src/lib/api.ts: RepromptMode union type, ConfigDto.repromptMode/repromptGraceSeconds, api.updateRepromptMode/api.updateRepromptGraceSeconds wrappers"
affects: [08-12]

# Actuals (#2632)
actuals:
  tokens: 5054
  tasks: 3
  commits: 3

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Config field-add pattern extended a seventh time (const allowlist -> default_* fn -> struct field with #[serde(default = ...)] -> Default impl entry -> Config::load snap-to-default block -> setter with allowlist-check-then-save-then-commit) - reprompt_mode/reprompt_grace_seconds are now the ui_variant-shaped sibling this plan explicitly modeled them on"
    - "Reprompt grace window is a fixed u32 allowlist ([15, 30, 60, 300]), not a free-range integer - same rationale as VALID_AUTO_LOCK_MINUTES: an unbounded value would let a user silently disable the gate"

key-files:
  created: []
  modified:
    - core/src/config.rs
    - core/src/dto.rs
    - core/src/commands.rs
    - src-tauri/src/commands.rs
    - src-tauri/src/lib.rs
    - web-src-tauri/src/handlers.rs
    - web-src-tauri/tests/http_integration.rs
    - src/lib/api.ts

key-decisions:
  - "Task 1 (tracer, tdd=true) was executed as real, production-quality code from the start, per this phase's established convention (see 08-10-SUMMARY.md precedent) - RED/GREEN were done in-session (tests written, confirmed to compile-fail before the implementation existed, then made to pass) but committed as a single feat commit per task rather than as separate RED/GREEN commits, matching this project's existing task-commit granularity."
  - "Tracer feedback gate after Task 1: re-ran `cargo test -p pwmgr-core reprompt` and the full `cargo test -p pwmgr-core` suite (329 tests) before expanding into Task 2. Both passed on the first attempt - no live checkpoint stop, per workflow.human_verify_mode=end-of-phase and Task 1's automated-only <verify>."
  - "The plan's Task 2 read_first pointed to an existing config-getter test pattern in web-src-tauri/tests/http_integration.rs to mirror - no such test exists yet in that file (only auth/session/backup-destination tests do). The three new tests were written following that file's established request/assert conventions (spawn_server, cookie-jar client, CSRF header, JSON body) rather than literally mirroring a nonexistent config test."
  - "RepromptModeRequest/RepromptGraceSecondsRequest both got #[serde(rename_all = \"camelCase\")] per the plan's explicit instruction, even though both field names (mode, seconds) are single words where the attribute has no observable effect - kept for consistency with every sibling request struct in the file."

requirements-completed: [AUTH-01]

coverage:
  - id: D1
    description: "Config.reprompt_mode and Config.reprompt_grace_seconds persist through config.json, default to perCredential/30 on a config file that predates these fields, and snap any out-of-allowlist stored value back to its default on load"
    requirement: AUTH-01
    verification:
      - kind: unit
        ref: "cargo test -p pwmgr-core reprompt (9 tests, all pass)"
        status: pass
      - kind: unit
        ref: "cargo test -p pwmgr-core (329 tests total, all pass)"
        status: pass
    human_judgment: false
  - id: D2
    description: "do_update_reprompt_mode and do_update_reprompt_grace_seconds validate against their allowlists, persist only on a successful save, and never desync in-memory state from disk on a failed save"
    requirement: AUTH-01
    verification:
      - kind: unit
        ref: "core/src/commands.rs::tests::update_reprompt_mode_persists_and_rejects_invalid_value, update_reprompt_grace_seconds_persists_and_rejects_invalid_value, get_config_includes_reprompt_defaults, update_reprompt_mode_does_not_mutate_state_when_save_fails"
        status: pass
    human_judgment: false
  - id: D3
    description: "Both setters are reachable over Tauri IPC (registered in invoke_handler) and over the localhost HTTP portal (session+CSRF-protected POST routes), and the web routes reject an unknown mode"
    requirement: AUTH-01
    verification:
      - kind: integration
        ref: "web-src-tauri/tests/http_integration.rs::update_reprompt_mode_route_persists_and_is_visible_in_get_config, update_reprompt_mode_route_rejects_an_unknown_mode, update_reprompt_grace_seconds_route_persists (3 tests, all pass)"
        status: pass
      - kind: other
        ref: "grep -c 'commands::update_reprompt_mode' src-tauri/src/lib.rs (1), grep -c 'commands::update_reprompt_grace_seconds' src-tauri/src/lib.rs (1)"
        status: pass
      - kind: other
        ref: "grep -c '/api/update_reprompt_mode' web-src-tauri/src/handlers.rs (1), grep -c '/api/update_reprompt_grace_seconds' web-src-tauri/src/handlers.rs (1)"
        status: pass
      - kind: unit
        ref: "cargo test --workspace (all crates, all pass)"
        status: pass
    human_judgment: false
  - id: D4
    description: "src/lib/api.ts exposes RepromptMode, ConfigDto.repromptMode/repromptGraceSeconds, and api.updateRepromptMode/api.updateRepromptGraceSeconds wrappers, reachable over both transports via the existing call<T>() dispatch"
    requirement: AUTH-01
    verification:
      - kind: other
        ref: "npm run build (tsc + vite build), exit 0"
        status: pass
      - kind: unit
        ref: "npm test (29 tests across 6 files, all pass)"
        status: pass
      - kind: other
        ref: "grep -c 'repromptMode: RepromptMode;' src/lib/api.ts (1), grep -c 'updateRepromptMode' src/lib/api.ts (1), grep -c 'updateRepromptGraceSeconds' src/lib/api.ts (1)"
        status: pass
    human_judgment: false

duration: ~15min
completed: 2026-09-16
status: complete
commits: 3
plan_head_before: 39b45d48c5cc03ae6b7a266e1b7e1c9542ef39c2
---

# Phase 8 Plan 11: Reprompt Mode/Grace Config Backend Summary

**Backend half of G-08-2: `repromptMode` ("perCredential" | "global") and `repromptGraceSeconds` (15/30/60/300) fully wired from `config.json` through validated core setters, both transports (Tauri IPC + localhost HTTP), and `src/lib/api.ts`, defaulting to today's exact per-credential reprompt behavior.**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-09-16T23:10:00+05:30 (approximate)
- **Completed:** 2026-09-16T23:25:17+05:30
- **Tasks:** 3 (1 `type="tracer" tdd="true"`, 1 `type="auto" tdd="true"`, 1 `type="auto"`)
- **Files modified:** 8

## Accomplishments

- Added `VALID_REPROMPT_MODES` (`["perCredential", "global"]`) and `VALID_REPROMPT_GRACE_SECONDS` (`[15, 30, 60, 300]`) allowlist constants, `default_reprompt_mode()`/`default_reprompt_grace_seconds()`, both fields on `Config` with per-field serde defaults, both added to the hand-written `Default` impl, and `Config::load`'s snapping block extended so a hand-edited or corrupted `config.json` with an out-of-allowlist value for either field falls back to its default rather than propagating a nonsense/disabling value.
- Added `reprompt_mode`/`reprompt_grace_seconds` to `ConfigDto` and mapped both in `do_get_config`.
- Added `do_update_reprompt_mode` and `do_update_reprompt_grace_seconds`, each copying `do_update_ui_variant`'s exact shape: validate against the allowlist and return `invalid_input_error` on a miss, then clone-mutate-save-then-commit so a failed disk write can never desync in-memory state from disk.
- Registered `update_reprompt_mode`/`update_reprompt_grace_seconds` as `#[tauri::command]` wrappers in `src-tauri/src/commands.rs` and in the `invoke_handler` list in `src-tauri/src/lib.rs`.
- Added `RepromptModeRequest`/`RepromptGraceSecondsRequest`, two axum handlers, and `/api/update_reprompt_mode` / `/api/update_reprompt_grace_seconds` routes in `web-src-tauri/src/handlers.rs`, in the same session-and-CSRF-protected router chain as every sibling setter route.
- Added `RepromptMode` union type, `ConfigDto.repromptMode`/`repromptGraceSeconds`, and `api.updateRepromptMode`/`api.updateRepromptGraceSeconds` to `src/lib/api.ts`. `src/screens/Settings.tsx` is deliberately untouched — the gate logic and Settings controls are plan 08-12's work.
- Added 9 new Rust unit tests (`core/src/config.rs` + `core/src/commands.rs`) and 3 new HTTP integration tests (`web-src-tauri/tests/http_integration.rs`), all passing alongside the full pre-existing suite (`cargo test --workspace`: 329 core + 2 e2e + 17 web-unit + 9 http-integration, all pass; `npm run build` and `npm test`: 29 tests, all pass).

## Task Commits

1. **Task 1: Persist and validate the two new config fields in core** - `a767158` (feat, tracer/TDD)
2. **Task 2: Expose both setters on the Tauri IPC surface and the HTTP portal** - `47ff60d` (feat, TDD)
3. **Task 3: Extend the frontend API surface with the new config fields and setters** - `ec3166a` (feat)

**Plan metadata:** commit pending (this SUMMARY + STATE.md/ROADMAP.md/REQUIREMENTS.md)

## Files Created/Modified

- `core/src/config.rs` - `VALID_REPROMPT_MODES`, `VALID_REPROMPT_GRACE_SECONDS`, `default_reprompt_mode()`, `default_reprompt_grace_seconds()`, both `Config` fields, `Default` impl entries, `Config::load` snapping, 6 new tests
- `core/src/dto.rs` - `ConfigDto.reprompt_mode`/`reprompt_grace_seconds`
- `core/src/commands.rs` - `do_get_config` mapping, `do_update_reprompt_mode`, `do_update_reprompt_grace_seconds`, 4 new tests
- `src-tauri/src/commands.rs` - import list extension, `update_reprompt_mode`/`update_reprompt_grace_seconds` Tauri command wrappers
- `src-tauri/src/lib.rs` - `invoke_handler` registration for both commands
- `web-src-tauri/src/handlers.rs` - `RepromptModeRequest`/`RepromptGraceSecondsRequest`, two async handlers, two route registrations
- `web-src-tauri/tests/http_integration.rs` - 3 new integration tests
- `src/lib/api.ts` - `RepromptMode` type, `ConfigDto` fields, two setter wrappers

## Decisions Made

See `key-decisions` in frontmatter. Summary:
1. Task 1's tracer/TDD work landed as a single production-quality commit (test+implementation together), consistent with this phase's established convention from 08-10.
2. The tracer feedback gate re-ran `cargo test -p pwmgr-core` end-to-end before expanding into Task 2; both the targeted `reprompt` filter and the full suite passed, so no checkpoint stop occurred.
3. Task 2's HTTP integration tests were written fresh (no existing config-getter test to mirror in that file, despite the plan's read_first pointing there) following the file's own established request/assert conventions.
4. `RepromptModeRequest`/`RepromptGraceSecondsRequest` both carry `#[serde(rename_all = "camelCase")]` for consistency with every sibling request struct, even though it has no effect on these particular single-word field names.

## Deviations from Plan

None — plan executed exactly as written. The one adjustment (Task 2's HTTP integration tests being written fresh rather than mirroring a nonexistent existing config test) is not a Rules 1-3 auto-fix or a Rule 4 architectural question; it's the plan's own read_first pointer not matching what's currently on disk in that file, resolved by following the file's actual established conventions instead.

## Issues Encountered

None. `cargo test -p pwmgr-core`, `cargo test -p pwmgr-web`, `cargo test --workspace`, `cargo build -p pwmgr-app`, `npm run build`, and `npm test` all passed clean on the first attempt for every task.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Backend half of G-08-2 is complete: `repromptMode`/`repromptGraceSeconds` are persisted, validated, defaulted to today's exact behavior, and reachable from the frontend over both transports (`api.getConfig()`, `api.updateRepromptMode()`, `api.updateRepromptGraceSeconds()`).
- Zero observable behavior change for any existing install — the gate itself and the Settings controls are plan 08-12's scope.
- Plan 08-12 (G-08-2 frontend) can proceed: it consumes `RepromptMode`, `ConfigDto.repromptMode`/`repromptGraceSeconds`, and both setter wrappers to build the Global/Per-credential gate with a grace window in `SecretField.tsx`, plus the Settings controls.

---
*Phase: 08-new-vault-ui-figma-port*
*Completed: 2026-09-16*

## Self-Check: PASSED

All 9 key files verified present on disk (`[ -f ]`), and all 3 task commits (`a767158`, `47ff60d`, `ec3166a`) verified present in `git log --oneline --all`.
