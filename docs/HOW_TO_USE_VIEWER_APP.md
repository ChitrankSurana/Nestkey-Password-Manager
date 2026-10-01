# How to Use — Viewer App

> **Status note (2026-09-30):** The Viewer was restyled onto the Make UI's visual language after this guide was written (commit `f417284`), so screenshots/descriptions of the old Classic look no longer match. It remains read-only (master-password login; view, search, copy). Full feature reference for the main app: `docs/HOW_TO_USE_APP.md`.

The Viewer App is a **second, smaller desktop app** for one specific situation: you need to look up
a password, but the Main App is unavailable — broken, mid-update, or just not installed on the
machine you're at. It opens the exact same encrypted vault, but can only ever read from it: this
build of the app has no add/edit/delete commands compiled into it at all, so there's no code path
that could change your data even if something went wrong.

## When to use this instead of the Main App

- The Main App won't launch or is being reinstalled/updated.
- You just want to glance up a password without risking an accidental edit.
- You want a second, purpose-built "just let me see my stuff" tool as a fallback.

For everyday use — adding new sites, generating passwords, running the health check, backups — use
the [Main App](HOW_TO_USE_MAIN_APP.md) instead. This app can't do any of that.

## Launching it

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;C:\Strawberry\perl\bin;C:\Strawberry\c\bin;" + $env:PATH
cd viewer-src-tauri
npx tauri dev
```

A window titled **"Password Manager — Viewer"** opens.

## Logging in

Same login screen, same credentials as the Main App — it's opening the identical
`~/.pwmgr/registry.db` and `~/.pwmgr/vaults/<user>.vault` files, so there is nothing separate to set
up. If you've never registered a user, you'll need to do that once in the Main App first — the
Viewer has no Register screen (registering counts as "adding data," which this build deliberately
can't do).

## What you can do

- **Browse every site** you've saved, exactly as in the Main App.
- **Search** in real time by site name, username, or remarks — the same instant, as-you-type search.
- **Expand a site** to see its credentials.
- **Reveal a password** (👁 button) or hide it again (🙈), one at a time, per credential — nothing is
  shown in the clear until you click to reveal it.
- **Copy a password** to your clipboard — it auto-clears after 30 seconds, same as the Main App.
- **Lock** the vault manually, or let it **auto-lock** after the same idle timeout you configured in
  the Main App's Settings (this app shares that same setting; it just has no Settings screen of its
  own to change it from).

## What you cannot do

Nothing else — there is no Add, Edit, Delete, Settings, Trash, Generator, Health Dashboard, Backup,
or CSV import/export screen in this app. If you need any of those, switch to the Main App. This
isn't a restricted view of a bigger app; it's a genuinely smaller program with less code in it.

## Theming

The Viewer reflects whatever Theme and Accent color you last set in the Main App's Settings (same
`config.json`), so it looks consistent even though it has no theme picker of its own.

## Related guides

- Full read/write daily use → [`HOW_TO_USE_MAIN_APP.md`](HOW_TO_USE_MAIN_APP.md)
- Browser access on this machine → [`HOW_TO_USE_WEB_PORTAL.md`](HOW_TO_USE_WEB_PORTAL.md)
- Step-by-step verification of this app → [`testing/HOW_TO_TEST_VIEWER_APP.md`](testing/HOW_TO_TEST_VIEWER_APP.md)
