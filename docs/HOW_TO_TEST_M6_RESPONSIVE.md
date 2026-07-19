# How to Test — M6 Responsive Layout

Manual QA checklist for the M6 responsive-layout half (single 640px breakpoint, 3 selectable
narrow-nav patterns, fluid-width editor dialogs). Per this milestone's design decision, there is no
automated viewport/screenshot tooling — see `docs/DEFERRED.md`. This is the only verification step
in `docs/superpowers/plans/2026-07-19-m6-responsive-layout.md` (Task 8) that wasn't already covered
by `cargo test` / `npm test` / `npm run build`.

## Setup

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;C:\Strawberry\perl\bin;C:\Strawberry\c\bin;" + $env:PATH
npm run dev
```

Open `http://localhost:1420/` (main app) and `http://localhost:1420/viewer.html` (read-only Viewer)
in a browser. Use dev tools' device toolbar (or manual window resize) to test at roughly:

- **375px** — phone
- **640px** — the exact breakpoint
- **1024px** — desktop

## Checklist (for both `index.html` and `viewer.html`)

- [ ] At **≥640px**: sidebar (or Viewer's equivalent) looks and behaves exactly as before this
      milestone — no visual regression.
- [ ] At **<640px**, for each of the three `mobileNavStyle` Settings values in turn (switch via
      Settings, reachable through whichever narrow-nav is currently active):
  - [ ] **Icon rail** — collapsed `NavRail` shown, no toggle button (nothing to toggle).
  - [ ] **Bottom tab bar** — fixed strip at the bottom, all 6 destinations + Lock reachable.
  - [ ] **Hamburger drawer** — hamburger button in the header opens a slide-over panel; clicking a
        destination or Lock closes the drawer and navigates/locks correctly.
- [ ] `SiteEditor`/`CredentialEditor` dialogs stay fully on-screen with visible margins at
      320-375px, don't touch the viewport edges.
- [ ] `CsvImport`'s preview table scrolls horizontally at narrow widths instead of breaking the
      page layout.
- [ ] No screen produces a horizontal scrollbar on the `<body>` itself (individual scrollable
      elements like the CSV table are fine).

If this surfaces a genuine issue, fix it in a small, targeted commit on this branch rather than
leaving it.
