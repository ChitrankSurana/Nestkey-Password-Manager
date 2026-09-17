---
phase: 08-new-vault-ui-figma-port
plan: 04
subsystem: ui
tags: [react, tailwind, vitest, tdd, security]

# Dependency graph
requires:
  - phase: 08-03
    provides: "src/newui/NewShell.tsx three-pane shell, src/newui/hooks/useVault.ts (selectedSite, credentials, credentialsLoading, the one listCredentialsForSite call site and the vault-locked clear), src/newui/components/PaneSurface.tsx"
provides:
  - "src/newui/lib/credentialLayout.ts: splitCredentials, otherLoginsLabel, strengthSegments, strengthTone - pure credential-repeater and strength-meter logic, unit tested"
  - "src/newui/components/DetailField.tsx, CopyButton.tsx, CredentialBlock.tsx, DetailPanel.tsx: the detail column - site header/notes, the zero/one/many credential repeater, reprompt-gated reveal/copy on both the secret and hidden custom fields, TOTP, remarks, a live five-segment strength meter, and the pinned empty/loading states"
  - "src/newui/NewShell.tsx: detail column now renders DetailPanel for the current selection; adds a `screen` state slice (initialised to \"vault\") for plan 08-05 to dispatch the five secondary panes into"
affects: [08-05, 08-06]

# Actuals (#2632)
actuals:
  tokens: 5827
  tasks: 3
  commits: 4
plan_head_before: e52e4f04e3e89cfb7712326bf199c14eeebcbd2b

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Pure-logic module (no React, no api import) for repeater/strength-meter mapping, unit tested under vitest's node environment, following src/newui/lib/vaultSelectors.ts's established style"
    - "Per-credential expand state lifted into the parent (DetailPanel) rather than owned by CredentialBlock itself - primary is always expanded, `others` render collapsed (header only, secret unmounted) until individually toggled, so a collapsed credential's plaintext is never sitting in the DOM (T-08-15)"
    - "Strength-tone-to-CSS mapping kept out of the pure logic module (strengthTone returns bare strings) and resolved to Tailwind/CSS-variable classes only inside the one React component that renders it (CredentialBlock), keeping the same CSS-free/testable-under-node boundary vaultSelectors.ts already established"
    - "Detail column's `screen` state slice added to NewShell now, wired to real setScreen calls on all six nav destinations, with only the \"vault\" branch rendering content - forward-compatible scaffolding for plan 08-05's secondary panes without building them early"

key-files:
  created:
    - src/newui/lib/credentialLayout.ts
    - src/newui/lib/credentialLayout.test.ts
    - src/newui/components/DetailField.tsx
    - src/newui/components/CopyButton.tsx
    - src/newui/components/CredentialBlock.tsx
    - src/newui/components/DetailPanel.tsx
  modified:
    - src/newui/NewShell.tsx

key-decisions:
  - "Task 1 followed the TDD cycle (tdd=\"true\"): RED commit (failing test importing a module that did not yet exist) before GREEN commit (implementation). No REFACTOR commit - the first-pass implementation needed no cleanup."
  - "credentialLayout.ts's strengthTone doc comment was reworded mid-task to avoid the literal substring `var(--...)` after the first draft accidentally tripped the file's own \"contains no CSS variable names\" acceptance criterion via a comment, not code - reworded to describe the design-token boundary without naming a CSS custom property."
  - "CredentialBlock's `onToggle`/`expanded` props are driven by DetailPanel's local per-credential Set<number> state (which of the `others` are individually expanded), separate from the single `othersOpen` boolean that shows/hides the whole \"Other logins (N-1)\" group - two independent pieces of local UI state, neither persisted, matching D-08's \"local UI state only\" instruction and keeping unopened credentials' secrets fully unmounted even after the group toggle opens."
  - "DetailPanel's `onAddCredential` prop is wired to a no-op in NewShell.tsx - CredentialEditor/modal wiring for adding a credential is CRUD-wiring scope (CONTEXT.md phasing step 3, a later plan), out of this plan's read/reveal/copy-column scope. The CTA renders and is tooltip-reachable; clicking it does nothing yet."
  - "NewShell.tsx's `screen` state slice is wired to real `setScreen` calls on all six destinations (not left as the prior plan's no-ops), because leaving `setScreen` uncalled would trip `noUnusedLocals` under this project's strict tsconfig - only the \"vault\" branch renders anything, so clicking Health/Generator/Trash/Data/Settings currently shows a blank content area until plan 08-05 gives them real panes."

requirements-completed: [HEALTH-02, VAULT-03, VAULT-04, VAULT-05]

coverage:
  - id: D1
    description: "Pure credential-repeater and strength-meter logic (splitCredentials' zero/one/many rule per D-08, otherLoginsLabel's collapse copy, strengthSegments'/strengthTone's mapping of the backend's 0..4 zxcvbn score) is unit tested"
    requirement: HEALTH-02
    verification:
      - kind: unit
        ref: "src/newui/lib/credentialLayout.test.ts (9 cases across splitCredentials x3, otherLoginsLabel x1, strengthSegments x2, strengthTone x3)"
        status: pass
      - kind: other
        ref: "npm run build"
        status: pass
    human_judgment: false
  - id: D2
    description: "Credential block renders username+copy, a neutral (non-destructive-coloured) reprompt badge, the secret through SecretField with no locally-built mask, TOTP when hasTotp, custom fields with the hidden branch also routed through SecretField, remarks in a whitespace-pre-wrap block, and a live five-segment strength meter gated on the credential existing (not on any item type) - with no breach or reuse row anywhere"
    requirement: HEALTH-02
    verification:
      - kind: other
        ref: "grep -c 'SecretField' src/newui/components/DetailField.tsx (3, expects >0)"
        status: pass
      - kind: other
        ref: "grep -c 'reprompt={credential.reprompt}' src/newui/components/CredentialBlock.tsx (2, expects >0)"
        status: pass
      - kind: other
        ref: "grep -rn 'navigator.clipboard' src/newui/ (0 matches, expects 0)"
        status: pass
      - kind: other
        ref: "grep -rn 'repeat(' src/newui/components/CredentialBlock.tsx src/newui/components/DetailField.tsx (0 matches, expects 0)"
        status: pass
      - kind: other
        ref: "grep -c 'strengthSegments' src/newui/components/CredentialBlock.tsx (2, expects >0)"
        status: pass
      - kind: other
        ref: "npm run build"
        status: pass
    human_judgment: false
  - id: D3
    description: "DetailPanel renders the site header (name, url as an accent-coloured link+copy row with target=_blank/rel=noreferrer, notes in whitespace-pre-wrap), the pinned zero-credential empty state, a loading indicator while credentials are still fetching, and the credential repeater; NewShell wires DetailPanel into the detail column and keeps the no-selection empty state; api.listCredentialsForSite still appears exactly once across src/newui/"
    requirement: VAULT-03
    verification:
      - kind: other
        ref: "grep -rn 'listCredentialsForSite' src/newui/ (1 match, in useVault.ts only, expects 1)"
        status: pass
      - kind: other
        ref: "grep -c 'Select a site to view its details' src/newui/NewShell.tsx src/newui/components/DetailPanel.tsx (NewShell.tsx:1, DetailPanel.tsx:0)"
        status: pass
      - kind: other
        ref: "grep -c 'No credentials yet' src/newui/components/DetailPanel.tsx (1, expects >0)"
        status: pass
      - kind: other
        ref: "grep -c 'DetailPanel' src/newui/NewShell.tsx (3, expects >0)"
        status: pass
      - kind: unit
        ref: "npm test (18/18 pass, includes the 9 new credentialLayout cases)"
        status: pass
      - kind: other
        ref: "npm run build"
        status: pass
    human_judgment: false
  - id: D4
    description: "Live interactive verification: reprompt fires on reveal AND copy for both the credential secret and a hidden-type custom field; the mask width is constant regardless of secret length; the zero/one/many repeater behaves correctly in the running app; site notes and credential remarks render with line breaks intact; leaving the app idle until auto-lock empties the detail column and returns to login"
    verification: []
    human_judgment: true
    rationale: "The automated checks confirm code shape (SecretField usage, no local mask, the exactly-one lazy-decryption call site, the pinned copy strings) but not the live rendered behaviour - that reveal/copy actually re-prompt in a running app, that a 6-character and a 40-character password mask to visually identical widths, that the credential repeater and its collapse toggle behave correctly with real data, and that an actual auto-lock event empties the mounted panel. This plan's own <verify><human-check> (desktop npm run tauri dev walkthrough) is deferred to end-of-phase UAT per workflow.human_verify_mode=end-of-phase (default, no .planning/config.json override present in this project), matching 08-01's D4/D5, 08-02's D3, and 08-03's D5 deferrals."

duration: ~20min
completed: 2026-09-14
status: complete
commits: 4
---

# Phase 8 Plan 4: Vault Detail Column Summary

**DetailPanel + CredentialBlock render every credential for a selected site - reveal, copy, TOTP, custom fields, remarks, and a live strength meter - with the reprompt gate and 30-second clipboard clear intact on both the secret and any hidden custom field.**

## Performance

- **Duration:** ~20 min
- **Started:** 2026-09-14T21:20:00+05:30 (approximate)
- **Completed:** 2026-09-14T21:26:58+05:30
- **Tasks:** 3 (Task 1 `tdd="true"`, Tasks 2-3 `type="auto"`)
- **Files modified:** 7 (6 created, 1 modified)

## Accomplishments

- `src/newui/lib/credentialLayout.ts` + `.test.ts`: pure `splitCredentials`/`otherLoginsLabel`/`strengthSegments`/`strengthTone` functions, TDD RED→GREEN, pinning D-08's zero/one/many repeater rule and the backend's 0..4 score → 5-segment/3-tone mapping
- `src/newui/components/DetailField.tsx`: one labelled detail row branching exactly as `SiteDetail.tsx` already branches - `hidden` routes through `SecretField` (same reprompt gate the main secret uses), `boolean` renders Yes/No, `text` renders `font-mono break-all`
- `src/newui/components/CopyButton.tsx`: tooltip-wrapped copy control, the one non-secret clipboard entry point besides `SecretField` itself, both routed through `api.copyToClipboard`'s 30-second auto-clear
- `src/newui/components/CredentialBlock.tsx`: username + copy, a neutral (non-destructive-coloured) reprompt badge, `SecretField` for the secret with no locally-built mask (D-15), `TotpDisplay` when `hasTotp`, custom fields via `DetailField`, remarks in a `whitespace-pre-wrap` block, and a live five-segment strength meter from `api.analyzeStrength` gated on the credential existing rather than any item type (D-17) - no breach or reuse row, since neither has a per-item backend command
- `src/newui/components/DetailPanel.tsx`: site header (name, url as an accent-coloured link+copy row, notes), the pinned "No credentials yet" empty state, a loading indicator while credentials are still fetching, and the credential repeater (primary always expanded, `others` individually collapsible behind "Other logins (N-1)") - renders purely from props, never fetches
- `src/newui/NewShell.tsx`: detail column now renders `DetailPanel` for the current selection, still shows "Select a site to view its details" with nothing selected; adds a `screen` state slice wired to all six nav destinations for plan 08-05 to dispatch its secondary panes into

## Task Commits

1. **Task 1 RED: failing test for credential-repeater and strength-meter logic** - `6cfbe56` (test)
2. **Task 1 GREEN: implement vault credential-repeater and strength-meter logic** - `089a5c5` (feat)
3. **Task 2: credential block - secret, TOTP, custom fields, remarks, reprompt badge, strength meter** - `01a3fcc` (feat)
4. **Task 3: detail panel with the credential repeater, wired into the shell** - `b11ea64` (feat)

No REFACTOR commit for Task 1 - the first-pass implementation needed no cleanup.

**Plan metadata:** commit pending (this SUMMARY + STATE.md/ROADMAP.md)

## Files Created/Modified

- `src/newui/lib/credentialLayout.ts` - `splitCredentials`, `otherLoginsLabel`, `strengthSegments`, `strengthTone`: pure repeater/strength-meter logic, no React/no `api` import, no CSS variable names
- `src/newui/lib/credentialLayout.test.ts` - 9 unit tests pinning the repeater rule, the collapse label, and the meter mapping (including clamp edges)
- `src/newui/components/DetailField.tsx` - `DetailField`: one labelled row, hidden→`SecretField`, boolean→Yes/No, text→`font-mono break-all`
- `src/newui/components/CopyButton.tsx` - `CopyButton`: tooltip-wrapped copy control routed through `api.copyToClipboard`
- `src/newui/components/CredentialBlock.tsx` - `CredentialBlock`: one credential's full render, collapsible, strength meter
- `src/newui/components/DetailPanel.tsx` - `DetailPanel`: site header/notes, credential repeater, empty/loading states
- `src/newui/NewShell.tsx` - detail column renders `DetailPanel`; adds the `screen` state slice

## Decisions Made

See `key-decisions` in frontmatter. Summary:
1. Task 1 followed the full TDD RED→GREEN cycle (no REFACTOR needed).
2. `credentialLayout.ts`'s doc comment was reworded to avoid literally containing `var(--...)`, so the file's own "no CSS variable names" acceptance criterion holds for comments too, not just code.
3. `CredentialBlock`'s per-credential expand state and `DetailPanel`'s group-level "Other logins" toggle are two independent, unpersisted pieces of local UI state - collapsed credentials stay fully unmounted even after the group opens.
4. `onAddCredential` is a no-op in `NewShell.tsx` for now - CredentialEditor/modal wiring is CRUD-wiring scope, a later plan.
5. `NewShell.tsx`'s `screen` state slice is wired to real `setScreen` calls (not left as no-ops) so `noUnusedLocals` doesn't flag the unused setter; only the `"vault"` branch renders content this plan.

## Deviations from Plan

None - plan executed exactly as written. The five key-decisions above are all resolutions of ambiguity within the plan's own stated contracts (the acceptance criterion's literal-string check catching a comment, the plan's own local-UI-state instruction for the repeater, the plan's own CRUD-wiring-is-out-of-scope note, and this project's strict `noUnusedLocals` tsconfig setting), not departures from the plan's intent.

## Issues Encountered

- **Repository state anomaly, not a code issue:** partway through the required-reading step, `git branch --show-current` unexpectedly reported `main` at commit `eaa441b` instead of the expected `feature/bitwarden-port-phase1` at `e52e4f0` - confirmed via `git reflog` (`HEAD@{0}: checkout: moving from feature/bitwarden-port-phase1 to main`) that something external to this session checked out `main` mid-session, which would have caused every subsequent Read/Bash call to see a different repo state (and `src/newui/` to appear entirely absent, since it doesn't exist on `main`). Working tree had zero uncommitted tracked-file changes at the time (`git status --short` showed only pre-existing untracked files), so `git checkout feature/bitwarden-port-phase1` was a safe, lossless recovery back to the correct branch (e52e4f0, matching this task's `<upstream_context>`) before any file writes or commits occurred. All subsequent work happened on the correct branch; the plan's own commits (6cfbe56 onward) are anchored on `e52e4f0` per the plan-head-before ledger. Flagging this because the underlying cause (what checked out `main`) is outside this executor's visibility and may recur.
- Task 3's `<verify>` carries a `<human-check>` (desktop `npm run tauri dev` walkthrough: reprompt on reveal and copy for both the secret and a hidden custom field, constant mask width, the zero/one/many repeater, notes/remarks rendering, auto-lock emptying the panel). Per `workflow.human_verify_mode` defaulting to `end-of-phase` (no `.planning/config.json` override present), this was NOT executed as a mid-flight interactive checkpoint - deferred to the end-of-phase UAT harvest, consistent with 08-01's D4/D5, 08-02's D3, and 08-03's D5 deferrals.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Plan 08-04 is complete: the new UI's detail column renders every credential for a selected site with reveal/copy/TOTP/custom-fields/remarks and a live strength meter, with the reprompt gate and 30-second clipboard clear verified structurally on both the secret and hidden custom fields.
- Ready for plan 08-05 (the five secondary-pane wrapper screens - `GeneratorPane`, `HealthPane`, `DataPane`, `TrashPane`, `SettingsPane` - which `NewShell.tsx`'s new `screen` state slice is already wired for) and plan 08-06 (responsive tiers).
- Deferred to end-of-phase UAT: Task 3's `<human-check>` (live reprompt-on-reveal-and-copy for both the secret and a hidden custom field, constant mask width, the zero/one/many repeater with real data, notes/remarks rendering, auto-lock emptying the panel), consistent with 08-01/08-02/08-03's own deferrals under the same `end-of-phase` human-verify mode.
- **Requirement-tracking note carried from the plan's own "Requirement Scope Note":** VAULT-03/VAULT-04/VAULT-05 as literally worded in REQUIREMENTS.md (item types, favorites, folders) are NOT delivered by this plan or this phase - D-10 cut them for lack of backend persistence. HEALTH-02 is only PARTIALLY delivered (live per-credential strength meter; no per-item breach/reuse status, since neither has a backend command). `requirements-completed` above still copies the plan's frontmatter `requirements` array verbatim, matching the established precedent from 08-01/08-02/08-03's own SUMMARYs in this same phase - the shared-ID gate (`requirements.ready-ids`) defers the actual REQUIREMENTS.md checkbox until every plan declaring an ID has finished, so this does not by itself flip VAULT-04/VAULT-05/HEALTH-02 to Complete.

---
*Phase: 08-new-vault-ui-figma-port*
*Completed: 2026-09-14*

## Self-Check: PASSED

All 7 key files verified present on disk (`[ -f ]`), and all 4 task commits (`6cfbe56`, `089a5c5`, `01a3fcc`, `b11ea64`) verified present in `git log --oneline --all`.
