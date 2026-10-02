# UI/UX Implementation Plan: Porting Bitwarden's Interaction Model

Plan only — no code yet. Ports Bitwarden's UI *patterns* into this app's existing React 19 +
Tailwind v4 + Tauri stack, not a literal clone of their Angular component library. Skips anything
that only makes sense at Bitwarden's scale (org/product switcher, breadcrumb drilldown, bulk
actions, icon font, virtualized table) — flagged per phase as "skip for now."

Builds on gaps identified in `UI_Comparison_Study.md`. No new dependencies required — everything
below is achievable with existing React/Tailwind/Tauri APIs.

---

## Phase 1 — Persistent app shell

**Why first:** every other gap (always-visible search, list-stays-mounted, deep-linkable detail)
is downstream of the nav rail unmounting today.

1. Lift `NavRail`/`BottomTabBar`/`NavDrawer` out of `Vault.tsx` into a new `AppShell` component that
   wraps the *whole* post-login app (replace `App.tsx`'s bare `<Vault>` render with `<AppShell><Vault/></AppShell>` equivalent), so nav survives Settings/HealthDashboard/Generator/Data screens too.
2. Keep the existing `Screen`/`NavDestination` state machine — just move *where* it's read from
   (shell reads it, not `Vault.tsx` alone) so `activeKey` is no longer hardcoded `"vault"`.
3. No router library needed. The existing boolean-flag approach is fine at this app's screen count
   (12 screens); a router adds history/deep-link support this desktop app doesn't need yet.
   *Skip: React Router / TanStack Router — add only if URL deep-linking is explicitly wanted later.*

## Phase 2 — List+detail split (kill the full-screen swap)

1. Change `Vault.tsx`'s `selectedSiteId` handling from "early-return replaces the whole screen"
   to "list stays mounted, detail renders as a modal/side-panel over it" — same shape as the
   existing `SiteEditor`/`CredentialEditor` modals, just for *viewing*, not only editing.
2. This single change is what unlocks field-level copy/reveal on `SiteDetail`/credential rows
   without a bigger rewrite, since the list's search/scroll state no longer gets thrown away.
3. *Skip: virtualized list / `cdk-virtual-scroll` equivalent* — irrelevant until vault size is in
   the thousands; a plain mapped `<ul>`/`<div>` list is fine at personal-vault scale.

## Phase 3 — Field-level copy/reveal + toast feedback

1. Add a small `CopyButton` component (icon button + `navigator.clipboard`/existing
   `clipboard.ts` helper + fires a toast) — reuse the pattern `ViewerVault.tsx`'s accordion
   already proved out, just extract it so `SiteDetail`/`CredentialEditor` can use it too.
2. Add a `RevealToggle` for password fields (swap masked `<input>`/text for plain text on click) —
   again, `ViewerVault.tsx` already has this logic; extract, don't reinvent.
3. Add one tiny `Toast` component (fixed-position, auto-dismiss) + a `useToast()` hook (context
   or simple event-emitter) so copy/save/delete actions get a confirmation instead of silence.
   *Skip: Bitwarden's colorized password rendering (`bit-color-password`)* — nice-to-have, not
   structural; add later if wanted, doesn't block anything else.
4. Give `SiteEditor` edit + delete (currently add-only) — small gap, same modal, just wire the
   existing `existing: SiteDto | null` pattern `CredentialEditor` already uses.

## Phase 4 — Design tokens (finish what's half-started)

1. Extend `App.css`'s token block: keep the 5 accent hues, add semantic tokens for
   `--fg`/`--bg`/`--border` (2-3 tokens each is enough — don't build Bitwarden's full 050-950 ramp
   for a single-user app with one active theme at a time).
2. Actually implement the light-mode path (currently `dark:` utilities with no light counterpart
   — toggling `theme=light` in Settings today produces unstyled defaults). Either finish light
   mode properly or remove the toggle until it's real — a broken option is worse than no option.
3. Adopt Bitwarden's compact type-scale *idea* (few heading sizes, most text at sm/base) if the
   current UI feels inconsistent — but this is a polish pass, not a structural one; can slot in
   anytime after Phase 1-3, no ordering dependency.
   *Skip: icon font, container-query breakpoints* — emoji icons and viewport-based Tailwind
   breakpoints are proportionate for this app's size.

## Phase 5 — Settings reorganization

1. Split the current single flat `Settings.tsx` form into sections matching what Bitwarden
   separates — Security (auto-lock, HIBP opt-in, old-password threshold), Appearance (theme,
   accent, mobile nav style), Vault (strength algorithm) — as **collapsible sections on one
   screen**, not separate routes/pages. A single-user local app doesn't need Account/Subscription/
   Domain-rules/Emergency-access pages at all.
   *Skip: Settings as nav-group with routed sub-pages* — only worth it if Settings grows enough
   to need its own scroll position per section; a sectioned single page is proportionate now.

## Phase 6 — Site→Credential model decision (make it deliberate)

This isn't UI work — it's a data-model fork Bitwarden doesn't have (they're flat). Before further
UI polish on `SiteDetail`, explicitly decide, and note the decision in this file's follow-up:
- **Keep grouping** (Site as a folder-like parent for Credentials) — closest to current schema,
  cheapest, but diverges from every Bitwarden UI reference in this study.
- **Flatten to match Bitwarden** (each login is a standalone item, "site" becomes just the URL
  field) — bigger migration (`core/src/vault.rs` schema change), but makes the rest of this plan's
  patterns (flat filterable list, per-item modal) a closer match.

Recommendation: **keep grouping** — it's a deliberate, reasonable divergence (site-centric
organization is arguably more useful for a personal vault with multiple logins per service), not a
gap to close. Revisit only if it causes real friction after Phases 1-3 ship.

---

## Suggested order

Phase 1 → Phase 2 → Phase 3 → Phase 4/5 (either order, independent) → Phase 6 (decision only,
no code forced by it unless flattening is chosen).

Nothing here requires a new npm/cargo dependency — every gap closes with existing React/Tauri/
Tailwind primitives.

---

## Phase 6 follow-up: decision recorded

Implemented Phases 1-5 (2026-08-01). Phase 6 was a decision, not code: **keep the two-level
Site→Credential grouping**, per this file's own recommendation. No friction observed after
shipping Phases 1-3 (persistent shell, list+detail split, field-level copy/reveal/toast, site
edit/delete) — the grouping continued to fit the personal-vault use case cleanly. Not revisited.
