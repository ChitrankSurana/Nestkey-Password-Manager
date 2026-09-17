---
status: diagnosed
trigger: "UAT gap G-08-4 (Test 5, phase 08-new-vault-ui-figma-port): 'A delete control for a site is reachable directly, without first opening edit mode.' User reported: I shouldn't need to click on edit site to see the delete button, it should be available outside somewhere."
created: 2026-09-16T15:54:33Z
updated: 2026-09-16T15:54:33Z
---

## Current Focus

hypothesis: CONFIRMED — see Resolution
test: n/a (goal: find_root_cause_only, diagnosis complete)
expecting: n/a
next_action: none — return ROOT CAUSE FOUND to caller; do not fix (out of scope for this session)

## Symptoms

expected: From UAT Test 5 / gap G-08-4 — a delete-site action should be reachable directly from the site detail view, without first entering edit mode.
actual: New UI's DetailPanel header exposes only "Edit site" (opens the SiteEditor modal) and "Add credential". Delete-site is a button *inside* the SiteEditor modal, so the user must click "Edit site" first to reach it.
errors: None (UX/design gap, not a runtime error).
reproduction: New UI -> select a site with credentials -> DetailPanel header shows Edit site / Add credential only -> must click "Edit site" to open the modal that contains the Delete button.
started: Introduced in Phase 08 (New Vault UI Figma Port), discovered during UAT Test 5 (2026-09-15/16).

## Eliminated

- hypothesis: This is a New-UI-specific coding bug (implementation deviates from Phase 8's own written spec).
  evidence: `8-UI-SPEC.md` Component Inventory (D-11/D-12) explicitly locks `SiteEditor.tsx` as "Reused unchanged as modal overlays" and describes `DetailPanel`'s header as only carrying an "Edit site" affordance — no direct delete control is specified anywhere in `8-UI-SPEC.md` for `DetailPanel`. `src/newui/components/DetailPanel.tsx` (lines 136-157) implements exactly this: header has only "Edit site" and "Add credential" buttons. The code matches its own spec faithfully — this rules out a build-time implementation defect.
  timestamp: 2026-09-16T15:54:33Z

- hypothesis: This is a New-UI regression versus Classic UI (Classic already had a direct delete control that New UI dropped).
  evidence: Read `src/screens/SiteDetail.tsx` (Classic UI's site-detail overlay). Its header (lines 45-65) has the identical "Edit site" / "Add credential" pair with NO direct delete button — site deletion is only reachable via `setEditingSite(true)` -> `SiteEditor` modal (lines 136-149), same as New UI. (Contrast with *credential*-level delete, which SiteDetail.tsx *does* expose directly per-row at lines 81-96 — only site-level delete is edit-gated.) Confirms the edit-gated delete pattern is pre-existing product behavior inherited unchanged by Phase 8, not a New-UI-introduced regression.
  timestamp: 2026-09-16T15:54:33Z

## Evidence

- timestamp: 2026-09-16T15:54:33Z
  checked: `src/newui/components/DetailPanel.tsx` (full file)
  found: Header renders two buttons only — "Edit site" (`onEditSite`, opens SiteEditor via NewShell) and "Add credential". No delete affordance anywhere in DetailPanel.
  implication: Confirms the reported symptom at the code level.

- timestamp: 2026-09-16T15:54:33Z
  checked: `src/screens/SiteEditor.tsx` (full file)
  found: `handleDelete` (lines 41-54) — the only path to `api.softDeleteSite` — lives inside this modal, rendered only when `existing` is set (i.e. only reachable once already in "edit" state). Delete button at lines 90-100, gated behind the same modal as Save.
  implication: Both Classic and New UI funnel site deletion through this single reused component, which conflates "edit" and "delete" into one dialog with no way to invoke delete without opening it.

- timestamp: 2026-09-16T15:54:33Z
  checked: `.planning/phases/08-new-vault-ui-figma-port/8-UI-SPEC.md` (full file)
  found: Component Inventory reuse table (line 56): `SiteEditor`, `CredentialEditor` — "Reused unchanged as modal overlays ... Float above the new three-pane shell with no layout conflict." The `DetailPanel` build-new row (line 67) describes its header only as "site name + url shown as a link+copy `DetailField` row + optional `site.notes`" plus the credential repeater — no delete control is specified. Icon Mapping table (line 88) lists a "Delete / Trash" glyph mapping with precedent files `SiteEditor.tsx`, `SiteDetail.tsx`, `NavRail.tsx` (existing delete icon usages elsewhere), not a directive to add a new direct DetailPanel delete affordance. Copywriting Contract (line 174) documents the delete confirm-dialog copy in the context of `SiteEditor.tsx`, again implying the modal is the intended trigger point.
  implication: The spec approved for Phase 8 execution never asked for a direct/header-level delete control on DetailPanel — it explicitly locked in wholesale, unchanged reuse of the edit-modal-gated delete pattern.

- timestamp: 2026-09-16T15:54:33Z
  checked: `docs/FIGMA_UI_VS_CURRENT_FEATURES.md` (section "3. Present and equivalent on both sides", line 65)
  found: "Item detail panel | The selected item's fields, with edit and delete actions in the header. | The same, opening the existing editor dialogs." — this pre-Phase-8 research/comparison doc explicitly records that the **Figma design** shows edit AND delete as two separate, directly-reachable actions in the detail header, but classifies the **existing app's** behavior (delete nested inside the edit dialog) as already "equivalent," and lists this row under "Present and equivalent on both sides" rather than under a gap/deferred section.
  implication: This is the origin of the gap. The Figma mockup itself specifies a direct delete action, distinct from edit. The pre-phase research doc that fed into `8-CONTEXT.md`/`8-UI-SPEC.md` mischaracterized "delete nested inside edit" as equivalent to "delete as its own direct header action" — they are not equivalent from a UX-reachability standpoint. That mischaracterization is what let `8-UI-SPEC.md`'s D-11/D-12 lock in wholesale, unchanged `SiteEditor.tsx` reuse (edit+delete bundled) without anyone flagging the header-delete affordance as a thing to actually build. `docs/UI_REDESIGN_CAPABILITY_CHECKLIST.md` §1 ("Add/edit site") also only maps `soft_delete_site` to `SiteEditor.tsx`, consistent with the same framing throughout.

## Resolution

root_cause: "The delete-site action is only reachable through the 'Edit site' button opening the shared SiteEditor.tsx modal (which bundles Save and Delete in one dialog) — DetailPanel.tsx's header never got its own direct delete control. This traces back to a mischaracterization during Phase 8 pre-planning: docs/FIGMA_UI_VS_CURRENT_FEATURES.md (§3, 'Item detail panel' row) recorded the Figma design's separate, direct header edit+delete actions as already 'equivalent' to the existing app's edit-modal-gated delete pattern. On that premise, 8-UI-SPEC.md (D-11/D-12) locked in wholesale, unchanged reuse of SiteEditor.tsx as the sole trigger for both edit and delete, and DetailPanel.tsx was built faithfully to that spec. The pattern is not a New-UI regression — Classic UI's SiteDetail.tsx has the identical edit-gated delete-site behavior (verified: only per-credential delete is direct in Classic; site-level delete is not). This is a spec-level design gap inherited from a pre-existing Classic UI behavior that the Figma port carried forward unexamined, not a coding defect against what was actually specified for Phase 8."
fix: ""
verification: ""
files_changed: []
