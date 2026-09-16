---
phase: 08-new-vault-ui-figma-port
plan: 08
subsystem: ui
tags: [react, tailwind, vitest, accessibility, loading-state]

# Dependency graph
requires:
  - phase: 08-new-vault-ui-figma-port (plans 01-07)
    provides: Login/Register/SecretField screens and the Icon choke point this plan wires a spinner into
provides:
  - Reusable Spinner component (Icon + Tailwind animate-spin) as the app's single loading affordance
  - Moving loading animation on Login submit, Login Windows Hello unlock, Register submit, and
    SecretField reprompt-confirm buttons during their multi-second Argon2id derivations
  - Source-invariant test (Spinner.test.ts) gating all four wiring points against regression
affects: [08-09, 08-10, 08-11, 08-12, any future UAT pass re-measuring Argon2 cost]

# Actuals (#2632)
actuals:
  tokens: 1714
  tasks: 2
  commits: 3
plan_head_before: 33c0213

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Single loading affordance: any operation with a perceptible wait renders <Spinner />, composed from the existing Icon choke point + Tailwind's animate-spin utility, never an inline <i class=\"bi bi-...\">"

key-files:
  created:
    - src/components/Spinner.tsx
    - src/components/Spinner.test.ts
  modified:
    - src/screens/Login.tsx
    - src/screens/Register.tsx
    - src/components/SecretField.tsx

key-decisions:
  - "Spinner routes the bootstrap-icons glyph through the existing Icon component rather than emitting a raw <i class=\"bi bi-...\"> at each call site, preserving the project's single-choke-point rule for the icon font"
  - "No Argon2/KDF parameter touched - per the UAT's locked deferral, this plan only adds visual feedback"

patterns-established:
  - "Spinner.tsx: reusable role=\"status\" aria-label=\"Loading\" wrapper around Icon name=\"arrow-repeat\" className=\"animate-spin\" - the one loading affordance for any future slow operation"

requirements-completed: [UX-01]

coverage:
  - id: D1
    description: "Login submit button renders a moving spinner + \"Logging in…\" while submitting is true"
    requirement: "UX-01"
    verification:
      - kind: unit
        ref: "src/components/Spinner.test.ts#Login.tsx renders <Spinner at least twice (submit + Windows Hello)"
        status: pass
    human_judgment: true
    rationale: "Source-invariant test proves the JSX wiring exists, but whether the animation is visibly perceptible/smooth for the full ~4s wait is a visual judgment the UAT explicitly deferred to the next UAT pass (see plan <verification>)."
  - id: D2
    description: "Login Windows Hello button renders a moving spinner + \"Unlocking…\" while submitting is true"
    requirement: "UX-01"
    verification:
      - kind: unit
        ref: "src/components/Spinner.test.ts#Login.tsx renders <Spinner at least twice (submit + Windows Hello)"
        status: pass
    human_judgment: true
    rationale: "Same as D1 - wiring proven by source-invariant test, visual perceptibility deferred to next UAT pass."
  - id: D3
    description: "Register submit button renders a moving spinner + \"Creating your vault…\" while submitting is true"
    requirement: "UX-01"
    verification:
      - kind: unit
        ref: "src/components/Spinner.test.ts#Register.tsx renders <Spinner at least once"
        status: pass
    human_judgment: true
    rationale: "Wiring proven by source-invariant test; visual confirmation deferred to next UAT pass per plan <verification>."
  - id: D4
    description: "SecretField reprompt Confirm button renders a moving spinner + \"Checking…\" while verifying is true"
    requirement: "UX-01"
    verification:
      - kind: unit
        ref: "src/components/Spinner.test.ts#SecretField.tsx renders <Spinner at least once"
        status: pass
    human_judgment: true
    rationale: "Wiring proven by source-invariant test; visual confirmation deferred to next UAT pass per plan <verification>."

duration: ~12min
completed: 2026-09-16
status: complete
---

# Phase 08 Plan 08: Loading Spinner for Argon2id Waits Summary

**Reusable Spinner component (Icon + Tailwind animate-spin) wired into Login submit, Login Windows Hello, Register submit, and SecretField reprompt-confirm, closing G-08-7 without touching any KDF parameter**

## Performance

- **Duration:** ~12 min
- **Started:** 2026-09-16T17:00:00Z (approx)
- **Completed:** 2026-09-16T17:10:41Z
- **Tasks:** 2
- **Files modified:** 5 (2 created, 3 modified)

## Accomplishments
- Created `src/components/Spinner.tsx`, a reusable loading affordance composed from the existing `Icon` choke point (`name="arrow-repeat"`) and Tailwind's `animate-spin` utility - no new dependency
- Wired the spinner into all three slow Argon2id paths named in the gap: Login's submit button and Windows Hello unlock button, Register's submit button, and SecretField's master-password reprompt Confirm button
- Added `src/components/Spinner.test.ts`, a source-invariant test (following `src/newui/lib/parity.test.ts`'s comment-stripped, read-from-disk pattern) that gates all four wiring points against silent regression
- Left every existing `disabled={submitting}` / `disabled={verifying}` guard and `disabled:opacity-50` class untouched - the spinner is additive, not a replacement

## Task Commits

Each task was committed atomically:

1. **Task 1: Create the Spinner component and wire it into the Login submit path** - `5799e35` (feat)
2. **Task 2: Wire the spinner into the reprompt and register paths, and gate the wiring with a test** - `3b6fb5c` (test, RED) + `56f77f3` (feat, GREEN)

**Plan metadata:** (pending - final docs commit follows this SUMMARY)

_Note: Task 2 was tdd="true" and followed RED -> GREEN; no REFACTOR commit was needed since the implementation required no cleanup after GREEN._

## Files Created/Modified
- `src/components/Spinner.tsx` - New reusable Spinner component (Icon + animate-spin)
- `src/components/Spinner.test.ts` - Source-invariant test gating all four wiring points
- `src/screens/Login.tsx` - Submit and Windows Hello buttons render Spinner while `submitting`
- `src/screens/Register.tsx` - Submit button renders Spinner while `submitting`
- `src/components/SecretField.tsx` - Reprompt Confirm button renders Spinner while `verifying`

## Decisions Made
- Spinner routes the bootstrap-icons glyph through `Icon` rather than an inline `<i class="bi bi-...">`, preserving the project's single-choke-point rule for the icon font (documented in `Icon.tsx`'s own head comment)
- No Argon2/KDF parameter changed - per the UAT's locked deferral in `08-UAT.md` "Audit Notes", this plan is feedback-only; the underlying multi-second derivation cost is unchanged and its own decision is deferred until timings are re-measured on a release build

## Deviations from Plan

None - plan executed exactly as written. All four call sites wired, source-invariant test added following the plan's specified `parity.test.ts` pattern, no Argon2/crypto files touched.

## Issues Encountered
None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- G-08-7 closed: every operation the UAT named as a perceptible wait (login, Windows Hello unlock, register, reprompt confirm) now renders a moving loading animation
- Manual/visual confirmation of the animation's perceptibility across the full wait duration remains deferred to the next UAT pass, per this plan's own `<verification>` section - this is expected, not a gap in this plan's scope
- No blockers for 08-09 onward

---
*Phase: 08-new-vault-ui-figma-port*
*Completed: 2026-09-16*

## Self-Check: PASSED

All created/modified files (`src/components/Spinner.tsx`, `src/components/Spinner.test.ts`,
`src/screens/Login.tsx`, `src/screens/Register.tsx`, `src/components/SecretField.tsx`, this
SUMMARY.md) confirmed present on disk. All three commit hashes (`5799e35`, `3b6fb5c`, `56f77f3`)
confirmed present in `git log`.
