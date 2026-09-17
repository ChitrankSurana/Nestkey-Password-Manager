---
phase: 08-new-vault-ui-figma-port
plan: 12
subsystem: auth
tags: [reprompt, master-password, vault-lock, settings, react, vitest]

requires:
  - phase: 08-new-vault-ui-figma-port
    provides: repromptMode/repromptGraceSeconds config plumbing (plan 08-11)
provides:
  - Global/Per-credential reprompt gate as a pure, unit-tested decision function
  - Session-scoped grace-window store wired through SecretField and both app shells
  - Settings controls for Reprompt mode and grace window, effective without restart
affects: [SecretField, Settings, App shells (Classic + New UI)]

actuals:
  tokens: 9200
  tasks: 3
  commits: 3

tech-stack:
  added: []
  patterns:
    - "Pure decision function (repromptPolicy.ts) with no Date.now()/module state, wrapped by a thin stateful shell (reprompt.ts) — mirrors the split already used elsewhere for testable time-based logic"

key-files:
  created:
    - src/lib/repromptPolicy.ts
    - src/lib/repromptPolicy.test.ts
    - src/lib/reprompt.ts
  modified:
    - src/components/SecretField.tsx
    - src/screens/Settings.tsx
    - src/App.tsx
    - src/newui/NewApp.tsx

key-decisions:
  - "Grace window applies only in Global mode; Per-credential mode never reads lastSuccessAt, keeping the two modes on separate code paths per the plan's backward-compatibility contract"
  - "markRepromptSuccess() is called only after api.verifyMasterPassword returns true, and only after the existing wrong-password early return — a failed attempt can never open the window"
  - "Config-not-yet-loaded fails open to the Per-credential default rather than prompting on every field before load — treated as a convenience gate over an already-unlocked vault, not a security boundary (per SECURITY_CONCERNS.md)"

patterns-established:
  - "Session-scoped, in-memory-only state modules (reprompt.ts) with an explicit head-comment that a reload resetting the state is correct, not a bug"

requirements-completed: [AUTH-01]

coverage:
  - id: D1
    description: "repromptRequired() pure gate decision covering both modes, both flag values, and exact-boundary grace-window expiry"
    requirement: AUTH-01
    verification:
      - kind: unit
        ref: "src/lib/repromptPolicy.test.ts (11 tests, full truth table)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Session-scoped grace store wired through SecretField.requestAction/confirmReauth, with vault-locked clearing the window in both app shells"
    requirement: AUTH-01
    verification:
      - kind: unit
        ref: "npm test (40 tests, full suite)"
        status: pass
      - kind: unit
        ref: "src/newui/lib/parity.test.ts (7 tests) — New UI invariant gate"
        status: pass
      - kind: other
        ref: "grep -c markRepromptSuccess/isRepromptRequired src/components/SecretField.tsx; grep -c clearRepromptGrace src/App.tsx src/newui/NewApp.tsx"
        status: pass
    human_judgment: true
    rationale: "Live cross-credential grace-window behavior and auto-lock timing are end-to-end UX properties no unit test asserts — plan's own <verification> defers this to the next UAT pass"
  - id: D3
    description: "Settings screen exposes Reprompt mode and grace-window selects in the Vault section, both taking effect without restart, New UI inheriting via SettingsPane passthrough"
    requirement: AUTH-01
    verification:
      - kind: other
        ref: "grep -c REPROMPT_MODE_OPTIONS/REPROMPT_GRACE_OPTIONS/refreshRepromptConfig src/screens/Settings.tsx"
        status: pass
    human_judgment: true
    rationale: "Visual placement, tooltip copy, and disabled-state UX for the grace select are judgment calls the plan explicitly defers to manual confirmation"

duration: 3min
completed: 2026-09-16
status: complete
---

# Phase 08 Plan 12: Global/Per-credential Reprompt Mode Summary

**Vault-wide master-password reprompt gate with a configurable grace window, built as a pure decision function plus a session-scoped store wired through SecretField, both app shells, and Settings.**

## Performance

- **Duration:** 3 min
- **Started:** 2026-09-16T23:30:30+05:30
- **Completed:** 2026-09-16T23:32:55+05:30
- **Tasks:** 3
- **Files modified:** 7 (3 created, 4 modified)

## Accomplishments
- `repromptPolicy.ts` — pure `repromptRequired()` gate, fully unit-tested against the truth table (both modes, both flag values, exact-boundary grace expiry, backward-negative-clock handling)
- `reprompt.ts` — memoized config load, `isRepromptRequired`/`markRepromptSuccess`/`clearRepromptGrace` session state, wired into `SecretField` and both `vault-locked` listeners
- `Settings.tsx` — Reprompt mode and grace-window selects in the Vault section, taking effect immediately via `refreshRepromptConfig()`, inherited by the New UI through `SettingsPane`'s passthrough

## Task Commits

Each task was committed atomically:

1. **Task 1: Write the gate decision as a pure, fully unit-tested function** - `dfd8761` (feat)
2. **Task 2: Add the session-scoped grace store and rewire SecretField through it** - `cc4f0b0` (feat)
3. **Task 3: Add the Reprompt mode and grace-window controls to Settings** - `cb2fac3` (feat)

**Plan metadata:** this commit (docs: complete plan)

## Files Created/Modified
- `src/lib/repromptPolicy.ts` - pure gate decision, no Date.now()/module state/api import
- `src/lib/repromptPolicy.test.ts` - full truth table, 11 tests
- `src/lib/reprompt.ts` - stateful shell: memoized config load, grace-window store
- `src/components/SecretField.tsx` - `requestAction`/`confirmReauth` rewired through the shared policy
- `src/screens/Settings.tsx` - Reprompt mode + grace-window selects, change handlers
- `src/App.tsx` - `clearRepromptGrace()` added to the existing `vault-locked` listener
- `src/newui/NewApp.tsx` - same wiring on the New UI side

## Decisions Made
- Grace window scoped to Global mode only, on a separate code path from Per-credential — prevents any leakage between modes
- `markRepromptSuccess()` placed strictly after the `ok` check in `confirmReauth`, after the existing wrong-password early return
- Pre-config-load default fails open to Per-credential (today's behavior) rather than over-prompting

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

**Resume-gate anomaly (closed out, not re-executed):** The three task commits (`dfd8761`, `cc4f0b0`, `cb2fac3`) existed on the branch but this SUMMARY.md was never written or committed in the original run, leaving an illegal partial-plan state per `execute-phase`'s `safe_resume_gate`. Verified rather than re-executed: re-ran every plan `<verify>` command (`npx vitest run src/lib/repromptPolicy.test.ts`, `npm test`, `npm run build`, `npx vitest run src/newui/lib/parity.test.ts`, all task-level greps) — all passed — and manually inspected `reprompt.ts`, `repromptPolicy.ts`, `SecretField.tsx`, `Settings.tsx`, `App.tsx`, and `src/newui/NewApp.tsx` against every `must_haves.truths`/`key_links` entry in the plan frontmatter. All matched. This SUMMARY closes the plan out; no code changes were made in this pass.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- G-08-2 closed. Phase 08 plans 01–12 all summarized; ready for phase-level verification (`/gsd-verify-work 08`).
- Manual UAT still open per the plan's own `<verification>` note: live cross-credential grace-window timing and auto-lock behavior deferred to the next UAT pass.

---
*Phase: 08-new-vault-ui-figma-port*
*Completed: 2026-09-16*
