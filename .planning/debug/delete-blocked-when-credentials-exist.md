---
status: diagnosed
trigger: "UAT gap G-08-6 (Test 5, phase 08-new-vault-ui-figma-port): 'A site that already has saved credentials can still be deleted (cascading the delete to its credentials), gated behind a confirmation popup — not blocked outright.' User reported: it says I have a credential saved for that site and just stops deletion altogether until all credentials for that site are deleted one by one. User wants: I should be able to delete site (with a confirmation popup) even if it has credentials already saved for it."
created: 2026-09-16T16:01:56Z
updated: 2026-09-16T16:01:56Z
---

## Current Focus

hypothesis: CONFIRMED — see Resolution
test: n/a (goal: find_root_cause_only, diagnosis complete)
expecting: n/a
next_action: none — return ROOT CAUSE FOUND to caller; do not fix (out of scope for this session)

## Symptoms

expected: From UAT Test 5 / gap G-08-6 (truth) — deleting a site that has credentials should offer a confirmed cascade delete (confirmation popup, then site + its credentials are soft-deleted together), not be refused outright.
actual: User reported: "it says I have a credential saved for that site and just stops deletion altogether until all credentials for that site are deleted one by one." Delete is blocked with an error message instead of proceeding after confirmation.
errors: "This site still has credentials. Delete them first, then delete the site." — returned as an `invalid_input` CommandError from the backend, surfaced verbatim in SiteEditor.tsx's catch block (`isCommandError(err) ? err.message : "Something went wrong. Please try again."`).
reproduction: Test 5 in UAT (08-UAT.md) — either UI, select a site that has one or more live (non-trashed) credentials, click "Edit site" (only reachable path per sibling gap G-08-4), click "Delete" in the edit-site modal.
started: Pre-existing backend behavior, not introduced by Phase 8. First surfaced/reported during Phase 8 UAT (New Vault UI Figma port) because Phase 8 was the first UAT round to actually exercise deleting a site with live credentials.

## Eliminated

- hypothesis: This is a New-UI-specific frontend guard added by Phase 8 (e.g. a credential-count check in SiteEditor.tsx or DetailPanel.tsx that pre-emptively disables/blocks the Delete button or short-circuits before calling the API).
  evidence: Read `src/screens/SiteEditor.tsx` in full (124 lines). `handleDelete` (lines 41-54) contains no credential-count check of any kind — it unconditionally calls `window.confirm(...)` then `await api.softDeleteSite(existing.id)` for every site regardless of credential count, and only surfaces whatever error comes back from the API call. The Delete button itself (lines 90-100) is never disabled based on credential count — only `disabled={submitting}`. This is the exact same shared component used by both Classic UI (`Vault.tsx:113`) and New UI (`NewShell.tsx:230-244`), per sibling gaps G-08-4/G-08-5's findings — so any block cannot be frontend/New-UI-specific.
  timestamp: 2026-09-16T16:01:56Z

- hypothesis: This is a SQLite foreign-key `ON DELETE`/`ON DELETE RESTRICT` constraint failure surfacing as a generic DB error, misread by the user as an intentional message.
  evidence: Read `core/src/vault.rs:50-73` (schema). `credentials.site_id INTEGER NOT NULL REFERENCES sites(id)` has no `ON DELETE` clause at all (SQLite default action, not `CASCADE`). More importantly, this is entirely moot: the whole app uses **soft delete** exclusively — `soft_delete_site` (`core/src/vault.rs:218-227`) is a plain `UPDATE sites SET deleted_at = datetime('now') ...`, never a SQL `DELETE`. SQLite foreign-key `ON DELETE` actions only fire on an actual `DELETE` statement; they are never invoked by this code path. No FK constraint is involved in the block at all.
  timestamp: 2026-09-16T16:01:56Z

- hypothesis: The block happens inside the low-level `vault::soft_delete_site` function itself (i.e., it's baked into the primitive, not a separate business-rule layer).
  evidence: Read `core/src/vault.rs:218-227` in full. This function is a single unconditional `UPDATE`, with zero awareness of credentials — it has no credential-count check, no join, nothing. The refusal is NOT here.
  timestamp: 2026-09-16T16:01:56Z

## Evidence

- timestamp: 2026-09-16T16:01:56Z
  checked: `core/src/commands.rs:260-275` — `do_soft_delete_site`
  found: |
    ```rust
    pub fn do_soft_delete_site(state: &AppState, site_id: i64) -> Result<(), CommandError> {
        let vault_guard = state.vault.lock().unwrap();
        let (conn, _) = vault_guard.as_ref().ok_or_else(crate::error::locked_error)?;
        // Business rule enforced here, not in vault::soft_delete_site itself: that lower-level
        // function is also used to cascade-delete a site's credentials from every read path, and
        // several existing tests rely on it doing so unconditionally. The command layer is the right
        // place to refuse the destructive action instead - there's no restore-site UI, so an orphaned
        // site's credentials would otherwise become permanently unreachable.
        let live = crate::vault::count_live_credentials_for_site(conn, site_id)?;
        if live > 0 {
            return Err(crate::error::invalid_input_error(
                "This site still has credentials. Delete them first, then delete the site.",
            ));
        }
        Ok(crate::vault::soft_delete_site(conn, site_id)?)
    }
    ```
    This is the exact block. It is a deliberate command-layer business rule (with an explanatory code comment already in place, not an accidental artifact), refusing the destructive action outright with an `invalid_input` error whenever `count_live_credentials_for_site` (a `SELECT count(*) FROM credentials WHERE site_id = ?1 AND deleted_at IS NULL`) returns > 0. The error text matches the user's report verbatim.
  implication: This is THE root-cause location. It is intentional defensive code, not a bug/oversight — the comment explains a real underlying constraint (see next evidence entry) rather than an arbitrary restriction.

- timestamp: 2026-09-16T16:01:56Z
  checked: `core/src/vault.rs:291-303` — `list_credentials_for_site` (the read path the code comment above refers to)
  found: The query is `SELECT ... FROM credentials c JOIN sites s ON s.id = c.site_id WHERE c.site_id = ?1 AND c.deleted_at IS NULL AND s.deleted_at IS NULL` — it filters out credentials belonging to a soft-deleted site (`s.deleted_at IS NULL`), even though those credentials' own `deleted_at` is still NULL (i.e., still "live" by their own flag).
  implication: This confirms the code comment's stated rationale precisely: if `do_soft_delete_site` were allowed to soft-delete a site while it still has live credentials, WITHOUT also soft-deleting those credentials, the credentials would become permanently orphaned — still "live" (not in Trash, since Trash is `deleted_at IS NOT NULL`-driven), yet invisible from every current read path because their parent site is gone and there is no "restore site" UI to bring the site (and by extension visibility of its credentials) back. The current guard exists specifically to prevent creating unreachable, non-recoverable data — a genuine data-integrity concern, not mere caution.

- timestamp: 2026-09-16T16:01:56Z
  checked: `core/src/vault.rs:218-227` — `soft_delete_site` (the low-level primitive)
  found: `UPDATE sites SET deleted_at = datetime('now') WHERE id = ?1 AND deleted_at IS NULL` — a single-table UPDATE. It does NOT cascade to the `credentials` table in any way (no cascading UPDATE of `credentials.deleted_at`, no application-level loop over that site's credentials).
  implication: The data model has NO cascade-delete mechanism implemented anywhere (neither at the SQL/schema level — no `ON DELETE CASCADE`, which would be irrelevant anyway since this is UPDATE-based soft delete, not SQL DELETE — nor at the application/vault-function level). "Cascade delete" as the UAT truth describes it (site delete cascades to credentials) does not exist in the codebase today. Building it means teaching either `vault::soft_delete_site` or `commands::do_soft_delete_site` to also soft-delete every live credential for that site (e.g., loop calling `vault::soft_delete_credential` for each id, or a single `UPDATE credentials SET deleted_at = datetime('now') WHERE site_id = ?1 AND deleted_at IS NULL`) in the same transaction as the site update — this is a backend/core code change, not a schema migration (no new columns/constraints needed) and not a frontend-only fix.

- timestamp: 2026-09-16T16:01:56Z
  checked: `core/src/commands.rs:1203-1226` — existing test coverage: `soft_delete_site_refuses_when_site_still_has_a_live_credential` and `soft_delete_site_succeeds_once_its_credential_is_gone`
  found: Both tests assert the CURRENT refuse-outright behavior as correct/expected (`assert_eq!(err.code(), "invalid_input")`, and a follow-up test proving delete only succeeds once the credential is separately soft-deleted first). No test anywhere exercises or expects a cascade-delete path.
  implication: This is long-standing, deliberately tested product behavior (predates Phase 8 — this is core-layer code, not New UI code), not a regression or accidental gap. Changing it to cascade-on-confirm will require updating/adding tests here too, and is a genuine behavior change to `pwmgr-core`, not just a UI change.

- timestamp: 2026-09-16T16:01:56Z
  checked: `web-src-tauri/src/handlers.rs:351-357` — the web-portal's `soft_delete_site` HTTP handler
  found: `async fn soft_delete_site(...) { do_soft_delete_site(&s.app_state, b.site_id)?; ... }` — calls the exact same `core::commands::do_soft_delete_site` used by the Tauri desktop command (`src-tauri/src/commands.rs:190-192`).
  implication: The refusal is universal across every frontend surface (Classic desktop UI, New desktop UI, web portal) — all three route through one shared core function. This rules out any UI-variant-specific or transport-specific (Tauri IPC vs HTTP) explanation; the root cause is single and centralized in `pwmgr-core`.

- timestamp: 2026-09-16T16:01:56Z
  checked: `docs/UI_REDESIGN_CAPABILITY_CHECKLIST.md:60` and `.planning/phases/08-new-vault-ui-figma-port/8-UI-SPEC.md:174` (Copywriting Contract)
  found: The capability checklist (pre-Phase-8 research artifact) already documents this as existing, known backend behavior: `| soft_delete_site | delete; refuses if live credentials remain | ✓ | | ✓ |`. The Phase 8 UI-SPEC's Copywriting Contract explicitly locks the delete-confirm copy as "only reachable when the site has no live credentials, matching `soft_delete_site`'s backend refusal" — i.e., Phase 8 planning explicitly knew about and intentionally preserved this refuse-outright behavior rather than introducing a cascade flow, treating it as a given backend constraint rather than something in scope to change.
  implication: Phase 8 faithfully implemented existing backend behavior per its own approved spec — this is not an implementation defect against the Phase 8 plan. The UAT gap (G-08-6) is a genuine product-behavior-change request (block → confirmed cascade) that was never surfaced as a decision point during Phase 8 planning, not a bug introduced by the New UI port.

- timestamp: 2026-09-16T16:01:56Z
  checked: `src/screens/SiteEditor.tsx:91` — Tooltip copy on the Delete button
  found: `<Tooltip text="Delete this site (only if it has no credentials)">` wraps the Delete button.
  implication: The frontend already documents/hints at this constraint to the user proactively (via tooltip), further corroborating that this is treated as known, accepted (if now-disputed) behavior rather than a surprising bug in the existing product design.

## Resolution

root_cause: "Deleting a site with live credentials is deliberately refused outright by a command-layer business-rule guard in `core/src/commands.rs::do_soft_delete_site` (lines 260-275): it calls `vault::count_live_credentials_for_site` and returns an `invalid_input` error ('This site still has credentials. Delete them first, then delete the site.') whenever that count is > 0, before ever calling the actual delete primitive. This is centralized in `pwmgr-core` and is reached identically by every frontend surface (Tauri desktop commands, web-portal HTTP handlers) and both UI variants (Classic and New) via the single shared `SiteEditor.tsx` component — so it is not a New-UI-introduced bug or frontend guard, and not a SQL foreign-key/schema-level restriction (soft delete is UPDATE-based; no `ON DELETE CASCADE` exists or would even apply). The guard exists for a real, documented reason (code comment at commands.rs:263-267): the lower-level `vault::soft_delete_site` (`core/src/vault.rs:218-227`) does NOT cascade — it is a bare single-table UPDATE with no awareness of the site's credentials — and `list_credentials_for_site` (`core/src/vault.rs:291-303`) filters out any credential whose parent site is soft-deleted. So if the site-level block were simply removed without also implementing cascade behavior, the site's credentials would become permanently orphaned: still 'live' by their own `deleted_at` flag (not in Trash), yet unreachable through any existing read path, with no 'restore site' UI to ever get them back. This refuse-outright design is long-standing (pre-Phase-8) core behavior, explicitly covered by two existing unit tests (`soft_delete_site_refuses_when_site_still_has_a_live_credential`, `soft_delete_site_succeeds_once_its_credential_is_gone`), and was knowingly preserved (not introduced) by Phase 8's own UI-SPEC Copywriting Contract, which locked the delete-confirm copy specifically around the existing backend refusal. It is deliberate defensive code protecting real data reachability, not an accidental artifact — but the underlying data model has NO cascade-delete mechanism implemented anywhere (schema or application layer) today, so closing gap G-08-6 as specified (confirmed cascade instead of outright block) requires a genuine `pwmgr-core` behavior change — teaching `do_soft_delete_site` (or `vault::soft_delete_site`) to also soft-delete every live credential for that site in the same operation — not merely a frontend flow change (swapping the blocking alert for a confirm dialog alone would just surface the same 'still has credentials' error after confirmation, since the backend would still refuse)."
fix: ""
verification: ""
files_changed: []
