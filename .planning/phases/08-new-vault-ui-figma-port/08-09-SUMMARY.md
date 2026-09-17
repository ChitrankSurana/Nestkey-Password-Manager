---
phase: 08-new-vault-ui-figma-port
plan: 09
subsystem: vault-core-and-ui
tags: [rust, sqlite, react, tauri, dialog-plugin]

# Dependency graph
requires:
  - phase: 08-new-vault-ui-figma-port (plans 01-08)
    provides: DetailPanel.tsx/NewShell.tsx (New UI detail column), SiteEditor.tsx/SiteDetail.tsx
      (Classic delete path), the existing @tauri-apps/plugin-dialog dependency proven for file
      pickers
provides:
  - "core::vault::soft_delete_site cascading to a site's live credentials inside one transaction"
  - "core::vault::restore_credential reviving its cascade-deleted site so a restored credential is
    never live-but-unreachable"
  - "src/lib/confirm.ts: confirmDialog()/siteDeleteMessage() - the one confirm seam every
    destructive-action prompt in this app should route through going forward"
  - "src/newui/components/DetailPanel.tsx: a direct Delete site control in the header, reachable
    without opening edit mode"
affects: [08-10, 08-11, 08-12, any future destructive-action confirm anywhere in the app]

# Actuals (#2632)
actuals:
  tokens: 4876
  tasks: 4
  commits: 3
plan_head_before: 0fd15f699de69767d8f4d57edbfc169c269c950f

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Confirm seam (src/lib/confirm.ts) branching on isTauri: native @tauri-apps/plugin-dialog
      confirm() on desktop (window.confirm silently renders nothing in WRY-based Tauri v2
      webviews), window.confirm in web mode where the browser global works correctly - the two
      modes need genuinely different implementations, not one seam papering over a bug that only
      exists in one of them"
    - "Cascade delete inside one unchecked_transaction (not conn.transaction(), which needs &mut
      Connection unavailable at this call site) - a half-applied delete must never be observable,
      and the cascade UPDATE's own AND deleted_at IS NULL filter preserves an already-trashed
      credential's original timestamp"
    - "Business-rule guard removed from the command layer, not replaced with a different guard -
      the destructive action is now gated by a frontend confirmation instead of a hard backend
      refusal, with Trash + restore_credential's site-revival as the recovery path"

key-files:
  created:
    - src/lib/confirm.ts
  modified:
    - core/src/vault.rs
    - core/src/commands.rs
    - src-tauri/capabilities/default.json
    - src/screens/SiteEditor.tsx
    - src/screens/SiteDetail.tsx
    - src/screens/Backup.tsx
    - src/newui/components/DetailPanel.tsx
    - src/newui/NewShell.tsx

key-decisions:
  - "Task 1 checkpoint:decision auto-resolved to Option A (restoring a cascade-deleted credential
    also revives its site) per this dispatch's auto-mode instructions: the plan's own
    default_if_unanswered names A, and 8-CONTEXT.md's locked decision that site-with-credentials
    delete must cascade behind one confirmation - not leave an orphan state - independently rules
    out Option B (orphan returns) and Option C (unrecoverable). No live checkpoint stop occurred."
  - "siteDeleteMessage's cascade-count copy pluralizes credential/credentials and always ends with
    the required 'This cannot be undone.' suffix; the no-credential branch returns the UAT's
    byte-exact string unchanged so G-08-5's regression test keeps matching."
  - "restore_credential's site-revival UPDATE is a correlated subquery keyed off the credential
    being restored, scoped with AND deleted_at IS NOT NULL so it is a true no-op (0 rows affected)
    whenever the site was already live - restoring an individually-trashed credential under a live
    site behaves exactly as before."
  - "The existing commands.rs test asserting the old refuse-outright behavior was renamed to
    do_soft_delete_site_succeeds_when_the_site_still_has_credentials and its assertions inverted,
    per the plan's explicit 'locate and update, do not delete' instruction, rather than being
    deleted and replaced with a same-named new test."

requirements-completed: [VAULT-01]

coverage:
  - id: D1
    description: "soft_delete_site cascades to a site's live credentials in one transaction,
      preserves an already-trashed credential's own timestamp, and keeps the NotFound contract for
      an already-deleted site"
    requirement: VAULT-01
    verification:
      - kind: unit
        ref: "cargo test -p pwmgr-core soft_delete_site (5 passed)"
        status: pass
      - kind: unit
        ref: "cargo test --workspace (317 pwmgr-core + 2 end-to-end + 17 pwmgr-web + 5 http-integration, all passed)"
        status: pass
    human_judgment: false
  - id: D2
    description: "restore_credential revives a cascade-deleted site so the restored credential is
      reachable again, and is a no-op when the site was already live"
    requirement: VAULT-01
    verification:
      - kind: unit
        ref: "cargo test -p pwmgr-core restore_credential (2 passed)"
        status: pass
    human_judgment: false
  - id: D3
    description: "do_soft_delete_site (command layer) no longer refuses a non-empty site - it
      delegates straight to the now-cascading vault::soft_delete_site"
    requirement: VAULT-01
    verification:
      - kind: unit
        ref: "cargo test -p pwmgr-core do_soft_delete_site_succeeds_when_the_site_still_has_credentials"
        status: pass
    human_judgment: false
  - id: D4
    description: "confirmDialog()/siteDeleteMessage() exist in src/lib/confirm.ts, dialog:allow-confirm
      is granted on the main window, both former window.confirm() call sites (SiteEditor.tsx,
      Backup.tsx) route through confirmDialog, and the no-credential message is byte-identical to
      the UAT's expected copy"
    requirement: VAULT-01
    verification:
      - kind: other
        ref: "npm run build (exit 0)"
        status: pass
      - kind: other
        ref: "! grep -q 'window\\.confirm(' src/screens/SiteEditor.tsx src/screens/Backup.tsx (0 matches)"
        status: pass
      - kind: other
        ref: "grep -c 'dialog:allow-confirm' src-tauri/capabilities/default.json (1)"
        status: pass
      - kind: other
        ref: "grep -c 'confirmDialog' src/screens/SiteEditor.tsx (2), src/screens/Backup.tsx (2)"
        status: pass
      - kind: other
        ref: "grep -c 'This cannot be undone.' src/lib/confirm.ts (3)"
        status: pass
    human_judgment: false
  - id: D5
    description: "New UI detail panel header shows a direct Delete site control that confirms via
      the shared seam, cascades, clears the selection and refreshes the list, without a trip
      through edit mode; src/newui/lib/parity.test.ts's seven standing invariants still pass"
    requirement: VAULT-01
    verification:
      - kind: unit
        ref: "npm test (6 files, 29 passed)"
        status: pass
      - kind: unit
        ref: "npx vitest run src/newui/lib/parity.test.ts (7 passed)"
        status: pass
      - kind: other
        ref: "grep -c 'Delete site' src/newui/components/DetailPanel.tsx (1)"
        status: pass
      - kind: other
        ref: "grep -c 'onSiteDeleted' src/newui/NewShell.tsx (1)"
        status: pass
      - kind: other
        ref: "cargo build -p pwmgr-app (exit 0)"
        status: pass
    human_judgment: false
  - id: D6
    description: "Manual end-to-end confirmation: delete an empty site from the detail header and
      confirm the exact copy renders as a native dialog; cancel and confirm nothing was deleted;
      delete a site with two credentials and confirm the cascade message, the site disappearing,
      and both credentials appearing in Trash; restore one from Trash and confirm the site returns"
    verification: []
    human_judgment: true
    rationale: "Plan's own <verification> section defers this to the next UAT pass (desktop build)
      per workflow.human_verify_mode=end-of-phase, consistent with every prior plan in this phase
      (08-01 through 08-08)."

duration: ~25min
completed: 2026-09-16
status: complete
commits: 3
---

# Phase 8 Plan 9: Cascade Site Delete, Cross-Platform Confirm, Direct Delete Control Summary

**Site deletion is now one coherent, confirmable, reachable operation in both UIs: `soft_delete_site` cascades to a site's live credentials inside one transaction, a new `confirmDialog` seam renders visibly in the Tauri webview (fixing a `window.confirm()` no-op), and the New UI detail panel gains a direct Delete site control that no longer requires opening edit mode first — closing gaps G-08-4, G-08-5, and G-08-6 together.**

## Performance

- **Duration:** ~25 min
- **Tasks:** 4 (1 `checkpoint:decision` auto-resolved, 3 `type="auto"`)
- **Files modified:** 9 (1 created, 8 modified)
- **Commits:** 3

## Accomplishments

- **Task 1 (auto-resolved decision):** Cascade-restore semantics locked to Option A — restoring a
  cascade-deleted credential from Trash also revives its site, so nothing is ever orphaned and no
  restore-site UI is needed.
- **Task 2:** `core::vault::soft_delete_site` rewritten to cascade to a site's live credentials
  inside one `unchecked_transaction`; the cascade UPDATE's own `AND deleted_at IS NULL` filter
  preserves an already-trashed credential's original Trash timestamp. `restore_credential` gained
  a companion UPDATE that revives the credential's site if it was cascade-deleted (a true no-op
  when the site was already live). `core::commands::do_soft_delete_site`'s refuse-outright guard
  is gone — it now delegates straight to the cascading `vault::soft_delete_site`. Five new
  `vault.rs` tests plus one renamed/inverted `commands.rs` test cover all of this; full
  `cargo test --workspace` (317 + 2 + 17 + 5 tests) passes.
- **Task 3:** New `src/lib/confirm.ts` exports `confirmDialog()` (branches on `isTauri`: native
  `@tauri-apps/plugin-dialog confirm()` on desktop, `window.confirm` in web mode) and
  `siteDeleteMessage()` (byte-exact `Delete "{name}"? This cannot be undone.` for the no-credential
  case; names the cascading credential count otherwise). `src-tauri/capabilities/default.json`
  gained `dialog:allow-confirm`. `SiteEditor.tsx`'s `handleDelete` and `Backup.tsx`'s
  restore-cascade confirm both now route through `confirmDialog` instead of the
  Tauri-webview-silent `window.confirm()`. `SiteDetail.tsx` (the one Classic-UI change) passes
  `credentialCount` to `SiteEditor` so Classic's confirm is honest about the now-cascading backend.
- **Task 4:** `src/newui/components/DetailPanel.tsx` gained an `onSiteDeleted` prop, a local
  `deleteError` state (mirroring `CredentialBlock`'s convention), and a `handleDeleteSite` handler
  building the confirm message from the live `credentials` length. A Delete site button now sits
  in the header alongside Edit site and Add credential, styled with the same danger tokens
  Classic's delete button uses. `src/newui/NewShell.tsx` wires `onSiteDeleted` to clear the
  selection and refresh the site list, and passes `credentialCount` to the `editingSite`
  `SiteEditor` so both delete entry points show the same honest confirmation.
  `src/newui/lib/parity.test.ts`'s seven standing invariants still pass unchanged.

## Task Commits

1. **Task 2: Cascade `soft_delete_site`, drop the refuse-outright guard** - `2c714e6` (feat)
2. **Task 3: Cross-platform confirm seam, used everywhere** - `e891f5a` (feat)
3. **Task 4: Direct Delete site control in the New UI detail panel header** - `32cff35` (feat)

**Plan metadata:** this file's commit (docs: complete plan)

_Note: Task 1 (`checkpoint:decision`) produced no commit of its own — it was auto-resolved per
this dispatch's auto-mode instructions and its decision folded into Task 2's implementation._

## Files Created/Modified

- `src/lib/confirm.ts` - `confirmDialog()`/`siteDeleteMessage()`, the shared cross-platform confirm seam
- `core/src/vault.rs` - `soft_delete_site` cascades in one transaction; `restore_credential` revives its site
- `core/src/commands.rs` - `do_soft_delete_site`'s refuse-outright guard removed
- `src-tauri/capabilities/default.json` - `dialog:allow-confirm` granted on the main window
- `src/screens/SiteEditor.tsx` - `handleDelete` routes through `confirmDialog`, gains `credentialCount` prop
- `src/screens/SiteDetail.tsx` - passes `credentialCount` to `SiteEditor`
- `src/screens/Backup.tsx` - restore-cascade confirm routes through `confirmDialog`
- `src/newui/components/DetailPanel.tsx` - direct Delete site header control, `deleteError` state
- `src/newui/NewShell.tsx` - `onSiteDeleted` wiring, `credentialCount` passed to `editingSite` `SiteEditor`

## Decisions Made

See `key-decisions` in frontmatter. Summary:
1. Task 1's checkpoint auto-resolved to Option A per the dispatch's auto-mode instructions and the
   already-locked product decision in `8-CONTEXT.md` — no live checkpoint stop occurred.
2. `siteDeleteMessage`'s cascade copy pluralizes correctly and always ends with the required "This
   cannot be undone." suffix; the no-credential branch is untouched to keep matching the UAT's
   exact expected string.
3. `restore_credential`'s site-revival UPDATE is a correlated subquery scoped to only fire when the
   site is actually deleted, making it a true no-op for the already-common case of restoring a
   credential under a still-live site.
4. The pre-existing refusal test in `commands.rs` was renamed and inverted rather than deleted and
   replaced, per the plan's explicit instruction to locate and update it.

## Deviations from Plan

None - plan executed as written, including the pre-approved auto-resolution of Task 1's checkpoint.

## Issues Encountered

None. All automated verification passed on the first attempt: `cargo test -p pwmgr-core
soft_delete_site` (5/5), `cargo test -p pwmgr-core restore_credential` (2/2), `cargo test
--workspace` (all passed), `npm run build` (exit 0, both after Task 3 and after Task 4), `npm test`
(29/29), `npx vitest run src/newui/lib/parity.test.ts` (7/7), and `cargo build -p pwmgr-app` (exit
0).

## User Setup Required

None - no external service configuration required. `@tauri-apps/plugin-dialog` was already a
direct dependency (`^2.7.1`), already proven working for the existing file pickers.

## Next Phase Readiness

- G-08-4, G-08-5, and G-08-6 are all closed pending the deferred end-of-phase UAT walkthrough
  (D6 above): a delete control is directly reachable from the New UI detail header, the confirm
  dialog now renders in the Tauri webview at both occurrences (site delete, restore-backup
  cascade), and a site with credentials deletes behind a confirmation instead of being refused
  outright.
- `src/lib/confirm.ts` is now the sanctioned seam for any future destructive-action confirmation
  anywhere in this app, in either UI — worth reusing rather than reintroducing a bare
  `window.confirm()` in any later plan.
- Ready for 08-10 (G-08-1: native save-location picker for Create Backup).

---
*Phase: 08-new-vault-ui-figma-port*
*Completed: 2026-09-16*

## Self-Check: PASSED

All 9 key files verified present on disk (`[ -f ]`), and all 3 task commits (`2c714e6`, `e891f5a`,
`32cff35`) verified present in `git log --oneline --all`.
