# UI/UX Comparative Study: Current App vs. Bitwarden

Compares this app's actual frontend structure against Bitwarden's client UI/UX architecture (not
feature list — see `GUI_Feature_Plan.md`/`GUI_Feature_Comparison.md` for that). Sources: direct
read of `src/` in this repo, and Bitwarden's `clients`/`libs/components` monorepo + public docs.

---

## Side-by-Side

| Dimension | This app (now) | Bitwarden |
|---|---|---|
| Routing | No router lib; nested `useState` screen flags with early-return chain (`App.tsx`→`Vault.tsx`→`Data.tsx`) | Angular router, every section a real route, deep-linkable (`?itemId=…&action=edit`) |
| Persistent shell | **None** — nav rail (`NavRail.tsx`) renders only inside `Vault.tsx`; every other screen is a standalone full-page div, nav vanishes | Persistent left rail (`app-side-nav`) wraps a `router-outlet`; rail never unmounts across the whole app |
| Top-level sections | Vault, Settings, Data(hub: Backup/Export/Import), HealthDashboard, Generator, Trash — reached via ad hoc boolean flags | Vaults, Send, Tools(group: generator/import/export), Reports, Settings(group: account/security/appearance/domain-rules/emergency-access) — real nav items/groups |
| List layout | Plain `<ul>` of one-line rows, no columns, no icons/metadata | Virtualized `bit-table` (or card-list `bit-item` on newer surfaces) with columns, bulk-select checkbox, actions column |
| Filter/search placement | One search box inside the vault screen, replaces the list with a second list variant when active | Dedicated filter sidebar, **always visible**, search pinned at its top, filters by type/folder/collection/trash |
| Item hierarchy | **Two-level: Site → Credentials** (a site groups multiple logins) | **Flat**: every item (login/card/identity/note/SSH key) is a peer in one list |
| Opening an item | Full-screen replace (list unmounts entirely) | Modal dialog over the list (list stays mounted, context preserved), deep-linkable |
| Editing | Modal (`SiteEditor`, `CredentialEditor`) — matches Bitwarden's modal instinct, but SiteEditor is **add-only**, no edit/delete for sites | Single shared modal form (`cipher-form`), stacked sections by type, `add`/`edit`/`partial-edit`/`clone` modes |
| Field-level actions | **Missing on view.** `SiteDetail` shows credential rows as text + Edit/Delete links only — no copy, no reveal, no visible password without opening the editor. (Reveal+copy *does* exist, but only in the separate read-only `ViewerVault` accordion.) | Every sensitive field has its own copy button (suffix icon) + its own reveal toggle (swaps to colorized password display) + its own aria-label, always visible on the view |
| Design tokens | 5 accent hues only (`--accent`/`--accent-hover`); no type scale, no spacing scale, dark-only in practice (light mode unstyled) | Full 050–950 ramp per hue + semantic tokens (`--color-fg-*`, `--color-bg-*`, `--color-border-*`) + 8-step type scale, real light/dark token swap |
| Icons | Emoji string literals | Custom icon font (`bwi-*`), consistent per-action icon (copy=`bwi-clone`, etc.) |
| Feedback/loading | Errors as a red `<p>` per screen; no toast, no skeleton, no shared spinner | Toast on every copy/save/delete, skeleton + spinner for loading, illustrated empty-states (icon + title + description + CTA) |
| Bulk actions | None | Header checkbox → floating bulk-actions bar (move/assign/archive/delete), permission-gated |
| Settings organization | One flat screen (`Settings.tsx`): auto-lock, strength algorithm, theme, accent, mobile nav style, HIBP opt-in, old-password days | Nav-group with routed sub-pages: Account / Security (incl. reports) / Appearance / Domain-rules / Emergency-access |
| Account handling | One `username` prop threaded top-down; multi-user exists at the backend (`registry.rs`) but no in-app switcher | Product switcher + org switcher + account switcher (desktop, up to 5 accounts), all in persistent chrome |

---

## What actually functions well already (keep as-is)

- **Modal-for-editing instinct already matches Bitwarden** — don't rebuild this as a route, just extend it (add edit/delete to `SiteEditor`, add field-level copy/reveal to the view).
- **Local-first search** (undebounced, direct SQLite IPC) is a legitimate simplification given no browser-extension/remote-search constraint Bitwarden has — no reason to add debounce or a filter sidebar's complexity just to look the part.
- **`Tooltip.tsx` + emoji icons** are a fine, zero-dependency stand-in for `bwi-*` — don't add an icon font just for parity.

## What's structurally missing (the real gaps)

1. **No persistent shell.** This is the single biggest divergence — everything else in Bitwarden's UX (search-always-visible, list-stays-mounted, deep context) flows from the rail never unmounting.
2. **List+detail split view** doesn't exist; every navigation is a full replace. This is *why* item view has no field-level copy/reveal today — there's no "view" state that isn't also "I left the list."
3. **No design-token system beyond accent color** — no type scale, no semantic fg/bg/border tokens, no working light mode.
4. **No toast/feedback layer** — copy-to-clipboard and saves happen silently or via a full-page error string.
5. **Site→Credential two-level model** is a real data-model divergence, not just a UI one — worth a deliberate decision (keep grouping vs. flatten to match Bitwarden), not an accidental side effect of history.

These five map directly to the phases in `UI_Implementation_Plan.md`.
