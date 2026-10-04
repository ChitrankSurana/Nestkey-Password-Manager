# How to Test — Main App

> **Status note (2026-09-30):** Written for the Classic UI before the New/Make UIs and Phase 9. For current UAT use `docs/testing/UAT_MODERN_UI_OVERHAUL.md` and `docs/testing/UAT_USER_TESTING_GUIDE.md`.

**Scope:** the full read/write desktop app (`nestkey-app`, crate `src-tauri`, launched via
`npm run tauri dev`). Covers every feature shipped through M1–M6 (web-mode half): auth, vault CRUD,
search, trash, generator, health dashboard, settings/theming, backup/restore, CSV import/export.

This doc has two parts: **automated tests** (run these first — they're fast and catch regressions
before you ever open a window) and a **manual walkthrough** (required, because nothing automated in
this repo can drive a live Tauri window — there's no Playwright, no `*.test.tsx`).

---

## Part 1 — Automated tests

1. Put Rust/Perl on PATH (fresh shells don't have it):
   ```powershell
   $env:PATH = "$env:USERPROFILE\.cargo\bin;C:\Strawberry\perl\bin;C:\Strawberry\c\bin;" + $env:PATH
   ```
2. Run the whole Rust workspace test suite from the repo root:
   ```powershell
   cargo test --workspace
   ```
   **Expected:** every crate compiles and every test passes (0 failures). This covers `nestkey-core`
   (crypto, vault, auth, session, generator, health, backup, portio/CSV), the `src-tauri` command
   layer, and the shared end-to-end test in `core/tests/end_to_end.rs`.
3. Run the frontend test suite:
   ```powershell
   npm test
   ```
   **Expected:** all tests pass (currently covers the clipboard auto-clear behavior).
4. Confirm the frontend builds cleanly:
   ```powershell
   npm run build
   ```
   **Expected:** clean `tsc` type-check, then a Vite production build producing `dist/index.html`
   and `dist/viewer.html`, 0 errors.

If any of the above fails, stop and fix it before doing the manual walkthrough below — a red
automated test means the manual walkthrough will very likely fail too, in a way that's slower to
diagnose by hand.

---

## Part 2 — Manual walkthrough

### Setup

1. Open a terminal at the repo root: `E:\Coding_Projects-CS\Account_1\Password_Manager`
2. PATH setup (same as above, if a fresh shell):
   ```powershell
   $env:PATH = "$env:USERPROFILE\.cargo\bin;C:\Strawberry\perl\bin;C:\Strawberry\c\bin;" + $env:PATH
   ```
3. Install deps if you haven't: `npm install`
4. Start the app: `npm run tauri dev`
5. Wait for the Tauri window to open. First launch creates `~/.pwmgr/` (registry + config). For a
   truly clean first-run test, close the app and delete `%USERPROFILE%\.pwmgr\` first — otherwise
   you'll land on whatever state a prior run left (including any test user from the Viewer App or
   Web Portal walkthroughs, since all three apps share the same `~/.pwmgr/` files).

Work through the sections in order — later steps build on state created earlier (a registered user,
an added site/credential).

### 1. Registration and generic-error login security

1. **Action:** On Register, enter a username (e.g. `alice`) and a master password (e.g.
   `correct horse battery staple`). Click **Register**.
   **Expected:** Lands on the (empty) Vault screen. No error.
2. **Action:** Click **Lock** to return to Login. Try `alice` with the **wrong** password, then try
   a **nonexistent** username (e.g. `nobody`).
   **Expected:** Both attempts show the exact same message — **"Incorrect master password."** — never
   a different message for "user doesn't exist" vs. "wrong password."
3. **Action:** Log in with the correct username/password.
   **Expected:** Returns to the (still-empty) Vault screen.

### 2. Adding a site and credential, then live search

1. **Action:** Click **Add site**. Fill in a name (e.g. `GitHub`), URL (optional), notes (optional).
   Click **Save**.
   **Expected:** New site appears in the Vault list immediately.
2. **Action:** Open the site, click **Add credential**. Fill in username, password (or use the
   generator — see Part 2 §5), remarks. Click **Save**.
   **Expected:** Credential appears under that site immediately.
3. **Action:** Go back to Vault, type into the search box — try the site name, then a credential's
   username, then a word from its remarks.
   **Expected:** Results update live, within one keystroke, for all three search types — no Enter
   key, no search button.

### 3. Editing a credential

1. **Action:** Open the site → the credential → **Edit** → change the username or remarks → **Save**.
   **Expected:** Change reflected immediately in the credential list.

### 4. Delete → Trash → Restore → Empty Trash

1. **Action:** Open the credential, click **Delete**.
   **Expected:** Disappears from the site's credential list immediately.
2. **Action:** From Vault, open **Trash** (nav rail).
   **Expected:** The deleted credential appears, labeled with its site's name.
3. **Action:** Click **Restore**.
   **Expected:** Reappears under the site; no longer in Trash.
4. **Action:** Delete it again, go to Trash, click **Empty Trash**, confirm the dialog.
   **Expected:** Trash is empty; the credential is gone for good (this is the one destructive,
   unconfirmed-at-the-backend action in the app — the confirm dialog is the only safety net).

### 5. Password generator (all four methods)

Open the credential editor (or the standalone **Generator** screen from the nav rail) for each:

1. **Random:** set a length, toggle uppercase/lowercase/digits/symbols off one at a time, click
   **Generate** each time.
   **Expected:** Output respects every toggle (e.g. all-digits-off produces no digits); strength
   meter updates.
2. **Memorable:** set word count and separator, **Generate**.
   **Expected:** Output is that many dictionary words joined by the separator.
3. **Pattern:** try the default template `Cvcv-####`, then a custom one.
   **Expected:** Output matches the template's placeholders (`C`/`c` letter, `V`/`v` vowel, `#`
   digit, `?` symbol, `\` literal).
4. **Pronounceable:** set a length, **Generate**.
   **Expected:** Output is pronounceable-syllable-based, matching the requested length.
5. **Action:** Click **Copy** on any generated password.
   **Expected:** Copied to clipboard; per the tooltip, it auto-clears after 30 seconds (wait and try
   pasting again to confirm).
6. **Action:** In Settings, switch "Password strength algorithm" between Simple and Advanced
   (zxcvbn), then return to the generator/editor and type/generate a password.
   **Expected:** The meter's label and score change algorithm accordingly.

### 6. Health Dashboard

1. **Action:** Add a few credentials designed to trigger each category: a short/simple password
   (weak), the same password reused across two credentials (reused), and (if you can backdate it or
   just accept it won't trigger immediately) note the "old" threshold is configurable in Settings.
2. **Action:** Open **Health** from the nav rail.
   **Expected:** Weak, Reused, and Old cards show counts; click each to expand/collapse its entry
   list; clicking an entry navigates to its site.
3. **Action:** Note the caption: *"Weak" is always scored with the advanced algorithm, regardless of
   your strength-algorithm setting in Settings.*
   **Expected:** Confirm this holds — even with Settings set to "Simple," the Weak card's judgment
   matches zxcvbn's opinion, not the simple entropy count. (See `docs/KNOWN_LIMITATIONS.md`.)
4. **Action:** In Settings, enable "Enable breach check (HIBP)", return to Health, click **Check
   now** under Breached.
   **Expected:** Requires network access (this is the one feature that talks to the internet); shows
   a breached-entries list if any of your test passwords are known-breached (try a very common one
   like `password123` to get a hit).
5. **Action:** With HIBP left disabled in Settings, revisit Health.
   **Expected:** The Breached card shows "Enable 'Breach check (HIBP)' in Settings first" instead of
   a **Check now** button — no network call happens.

### 7. Settings — every control, and that auto-lock takes live effect

1. **Action:** Change **Auto-lock** to 1 minute.
   **Expected:** Dropdown reflects it immediately; briefly disabled while saving, then re-enabled.
2. **Action:** Return to Vault. **Don't touch mouse/keyboard for over a minute.**
   **Expected:** App automatically returns to Login on its own — proves the live session timeout
   reconfigure actually took effect on the running session, not just on next login.
3. **Action:** Log back in, confirm Settings still shows 1 minute (persisted correctly), then set it
   back to 5 or 15 minutes.
4. **Action:** Cycle through both **Theme** options (dark/light) and all five **Accent color**
   options.
   **Expected:** Applies instantly across every screen, persists across a restart.
5. **Action:** Change "Consider a password 'old' after" between 90/180/365 days.
   **Expected:** Saves; affects the Health Dashboard's Old card threshold.
6. **Action:** Toggle "Enable breach check (HIBP)" on and off.
   **Expected:** Immediately changes whether Health Dashboard's Breached card shows a Check button.

### 8. Manual Lock button

1. **Action:** Click **Lock** in the nav rail.
   **Expected:** Returns to Login immediately, no delay, no confirmation dialog.

### 9. Backup and restore

1. **Action:** Go to **Data → Backup & Restore**, click **Create Backup**.
   **Expected:** Status message shows the path the encrypted `.bak` file was written to (under
   `~/.pwmgr/backups`).
2. **Action:** Click **Restore Backup**, pick a `.bak` file.
   **Expected:** Success message tells you to log in with that account's master password; log out
   and back in to confirm the restored data is present.

### 10. CSV export (all three modes)

1. **Action:** Go to **Data → Export CSV**, choose **Encrypted (this device)**, export.
   **Expected:** File written; only openable by this app on this device.
2. **Action:** Choose **Encrypted (with a password)**, enter and confirm an export password, export.
   **Expected:** File written; portable — this password (not your master password) is needed to
   re-import it.
3. **Action:** Choose **Plaintext**, note the red warning, try clicking **Export** before typing the
   confirmation phrase.
   **Expected:** Export button stays disabled until you type `EXPORT PLAINTEXT` exactly.
4. **Action:** Type the phrase correctly and export.
   **Expected:** Export succeeds; open the CSV in a text editor to confirm it's genuinely plaintext
   (this is the one intentionally-unencrypted artifact this app produces — treat the resulting file
   accordingly).

### 11. CSV import (all three sources)

1. **Action:** Go to **Data → Import CSV**. Try **Chrome/Chromium export** and **Firefox export**
   with a real exported CSV from each browser (or a hand-made file matching their column layout),
   **Preview**, review the parsed table, edit a cell inline if needed, **Commit Import**.
   **Expected:** Preview shows parsed rows before anything is saved; commit reports
   imported/duplicate counts; duplicates are skipped, not double-inserted.
2. **Action:** Try **Other (map columns myself)** with an arbitrary CSV — pick a file, map each
   detected column to a target field (or "(ignore)"), **Preview**, **Commit Import**.
   **Expected:** Only mapped columns populate their target field; unmapped ones are ignored; same
   preview/commit flow as the named formats.

### 12. Dark theme consistency

1. **Action:** Click through every screen: Vault, Add Site, a site's detail view, Add/Edit
   Credential, Trash, Settings, Data (and its three sub-screens), Generator, Health Dashboard.
   **Expected:** Every screen renders in the selected theme throughout — no flash of unstyled
   content, no screen or modal stuck on the wrong background.

### 13. Keyboard-accessible tooltips

1. **Action:** On each screen, use **Tab** (not the mouse) through every button and the Settings
   `<select>`/checkbox controls.
   **Expected:** A small dark tooltip appears above the focused control, describing its action;
   disappears on Tab-away.
2. **Action:** Hover the same controls with the mouse (no click).
   **Expected:** Same tooltip on hover, disappears on mouse-leave.
3. **Action:** Confirm no control anywhere still shows the browser's native yellow/gray tooltip after
   a ~1s hover delay.
   **Expected:** Every tooltip is the custom dark styled one, appearing instantly.
4. **Best-effort, not a hard requirement:** try to catch a **disabled** control (e.g. Settings'
   `<select>` mid-save) with a hover/focus and see if its tooltip still shows. The timing window is
   narrow — if you can't catch it, that's expected and not a failure (see
   `docs/KNOWN_LIMITATIONS.md` §1).

---

## If something doesn't match

Note exactly: which section/step, which screen, what you did, what you expected, what actually
happened (screenshot if easy). That's a real, reportable finding.

## If everything matches

The Main App's manual walkthrough is complete. Continue to
[`HOW_TO_TEST_VIEWER_APP.md`](HOW_TO_TEST_VIEWER_APP.md) and
[`HOW_TO_TEST_WEB_PORTAL.md`](HOW_TO_TEST_WEB_PORTAL.md) — both reuse the same `~/.pwmgr/` data, so
the user/site/credential you created here is exactly what you'll use to test read-only access next.
