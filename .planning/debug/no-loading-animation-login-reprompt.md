---
status: diagnosed
trigger: "UAT Test 5 / gap G-08-7 (phase 08-new-vault-ui-figma-port): 'login and password check (reprompt popup) both take too much time with no feedback.' Orchestrator already audited and confirmed the delay itself is core::crypto::derive_key (Argon2id, debug-build artifact, KDF params not to be changed) — this session's job is narrower: find the root cause of the missing loading animation itself (find_root_cause_only, no fix)."
created: 2026-09-16T16:05:05Z
updated: 2026-09-16T16:05:05Z
---

## Current Focus

hypothesis: CONFIRMED - loading state variables already exist in both call sites (Login.tsx's `submitting`, SecretField.tsx's `verifying`) and are already wired into `disabled` attributes, but neither renders an actual loading animation/spinner - Login.tsx shows nothing but 50% opacity dimming, SecretField.tsx shows a static text swap ("Checking...") with no animation. No spinner/loading-animation component exists anywhere in the codebase to reuse as-is.
test: n/a - diagnose only, no fix
expecting: n/a
next_action: none - findings returned to orchestrator for fix planning

## Symptoms

expected: "A loading animation shows during any operation with a perceptible wait (login, master-password reprompt check, and similar)" (08-UAT.md gap G-08-7 truth)
actual: User reported login and reprompt master-password checks both "take too much time with no feedback." Orchestrator's prior audit this session confirmed the wait itself is real (~4s login / ~2s reprompt in debug builds, from `derive_key`'s Argon2id cost) and is out of scope to change; this investigation covers only why no loading indicator is shown during that wait.
errors: None
reproduction: Login screen submit (src/screens/Login.tsx); reprompt master-password modal submit inside SecretField (src/components/SecretField.tsx)
started: Present since Login.tsx and SecretField.tsx were first written (not a regression - loading state UX gap has existed since these components were authored); surfaced now via phase-08 UAT because phase 08 is the first UAT pass to exercise New UI's login/reprompt flows end-to-end with attention to wait-time feedback

## Eliminated

(none - single-pass confirmation, no false hypotheses required investigation given the audit already scoped the delay's source)

## Evidence

- timestamp: 2026-09-16T16:05:05Z
  checked: src/screens/Login.tsx (full file, 155 lines)
  found: >
    `handleSubmit` (lines 37-54): `setSubmitting(true)` -> single `await api.login(...)` (or
    `api.unlockWithPin`) -> `onSuccess` / catch -> `finally { setSubmitting(false) }`. No
    intermediate state update during the await - it's a single synchronous await inside try/finally,
    so there is exactly one place to add a loading indicator. The submit button (line 133-140) has
    `disabled={submitting}` and class `disabled:opacity-50` - the ONLY visual change during the
    ~4s wait is the button dimming to 50% opacity and becoming unclickable. Button text stays
    "Log in" throughout - no spinner icon, no "Logging in..." text, no animation of any kind.
    Same pattern for `handleBiometricUnlock` (lines 56-67) and its button (lines 119-130) -
    `disabled={submitting}`, no other feedback.
  implication: >
    The `submitting` boolean loading-state flag already exists and is already correctly wired to
    gate double-submits via `disabled` - it is NOT unwired/missing state. What's missing is any
    rendering of that state as an actual loading animation. This is a rendering gap, not a
    state-management gap.

- timestamp: 2026-09-16T16:05:05Z
  checked: src/components/SecretField.tsx (full file, 139 lines) - the reprompt gate modal
  found: >
    `confirmReauth` (lines 43-61): `setVerifying(true)` -> single `await api.verifyMasterPassword(password)`
    -> branch on result -> `finally { setVerifying(false) }`. Same single-await-in-try/finally shape
    as Login. The Confirm button (lines 126-132) has `disabled={verifying || password === ""}` AND
    conditionally renders `{verifying ? "Checking…" : "Confirm"}` - a static text swap, no spinner
    icon, no animation. This is marginally better than Login (has *some* textual feedback) but still
    not a "loading animation" per the gap's truth statement, and easy to miss since the modal itself
    doesn't otherwise move or pulse during the ~2s wait.
  implication: >
    Same pattern as Login: state flag (`verifying`) exists and is wired; only the visual
    representation is minimal (text-only, no animation).

- timestamp: 2026-09-16T16:05:05Z
  checked: >
    src/newui/NewApp.tsx (full file) and grep for "SecretField" across src/newui and src/screens
  found: >
    NewApp.tsx imports `Login` directly from `../screens/Login` (same file audited above) - New UI
    does not have its own login screen, it reuses the Classic one verbatim. Similarly,
    `src/newui/components/CredentialBlock.tsx:3` and `src/newui/components/DetailField.tsx:1` both
    import `SecretField` from `../../components/SecretField` (same shared file). Classic UI's
    `src/screens/SiteDetail.tsx` and `src/screens/ViewerVault.tsx` also import the same
    `SecretField`.
  implication: >
    This is a single shared root cause, not two separate ones per UI variant - both Classic and
    New UI's login and reprompt flows go through the exact same two component files. A fix applied
    to `Login.tsx` and `SecretField.tsx` covers both UI variants automatically; no New-UI-specific
    duplicate code path exists for this gap.

- timestamp: 2026-09-16T16:05:05Z
  checked: >
    Grep across src/ for spinner/loading-animation patterns: "spinner|Spinner|animate-spin|
    animate-pulse|LoadingIndicator|Loader|@keyframes spin|role=\"status\"|aria-busy"; also checked
    src/components/ToastHost.tsx, src/components/Icon.tsx, App.css imports, and package.json
  found: >
    No spinner/loading-animation component exists anywhere in the codebase (neither Classic nor
    New UI) - the only `role="status"` hit is ToastHost.tsx's toast container (unrelated, no
    animation, just an ARIA live-region role for the 2s-dismiss toast text). No `animate-spin`,
    `animate-pulse`, or any spin/loader utility is used anywhere. The `Icon` component
    (src/components/Icon.tsx) is a thin wrapper around `bootstrap-icons` glyphs (`bi bi-{name}`,
    imported globally in App.css via `@import "bootstrap-icons/font/bootstrap-icons.css"`) -
    bootstrap-icons ships spinner-appropriate glyphs (e.g. `arrow-repeat`, `arrow-clockwise`) that
    could be animated. Tailwind CSS (`@import "tailwindcss"`, used for all styling throughout both
    UIs) ships a built-in `animate-spin` utility class out of the box - no new dependency needed to
    build a spinner.
  implication: >
    There is no existing spinner component to "wire up" - the loading-animation gap is a full
    absence, not a disconnected/unused component. However, the codebase already has both
    ingredients to build one cheaply and consistently with existing conventions: the `Icon`
    component (bootstrap-icons glyph) + Tailwind's `animate-spin` utility. A fix should compose
    these two existing primitives into one small reusable spinner rather than pulling in a new
    dependency (e.g. react-spinners) or reinventing the icon font.

## Resolution

root_cause: >
  Both perceptible-wait operations (Login.tsx's handleSubmit/handleBiometricUnlock and
  SecretField.tsx's confirmReauth) already track their in-flight state correctly via boolean flags
  (`submitting`, `verifying`) that are set before the single `await` and cleared in `finally` - so
  double-submit is already prevented. But neither call site ever renders that boolean as an actual
  loading animation: Login's buttons only dim to 50% opacity (`disabled:opacity-50`) with no other
  visual change, and SecretField's Confirm button only swaps its label text to "Checking…" with no
  animation. The gap is purely a rendering/UX omission on top of already-correct state management -
  not a missing await, not an unwired flag, and not two separate bugs (both Classic and New UI route
  through the same two shared component files, confirmed via import trace). No spinner/loading
  component exists anywhere in the codebase to reuse as-is, but the codebase already has the two
  ingredients to build one consistently (bootstrap-icons via the existing `Icon` component +
  Tailwind's built-in `animate-spin` utility), so a fix should compose those rather than add a new
  dependency.
fix: (not applied - find_root_cause_only mode)
verification: (not applicable - diagnosis only)
files_changed: []
