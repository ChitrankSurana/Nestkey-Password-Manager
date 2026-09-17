# Phase 08 — UI Review

**Audited:** 2026-09-17
**Baseline:** `.planning/phases/08-new-vault-ui-figma-port/8-UI-SPEC.md` (approved design contract)
**Screenshots:** not captured — no dev server detected on :3000 or :5173 (Tauri desktop app, not a
standalone web dev server in this environment). Audit performed as a code-only review against
`src/newui/**`.

---

## Pillar Scores

| Pillar | Score | Key Finding |
|--------|-------|-------------|
| 1. Copywriting | 4/4 | Every empty/error/confirmation string matches the Copywriting Contract verbatim (spot-checked against `vaultSelectors.ts`, `DetailPanel.tsx`, `Settings.tsx`). |
| 2. Visuals | 3/4 | `ItemRow`'s URL subtitle is styled with the section-Label treatment (uppercase, tracking-widest, semibold), making ordinary data read as a shouting label on the vault's primary browsing surface. |
| 3. Color | 4/4 | Zero hardcoded hex/rgb in `src/newui/`; accent (`var(--accent...)`) usage confined to the 8 spec-sanctioned spots (CTAs, selected states, links); danger reserved for delete/purge. |
| 4. Typography | 3/4 | Only the 4 declared sizes (`text-xs/sm/base/xl`) and 2 declared weights (`font-semibold` + default regular) appear anywhere in `src/newui/` — but the same `ItemRow` defect (finding #1) misapplies the Label role outside its declared scope. |
| 5. Spacing | 3/4 | Bulk of spacing snaps to the 4px scale, and the four named `.5`-step exceptions (`ItemRow` padding, icon chips) are used correctly — but `CredentialBlock`/`PaneSurface` (`p-3` = 12px) and `DetailPanel`'s back button (`mb-3` = 12px) drift off both the declared scale and the named exception list. |
| 6. Experience Design | 4/4 | Loading/error/empty states all covered per the UI Considerations table; out-of-order-response guards for search and credential fetch exceed the spec's bar; scope-cut invariants (D-09/D-10/D-12) are enforced by an automated `parity.test.ts` gate, not just convention. |

**Overall: 21/24**

---

## Top 3 Priority Fixes

1. **`ItemRow` renders the site URL with Label typography** (`src/newui/components/ItemRow.tsx:30-32`: `text-xs font-semibold uppercase tracking-widest opacity-70`) — this is the exact class combination the spec reserves for section labels (`DetailField`, "Notes", "Remarks" headers). Applied to every row's URL in the primary vault-browsing list, it makes ordinary data (e.g. `https://example.com`) render as small-caps tracked text, hurting scanability of the list a user spends the most time in. **Fix:** drop `font-semibold uppercase tracking-widest`, keep `text-xs opacity-70` (or `text-sm opacity-70` to match Body) so the subtitle reads as plain secondary text, not a label.

2. **A few paddings fall outside the declared 4px spacing scale and the named `.5`-step exceptions** — `CredentialBlock`/`PaneSurface` card padding (`src/newui/components/CredentialBlock.tsx:72`, `p-3` = 12px) and `DetailPanel`'s mobile back-button (`src/newui/components/DetailPanel.tsx:150`, `mb-3` = 12px) are neither on the {4,8,16,24,32,48,64} scale nor among the spec's four explicitly named exception spots (ItemRow padding, Sidebar logo gap, icon-chip padding). This creates a slightly uneven spacing rhythm between the detail column's cards (12px) and its section gaps (16px, `p-4`/`mb-4` elsewhere). **Fix:** snap `p-3`→`p-4` (16px, matches `PaneSurface`'s sibling usage in the empty-credential state which already uses `p-4`) and `mb-3`→`mb-4` or `mb-2`.

3. **No live visual verification was possible in this pass** — the app is a Tauri desktop build with no exposed web dev server, so this audit is code-only (Tailwind class inspection, no rendered screenshots). Contrast, dark-mode rendering, and actual pixel-level spacing/alignment of the three-pane layout at all three breakpoints (per D-18) have not been visually confirmed. **Fix:** run `npm run tauri dev` (or the project's dev-server equivalent) and capture desktop/tablet/mobile screenshots of the new UI (`newui.html`) before shipping, to confirm the code-level findings above translate correctly and to catch anything a static-analysis pass structurally cannot (actual contrast ratios, icon rendering, real breakpoint transitions).

---

## Detailed Findings

### Pillar 1: Copywriting (4/4)
- `listEmptyState()` in `src/newui/lib/vaultSelectors.ts:40-45` returns the exact "No sites yet" / "Add one to start building your vault." and "No matches" / "Try a different search term." pairs from the Copywriting Contract — the code comment explicitly notes these strings "are the contract itself."
- `DetailPanel.tsx:83-85` renders "No credentials yet" / "Add a credential to store a username and password for this site." verbatim.
- `DetailPanel.tsx:57`/`CredentialBlock.tsx:51` fall back to "Something went wrong. Please try again." on any non-`CommandError`, matching the generic-fallback convention.
- Settings' Interface banner (`src/screens/Settings.tsx:659-661`) reproduces "Interface changed to {label}. Restart the app to see it." per D-07/spec.
- `NewShell.tsx:202-203` renders the detail-empty-state copy ("Select a site to view its details" / "Your vault is encrypted locally") verbatim.
- No generic "Submit"/"OK"/"Click Here" labels found anywhere under `src/newui/`.

### Pillar 2: Visuals (3/4)
- Icon-only buttons are consistently wrapped in the accessible `Tooltip` component (38 occurrences across `src/newui/components/*.tsx`), which shows on both hover and keyboard focus (`src/components/Tooltip.tsx:30-33`) — satisfies UX-01.
- Clear visual hierarchy: Display (site name, `text-xl font-semibold`) → Heading (pane titles, `text-base font-semibold`) → Body (`text-sm`) → Label (`text-xs font-semibold uppercase tracking-widest`, correctly used for "Notes"/"Remarks"/`DetailField` labels).
- **Defect:** `ItemRow.tsx:30-32` applies the Label role's exact class string to the URL subtitle, which is body data, not a section label — a copy-paste-adjacent styling mistake that visually shouts on every row in the list.
- Selected states correctly use `var(--accent-subtle-bg)`/`var(--accent-subtle-fg)` tints (never a solid accent fill) in both `ItemRow.tsx:21` and `SidebarNavItem.tsx:23`, per spec.

### Pillar 3: Color (4/4)
- `grep` for hex/rgb literals under `src/newui/` returns zero matches — all color is routed through CSS custom properties.
- Accent (`var(--accent...)`) usage count: 8, all at spec-sanctioned locations (Add-site/Add-credential CTAs, selected nav/list states, site-url link, "View full health" link).
- Destructive actions (`var(--danger)`/`var(--danger-hover)`) are isolated to the delete-site and delete-credential buttons only.
- The per-item strength meter's "strong" tone uses `bg-green-600 dark:bg-green-400` (`CredentialBlock.tsx:15`) exactly as the spec directs (Tailwind's green theme variables, not a new custom property).

### Pillar 4: Typography (3/4)
- Distinct `text-*` sizes found in `src/newui/`: `xs`, `sm`, `base`, `xl` — exactly the 4 declared roles (Label/Body/Heading/Display), no stray `lg`/`2xl`/etc.
- Distinct weights found: `font-semibold` only (regular is the unstyled default) — matches the "only two weights, no 500/700" rule.
- **Same defect as Pillar 2** counts here too: `ItemRow`'s URL subtitle uses the Label role's typography outside where the spec authorizes it (DetailField/section labels only), so the "correct sizes, wrong role assignment" issue is a typography-contract violation as much as a visual one.

### Pillar 5: Spacing (3/4)
- The large majority of spacing (`p-2`, `p-4`, `gap-1`, `gap-2`, `mb-2`, `mb-4`, `space-y-1`, `space-y-2`) snaps cleanly to the declared 4px scale.
- The four named `.5`-step exceptions are used correctly and only where named: `ItemRow.tsx:19` (`gap-2.5`, `py-1.5`) and `Sidebar.tsx` icon-chip/logo spacing.
- Layout widths (`w-[220px]` Sidebar, `w-[280px]` ItemList, `w-12` icon-rail) are the spec's fixed, off-scale-by-design values — correctly not flagged.
- **Defect:** `CredentialBlock.tsx:72` (`p-3` = 12px, via `PaneSurface`) and `DetailPanel.tsx:150` (`mb-3` = 12px) are off-scale values that are neither on {4,8,16,24,32,48,64} nor among the spec's four named exception spots — an unauthorized fifth "12px" spacing value has crept in.

### Pillar 6: Experience Design (4/4)
- Loading: `ItemList.tsx:24` shows "Loading your sites…" while `sitesLoading && visibleSites.length === 0`; `DetailPanel.tsx:74-76` shows "Loading credentials…" while `credentialsLoading && credentials === null` — both backstop items from the UI Considerations table are implemented.
- Error: every API call site (`useVault.ts`, `DetailPanel.tsx`, `CredentialBlock.tsx`, `NewShell.tsx`) catches and falls back to the generic copy or a specific `CommandError.message`.
- Empty states: zero-sites, zero-search-results, and zero-credentials are all implemented per the contract.
- Zero-one-many credential repeater (D-08) is correctly implemented via `splitCredentials`/`otherLoginsLabel` in `src/newui/lib/credentialLayout.ts`, with per-credential local expand/collapse state (`Set<number>`) that unmounts children when collapsed (comment at `CredentialBlock.tsx:23` — "a collapsed credential's secret is never sitting in the DOM").
- Destructive confirmation: delete-site correctly gates on `window.confirm` via `confirmDialog`/`siteDeleteMessage` (`DetailPanel.tsx:48-51`); delete-credential correctly has NO confirmation (immediate soft-delete, matches spec's explicit call-out).
- Scope-cut invariants (D-09 lazy decryption, D-10 no favorites/folders/sync, D-12 single IPC seam) are enforced by an automated test (`src/newui/lib/parity.test.ts`) that scans the actual source files on every `npm test` run — this is a genuinely durable guardrail, not just a one-time check.
- Out-of-order response guards for both search (`useVault.ts:36-59`) and credential fetch (`useVault.ts:69-93`) exceed what the spec strictly required, preventing stale-data flashes on rapid interaction.

---

## Registry Safety

Not applicable — `components.json` is absent (project has no shadcn / component-library registry, per UI-SPEC's own Registry Safety table: "Tool: none").

---

## Files Audited

- `.planning/phases/08-new-vault-ui-figma-port/8-UI-SPEC.md`
- `.planning/phases/08-new-vault-ui-figma-port/8-CONTEXT.md`
- `.planning/phases/08-new-vault-ui-figma-port/08-{01..12}-SUMMARY.md` (scanned for line counts / plan-vs-built cross-check)
- `src/newui/NewApp.tsx`
- `src/newui/NewShell.tsx`
- `src/newui/hooks/useVault.ts`
- `src/newui/lib/vaultSelectors.ts`
- `src/newui/lib/credentialLayout.ts`
- `src/newui/lib/parity.test.ts`
- `src/newui/components/Sidebar.tsx`
- `src/newui/components/SidebarNavItem.tsx`
- `src/newui/components/ItemList.tsx`
- `src/newui/components/ItemRow.tsx`
- `src/newui/components/DetailPanel.tsx`
- `src/newui/components/CredentialBlock.tsx`
- `src/newui/components/DetailField.tsx`
- `src/newui/components/PaneSurface.tsx`
- `src/components/Tooltip.tsx`
- `src/App.css` (color token reference)
- `src/screens/Settings.tsx` (Interface banner copy cross-check)
- `src/screens/NavRail.tsx` (icon-rail width precedent cross-check)
