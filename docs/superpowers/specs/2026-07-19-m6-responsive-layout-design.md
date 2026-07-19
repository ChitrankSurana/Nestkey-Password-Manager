# M6 — Responsive Layout Design

**Scope note:** M6 per `docs/PRD.md` §7 is one roadmap line ("Localhost-bound web mode, responsive
layout") covering two independent pieces of work. The web-mode half is done and merged
(`docs/superpowers/specs/2026-07-18-m6-web-mode-design.md`). **This spec covers only the
responsive-layout half.**

## 1. Problem

The app's UI (both the main read-write Vault app and the read-only Viewer from M5) was built
desktop-only, inside a fixed Tauri window. Now that web mode (M6 part 1) serves the same React
bundle to an ordinary browser tab, the layout needs to hold up when that tab is narrower than a
desktop window — resized side-by-side with another app, a smaller laptop screen, a tablet browser,
or a phone-width viewport. Today nothing in the codebase has a responsive breakpoint: the sidebar
is a fixed 48px/160px rail, dialogs are a fixed 320px, and no screen has been checked below desktop
width.

## 2. Scope

- **Both frontend entry points**: the main Vault app (`index.html`) and the read-only Viewer
  (`viewer.html`). The Viewer has no sidebar, so its scope is purely content-layout; the main app's
  scope includes navigation.
- **Target range**: phone width (~320-375px) through tablet through desktop. Note: web mode is
  bound to `127.0.0.1` only, so a phone cannot reach it today — this is deliberate future-proofing,
  not solving an immediate access problem. (A separate, later roadmap item covers an actual Android
  companion app.)
- **Out of scope**: any change to the desktop Tauri window experience at typical desktop widths
  (≥640px) — that layout is unchanged by this spec. Automated viewport/screenshot testing is
  explicitly deferred (see `docs/DEFERRED.md`).

## 3. Breakpoint

Single breakpoint at **640px**, matching Tailwind v4's built-in `sm` breakpoint (no custom value,
no new breakpoint system). `≥640px` = today's layout, byte-for-byte. `<640px` = the narrow-layout
rules in this spec apply. This was chosen over Tailwind's `md` (768px) so tablets in landscape and
most laptop windows keep the existing sidebar; only genuinely narrow windows (phones, a browser tab
tiled to a small fraction of the screen) get the narrow treatment.

Both `index.html` and `viewer.html` already carry `<meta name="viewport" content="width=device-width,
initial-scale=1.0">` — no change needed there.

## 4. Navigation (main app only)

The main app's `NavRail` already has two states: collapsed (48px, icon-only) and expanded (160px,
icon+label), toggled manually and persisted via `Config.nav_collapsed`. That toggle and both states
are **unchanged at ≥640px**.

Below 640px, the sidebar is replaced by one of three interchangeable narrow-nav patterns, all built:

1. **Icon rail** (default) — reuses the existing collapsed `NavRail` component as-is, forced into
   its collapsed visual state regardless of the desktop toggle's saved value. No new component.
2. **Bottom tab bar** — the sidebar is hidden; a new fixed bottom bar renders the same destinations
   as icon buttons in a row. Follows the app's existing dark-mode (`dark:` classes) and accent-color
   (`var(--accent)` for the active item) conventions already established in `NavRail.tsx` — no new
   color or spacing system introduced for this component.
3. **Hamburger + slide-over drawer** — the sidebar is hidden; a new top bar with a ☰ button opens a
   full-height overlay drawer showing the same icon+label list as today's expanded rail (visually,
   literally the same list markup, just presented as an overlay). Same styling conventions as above.

**Selectable via Settings** (new entry, e.g. "Mobile nav style"), persisted through the same
mechanism every other Settings value already uses:
- New `Config` field (validated against `["icon-rail", "bottom-tabs", "drawer"]`, mirroring the
  existing `VALID_*` const pattern in `core/src/config.rs`)
- New `do_update_mobile_nav_style` command in `pwmgr-core`, exposed as both a Tauri command
  (`src-tauri`) and an HTTP route (`web-src-tauri`) — identical shape to `do_update_nav_collapsed`
- New `ConfigDto` field (camelCase: `mobileNavStyle`)
- New picker control in the `Settings` screen (radio group or select, next to the existing
  theme/accent-color/nav-collapsed controls)
- Default value for all existing and new users: `icon-rail`

This setting has no effect at ≥640px — it only governs what replaces the sidebar below the
breakpoint. The Lock button (currently the last item in `NavRail`) is included in all three
narrow-nav patterns as one more icon alongside the six destinations.

## 5. Modals & dialogs

Every editor dialog (`SiteEditor`, `CredentialEditor`, and any other centered-overlay form) keeps
today's pattern at every width: a dim backdrop with a centered card. The only change is the card's
width below 640px — fluid with side margins (e.g. `w-full max-w-80 mx-4` in place of today's fixed
`w-80`) instead of a hard-coded pixel width, so it can never overflow or touch the viewport edges.
No new "full-screen sheet" concept is introduced.

## 6. Content screens (general responsive sweep)

The remaining screens — `Vault`, `SiteDetail`, `HealthDashboard`, `Generator`, `Settings`, `Trash`,
`Data`/`Backup`/`CsvExport`/`CsvImport`, `Login`/`Register`, and the Viewer's `ViewerVault` — get a
straightforward pass below 640px: replace fixed-width or side-by-side (`flex-row`) assumptions with
fluid or stacking (`flex-col`) layouts, and check interactive-element sizing is reasonable for touch.

One structural risk was checked and found already safe: `CsvImport`'s preview `<table>` is already
wrapped in `overflow-x-auto`, so it already degrades to horizontal scroll rather than breaking the
layout at narrow widths — this needs verification during implementation, not a redesign. No other
screen has a grid/table/fixed-width pattern that needs its own design decision beyond the general
sweep above.

## 7. Testing

Manual QA only, via the standard deferred walkthrough used for every prior milestone's visual
changes (M4, M5, M6 web-mode) — resize the browser window through a few representative widths
(phone/tablet/desktop) once implementation is complete, as part of the manual walkthrough already
deferred until both M6 halves are done. No new test tooling is introduced. Automated
viewport/screenshot testing (e.g. Playwright) was considered and explicitly deferred — tracked in
`docs/DEFERRED.md` with the reasoning and a revisit trigger.

## 8. Non-goals

- No visual redesign, rebrand, or new color/typography system. Every new element (bottom tab bar,
  drawer) must read as belonging to the app's existing design language, not a bolted-on new style.
- No change to the desktop experience at ≥640px.
- No mobile-specific features beyond layout (no swipe gestures, no PWA/installability, no offline
  support).
- No change to `pwmgr-viewer` (the M5 Tauri desktop viewer binary) or `pwmgr-web`'s HTTP surface
  beyond the one new `mobile_nav_style` setting's plumbing.
