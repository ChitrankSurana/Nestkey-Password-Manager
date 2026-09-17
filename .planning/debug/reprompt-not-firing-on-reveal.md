---
status: diagnosed
trigger: "DATA_START UAT gap G-08-2: no master password was asked to view a credential's password (Test 1). Locked decision: add a Global/Per-credential 'Reprompt mode' Settings toggle described in the truth statement, rather than changing per-credential default behavior. Goal: find_root_cause_only — diagnose two sub-questions: (1) is Test 1's 'no prompt' a per-credential-flag test-data artifact (working as designed) or a real gate bypass bug in the New UI; (2) map the current per-credential reprompt gate implementation so a follow-on gap-closure plan can extend it with the Global/Per-credential toggle. Reconcile with Test 4 in the same UAT session, which confirmed reprompt DOES fire correctly for a different credential ('main-secret reprompt on reveal+copy... works correctly'). DATA_END"
created: 2026-09-16T00:00:00Z
updated: 2026-09-16T00:00:00Z
---

## Current Focus

hypothesis: CONFIRMED — Test 1's credential simply had its per-credential `reprompt` flag OFF (the DB/form default), while Test 4's "main-secret" credential had it explicitly ON. No gate-bypass bug exists; the New UI reuses the exact pre-existing (pre-phase-08) reprompt gate unchanged.
test: Traced the full gate chain end to end (DB schema default -> Rust DTO round-trip -> New UI fetch path -> SecretField gate logic) and compared it against Test 4's confirmed-working pass of the identical code path.
expecting: n/a — diagnosis complete, goal is find_root_cause_only.
next_action: Return ROOT CAUSE FOUND to caller. No further investigation needed for this gap; do not apply a fix (out of scope for this session — gap-closure planning owns implementing the Global/Per-credential toggle).

## Symptoms

expected: From UAT Test 1 — tester expected some reprompt behavior to fire when viewing a credential's password.
actual: "No master password was asked to view a credential's password." (Test 1, New UI, desktop launch/parity check.)
errors: None reported.
reproduction: Test 1 in UAT — New UI, viewing a credential's password (reveal). Test 4 (different credential, "main-secret") in the same UAT session reproduced the opposite: reprompt fired correctly on both reveal and copy.
started: Discovered during UAT for phase 08 (New Vault UI Figma port), 2026-09-15/16.

## Eliminated

- hypothesis: The New UI bypasses or fails to wire the reprompt gate (a phase-08 regression specific to the New UI's detail panel).
  evidence: >
    `src/newui/components/CredentialBlock.tsx` and `src/newui/components/DetailField.tsx` both pass `credential.reprompt` straight into the same shared `src/components/SecretField.tsx` component that Classic UI's `src/screens/SiteDetail.tsx` already used (lines 75 and 109 of SiteDetail.tsx call the identical `<SecretField secret={...} reprompt={...} />`). There is no New-UI-specific reprompt logic to diverge — the gate component itself is 100% shared, unmodified in this respect by phase 08. Test 4 (New UI, same session) independently confirmed this exact shared path fires correctly for a reprompt-ON credential, which is strong positive evidence the gate is wired correctly in the New UI.
  timestamp: 2026-09-16T00:00:00Z

- hypothesis: New UI fetches credential data through a different/lossy path that drops or defaults the `reprompt` flag before it reaches SecretField (e.g. a stale-DTO shape, a serialization mismatch, or a race in `useVault.ts`).
  evidence: >
    `src/newui/hooks/useVault.ts` (`refreshCredentials`, lines 75-93) calls `api.listCredentialsForSite(siteId)` and commits the response array to state verbatim — no field is dropped, defaulted, or transformed. A dedicated parity test (`src/newui/lib/parity.test.ts:70-80`) asserts `listCredentialsForSite` is called in exactly one place (`hooks/useVault.ts`) exactly once, so there is no second/divergent fetch path in the New UI. On the Rust side, `core/src/dto.rs:96-107` (`impl From<Credential> for CredentialDto`) round-trips `reprompt: c.reprompt` faithfully from the DB row (`core/src/vault.rs:313`, `SELECT ... reprompt ... FROM credentials`) with no default override. Classic and New UI hit the identical Tauri command (`list_credentials_for_site`) and identical DTO.
  timestamp: 2026-09-16T00:00:00Z

- hypothesis: A partial/broken "Global reprompt" or grace-window mechanism already exists somewhere and is misfiring (suppressing prompts it shouldn't).
  evidence: >
    Repo-wide search for "grace" and "reprompt" across `src/`, `core/`, `src-tauri/`, `web-src-tauri/` turns up only the existing per-credential flag plumbing (DTOs, DB column, editor checkbox, SecretField gate) — zero references to any grace window, reprompt-mode enum, or global-cache concept anywhere in frontend or backend code, and zero fields for it in `core/src/dto.rs`'s `ConfigDto` (theme, auto_lock_minutes, strength_algorithm, accent_color, old_password_days, hibp_opt_in, nav_collapsed, mobile_nav_style, biometric_enabled, ssh_agent_enabled, ui_variant — no reprompt-related field at all). This confirms the Global/Per-credential toggle described in the UAT `truth` is genuinely new functionality to build, not a broken/half-implemented existing feature.
  timestamp: 2026-09-16T00:00:00Z

## Evidence

- timestamp: 2026-09-16T00:00:00Z
  checked: "src/components/SecretField.tsx (the actual gate implementation, shared by both UIs)"
  found: >
    `requestAction(action)` (lines 35-41): `if (reprompt) { setPendingAction(action) } else { runAction(action) }`. `runAction` (lines 26-33) reveals/copies immediately with no re-auth when `reprompt` is falsy. The re-auth modal (lines 96-136) only renders when `pendingAction !== null`, which only becomes non-null when the `reprompt` prop passed in was truthy. This is a pure per-credential boolean gate — no grace window, no global mode, no caching of a prior successful verification across credentials. Matches the UAT truth statement's description of "today's existing behavior" exactly.
  implication: The gate's behavior for a given credential is entirely a function of that one credential's `reprompt` flag. No prompt firing on Test 1 is fully explained by that credential's flag being false — this is expected/designed behavior for the current (per-credential-only) implementation, not a defect in the gate logic.

- timestamp: 2026-09-16T00:00:00Z
  checked: "core/src/vault.rs (schema + credential CRUD) and src/screens/CredentialEditor.tsx (shared Add/Edit Credential form used by both Classic SiteDetail.tsx and New UI NewShell.tsx)"
  found: >
    `core/src/vault.rs:98`: `ALTER TABLE credentials ADD COLUMN reprompt INTEGER NOT NULL DEFAULT 0` — every credential defaults to reprompt OFF at the schema level. `src/screens/CredentialEditor.tsx:24`: `const [reprompt, setReprompt] = useState(existing?.reprompt ?? false)` and line 192's checkbox ("Require master password to view") is unchecked by default for a new credential and only reflects `existing.reprompt` when editing. `CredentialEditor` is imported and used by both `src/screens/SiteDetail.tsx` and `src/newui/NewShell.tsx` (confirmed via repo-wide grep) — one shared editor, no New-UI-specific credential-creation path.
  implication: Any credential created without explicitly checking "Require master password to view" — which is the default for every newly created credential, and for any credential that existed before this checkbox was ever touched — will never trigger a reprompt on reveal/copy. This is data/config state, not application logic.

- timestamp: 2026-09-16T00:00:00Z
  checked: "git history of src/components/SecretField.tsx"
  found: >
    `git log --follow` shows the reprompt gate (and the `reprompt` DB column, CredentialEditor checkbox, SecretField gate logic) was introduced in commit `aa0dcad` ("feat(vault): add custom fields and master-password re-prompt to items"), dated 2026-08-02 — over a month before phase 08 (New Vault UI Figma port) began. This is pre-existing functionality, not something phase 08 introduced or touched.
  implication: Phase 08 did not modify the reprompt gate's logic; it only reused the existing `SecretField`/`CopyButton`/`DetailField` components inside new layout wrappers (`CredentialBlock.tsx`, `DetailField.tsx`). Rules out "phase 08 introduced a regression in the gate itself" as a cause.

- timestamp: 2026-09-16T00:00:00Z
  checked: "08-UAT.md Test 1 vs Test 4 wording"
  found: >
    Test 1 (line 15-19 of 08-UAT.md) was a general "Full desktop launch and Classic/New UI switch parity" check — its expected criteria do not call out a specific reprompt-flagged credential; the tester simply viewed "a credential's password" and observed no prompt. Test 4 (line 30-34) explicitly targeted "reveal/copy a reprompt-flagged credential" and named the tested item "main-secret" — i.e. Test 4's test plan specifically selected a credential known/expected to carry `reprompt: true`, while Test 1's did not specify one and evidently hit a credential with `reprompt: false`.
  implication: The two UAT results are fully consistent, not contradictory — they exercised two different credentials with two different flag values through the identical code path. This is the most economical explanation and requires no code defect.

- timestamp: 2026-09-16T00:00:00Z
  checked: "core/src/dto.rs ConfigDto (Settings surface) and repo-wide grep for 'grace'/'reprompt mode'"
  found: >
    `ConfigDto` (dto.rs:122-134) currently has no reprompt-related field at all (fields present: theme, auto_lock_minutes, strength_algorithm, accent_color, old_password_days, hibp_opt_in, nav_collapsed, mobile_nav_style, biometric_enabled, ssh_agent_enabled, ui_variant). No "grace" or "reprompt mode" identifier exists anywhere in `src/`, `core/`, `src-tauri/`, or `web-src-tauri/` source.
  implication: The Global/Per-credential Reprompt-mode toggle (with configurable grace window) described in the UAT `truth` is net-new functionality with zero existing scaffolding to build on. It needs a new Settings config field (natural home: `ConfigDto` alongside the other per-app settings), a grace-window session-scoped timer (comparable in spirit to the existing idle auto-lock timer), and a branch in `SecretField.requestAction` (or a shared hook wrapping it) to check the global mode/grace-window state before falling back to the per-credential `reprompt` flag check that already exists.

## Resolution

root_cause: "Not a defect. Test 1's tested credential had its per-credential `reprompt` database flag set to false — the schema default (`DEFAULT 0`) and the CredentialEditor form's default (unchecked 'Require master password to view' checkbox) — so `SecretField.requestAction()` took the no-reprompt branch and revealed/copied immediately, exactly as per-credential-mode is designed to behave. Test 4's credential ('main-secret') had `reprompt: true` explicitly set, so the identical, unmodified gate code correctly prompted. The two results are reconciled as expected per-credential-flag variance across two different test credentials, not a bypass or regression — confirmed by tracing the gate (`SecretField.tsx`) is 100% shared and unmodified between Classic and New UI, and the New UI's data-fetch path (`useVault.ts` -> `listCredentialsForSite` -> `CredentialDto`) round-trips the `reprompt` flag faithfully with no default-override or race condition. The UAT gap (G-08-2) is legitimately about a MISSING feature — the Global reprompt mode with a grace window described in the locked design decision — which has zero existing implementation anywhere in the codebase (no Settings field, no grace-window timer, no mode branch), not about a broken existing one."
fix: ""
verification: ""
files_changed: []
