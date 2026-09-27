# UAT — Your Independent D + W Run (User Testing Guide)

This is your self-contained guide for Process step 3 of `docs/Plans/UAT_Modern_UI_Overhaul_Plan_2026-09-23.md`:
run the same script Claude already ran, independently, on both Desktop (D) and Web (W), and record
your own results. It expands the frozen script (`docs/testing/UAT_MODERN_UI_OVERHAUL.md`) into
step-by-step instructions with explicit expected outcomes, so you don't need to cross-reference
anything else while testing.

Your results go in the **User result** column of `docs/testing/UAT_FINDINGS_2026-09-23.md` — see
Section 5. After you're done, Claude runs joint review (plan Process step 4): fix now / defer /
accept, per finding.

---

## 1. What's being tested (phase by phase)

| Phase | What it covers |
|---|---|
| **M1** | Interface switching (Classic/New/Make), launch behavior, missing-asset fallback |
| **M2** | Login/Register (shared across all interfaces) |
| **M3** | Adding/editing all 4 item types (Login, Secure Note, Payment Card, Identity), custom fields, TOTP, multi-credential sites |
| **M4** | Favorites and folders (create/rename/delete/assign) |
| **M5** | Search — what fields are searchable, what isn't |
| **M6** | Security gates — reprompt-before-reveal, clipboard auto-clear, password masking, TOTP |
| **M7** | Password history (cap at 5, newest-first) |
| **M8** | Trash — delete/restore, cascade delete, empty trash |
| **M9** | Master password change, PIN reset side-effect, backup-before-change |
| **M10** | Data Hub — encrypted backup/restore, CSV export (3 modes), CSV import |
| **M11** | Make UI's restyled screens — theme/accent, Health Dashboard, Generator, settings persistence |
| **M12** | Responsive layout — narrow (<768px) vs wide breakpoint |
| **M13** | Regression check — Classic/New/Viewer still work after all the above |
| **D1** | Windows Hello biometric unlock (Desktop only) |
| **D2** | SSH agent (Desktop only) — **see crash warning in Section 2** |
| **D3** | Window title (cosmetic, Desktop only) |

Run **all of it twice** — once as D (desktop app, `npm run tauri:dev`), once as W (web mode,
`pwmgr-web` + browser). D-only cases (D1–D3) obviously skip on W. Wipe `~/.pwmgr` between the two
runs (Section 2, step 0.6) so each starts from a clean vault.

---

## 2. How to test — setup

### 2.1 One-time environment check
1. Open PowerShell at `E:\Coding_Projects-CS\Account_1\Password_Manager`.
2. Run:
   ```powershell
   $env:PATH = "$env:USERPROFILE\.cargo\bin;C:\Strawberry\perl\bin;C:\Strawberry\c\bin;" + $env:PATH
   ```
3. Run `git branch --show-current` — must print `feature/modern-ui-overhaul`.
4. Run `git status --short` — must be empty (clean tree).

### 2.2 Before every run (D and W each get this)
1. Close any running `pwmgr-app.exe` or `pwmgr-web` process.
2. Wipe test data:
   ```powershell
   Remove-Item -Recurse -Force "$env:USERPROFILE\.pwmgr"
   ```
   Confirm the folder is gone.
3. Create fixture files:
   ```powershell
   New-Item -ItemType Directory -Force "$env:TEMP\uat" | Out-Null
   ssh-keygen -t ed25519 -N '""' -f "$env:TEMP\uat\uat_key" -C uat   # D only, for D2
   ```
   Then create `%TEMP%\uat\chrome.csv` with **exactly** this content (header must match Chrome's
   export format — 4 columns, no extra `note` column):
   ```csv
   name,url,username,password
   ImportA,https://a.example,alice,Imp0rt!A1
   ImportB,https://b.example,bob,Imp0rt!B2
   ImportA,https://a.example,alice,Imp0rt!A1
   ```
   (Third row is a deliberate duplicate of the first, to test import dedupe.)

### 2.3 Standard test data (use these exact values throughout)

| Key | Value |
|---|---|
| Username | `uatuser` |
| Master password v1 | `Uat-Master-Pass-1!` |
| Master password v2 | `Uat-Master-Pass-2!` |
| Login item "GitHub" | URL `https://github.com`, username `octo@uat.test`, password `Gh!Pass-0001`, remarks `work acct`, TOTP secret `JBSWY3DPEHPK3PXP` |
| GitHub custom fields | `Recovery code` (Hidden) = `RC-ALPHA-7788`; `Team` (Text) = `platform`; `2FA on` (Boolean) = Yes |
| Insecure login "OldForum" | URL `http://oldforum.test`, username `u1`, password `password` |
| Reused logins | `ShopA` password `Reuse-Me-123`; `ShopB` password `Reuse-Me-123` |
| Secure Note "Wifi" | body `SSID home / key wifi-secret-99` |
| Payment Card "Visa Main" | brand `Visa`, number `4111111111111111`, expiry `12/29`, CVV `123` |
| Identity "Me" | full name `Uat Tester` |
| Folders | `Work`, `Personal` |

### 2.4 Launching

**Desktop (D):**
```powershell
npm run tauri:dev
```
Wait for the window. It must show a title bar and render content within ~30s (no blank white window).

**Web (W):**
```powershell
npm run build
cargo run -p pwmgr-web
```
Wait for console output `pwmgr-web listening on http://127.0.0.1:7373`, then open
`http://127.0.0.1:7373/makeui.html` in any Chromium browser. Open DevTools (F12) → Console tab and
leave it open — **any red error during a case is a finding, write it down even if the case
otherwise passes.**

Don't run D and W at the same time — they share `~/.pwmgr`. Finish one mode fully (wipe, retest)
before starting the other. Either order (D-then-W or W-then-D) is fine for your independent run.

### 2.5 Known issues — don't be alarmed, but do confirm

These bugs are already recorded from Claude's pass. If you hit the same symptom, a one-line
confirmation in your Observation column ("reproduced, same as Claude's finding") is enough — you
don't need to re-investigate. If you see something **different** from the description below,
that's a new finding and needs full detail.

- **Make UI's item list can go stale** after Restore Backup forces a relogin — it may show 0 items
  or a stale count until you add a new item. Don't conclude data loss; cross-check with Classic UI
  first if worried (see `M10-makeui-stale-list` in the findings doc).
- **SSH Agent toggle (D2) has a confirmed crash.** Enabling "Enable SSH agent" in Settings will
  crash the app. Root cause is now known (a `tokio::spawn` panic — see the D2 row in
  `docs/testing/UAT_FINDINGS_2026-09-23.md`) but **not fixed yet**. If it crashes and then
  crash-loops on every relaunch: edit `%USERPROFILE%\.pwmgr\config.json`, set
  `"ssh_agent_enabled": false`, save, relaunch. **Recommendation: skip actually enabling the
  toggle in your run** (mark D2 as "Fail — confirmed crash, see D2 finding" without repeating the
  crash) unless you specifically want to re-verify it yourself.
- **Clicking Lock in Make UI (web) or the Viewer app doesn't tear down the UI** — a stale,
  still-interactive screen with revealed secrets can persist until a hard reload. Known
  (`M3.6-lockbug`, `M13.3-viewer-lockbug`).
- **The Edit-credential modal can be unreachable at short viewport heights** (below ~900px) — its
  Save/Cancel row scrolls off with no scrollbar. Maximize your window/DevTools panel if a modal
  seems to have no visible buttons (`M6.1-modaloverflow`).

---

## 3. Very detailed step-by-step test cases

Run every case below, in order, for whichever mode (D or W) you're currently testing. Each case
lists **Steps** (do these, in order) and **Expected Outcome** (what must be true for a Pass — if
any one point fails, the whole case is a Fail; write down exactly which point failed).

### M1 — Interface switching & launch

**M1.1 — Default interface and switch to Make**
- Steps:
  1. Launch with a fresh `~/.pwmgr` (wiped per 2.2).
  2. Register `uatuser` / password v1 (full Register steps are in M2.1 below — do that now, then continue here).
  3. Open Settings → Interface.
  4. Select `Make`.
  5. D: close the window and relaunch. W: navigate to `/makeui.html`.
- Expected outcome:
  - Fresh launch shows the Classic Register screen (default variant).
  - Interface setting shows exactly three options: `Classic`, `New`, `Make`.
  - Selecting Make shows a message: D says it applies on next launch; W names `/makeui.html` directly.
  - After relaunch/reopen, Make UI loads: sidebar header reads `LocalVault` / `Personal`.

**M1.2 — Switch back and forth**
- Steps:
  1. In Make, Settings → Interface → `New`. D: relaunch. W: open `/newui.html`.
  2. Switch to `Classic`. D: relaunch. W: open `/index.html`.
  3. Switch back to `Make`, relaunch/reopen.
- Expected outcome: New UI loads at step 1, Classic loads at step 2, Make loads again at step 3 —
  and (D) you're required to log in again each time (no persisted session across relaunch).

**M1.3 — Missing makeui.html fallback (D only)**
- Steps:
  1. Close the app.
  2. Rename `dist\makeui.html` to `dist\makeui.html.bak`.
  3. Confirm `ui_variant` is `make` in config, then launch.
  4. Close the app, rename the file back, relaunch.
- Expected outcome: with the asset missing, Classic UI opens instead of a blank window, and the
  terminal prints a fallback warning. After renaming back, Make loads again normally.

**M1.4 — Load-failure recovery button**
- Steps: only act if a Make render error surfaces naturally during your run.
- Expected outcome: an error screen appears with a "Switch to Classic UI" button; clicking it opens
  Classic, and it stays Classic on the next launch. If no such error ever occurs, record
  `N/A — not reproducible without deliberately breaking something`.

**M1.5 — Offline fonts (W only)**
- Steps:
  1. DevTools → Network tab → set throttling to `Offline`.
  2. Hard-reload `/makeui.html` (Ctrl+Shift+R).
  3. Check the Network tab for any request to `fonts.googleapis.com` or `fonts.gstatic.com`.
  4. DevTools → Elements → inspect `<body>`'s computed `font-family`.
  5. Restore throttling to `No throttling`.
- Expected outcome: no font-CDN requests fire; computed font-family still resolves to `Inter`; text
  looks the same as when online.

### M2 — Auth

**M2.1 — Register**
- Steps:
  1. On the Register screen, note whether a "no recovery" warning is visible.
  2. Enter username `uatuser`, master password `Uat-Master-Pass-1!`.
  3. Submit.
- Expected outcome: warning about no password recovery is shown; after submit, an empty vault
  opens — list shows an empty state, detail pane says something like "Select an item to view
  details" / "Your vault is encrypted locally".

**M2.2 — Wrong password**
- Steps:
  1. Click the Lock Vault icon in the sidebar.
  2. Confirm you land on the Log in screen with no confirmation prompt.
  3. Enter username `uatuser`, password `wrong-pass`. Submit.
  4. Enter username `nosuchuser`, any password. Submit.
- Expected outcome: both attempts show the exact same generic error, e.g. "Incorrect master
  password" — it must not hint whether the username or the password was wrong.

**M2.3 — Correct login**
- Steps: log in as `uatuser` / password v1.
- Expected outcome: a login spinner is shown briefly, then the vault opens.

### M3 — Item types & Add/Edit drawer

**M3.1 — Add Login with all fields**
- Steps:
  1. Click Add item (+). Confirm the type selector shows `Login`, `Secure Note`, `Payment Card`, `Identity`.
  2. Choose `Login`. Fill Name `GitHub`, Website URL `https://github.com`, Username `octo@uat.test`.
  3. Click Generate on the password field — confirm it fills with a random value; click Generate again — confirm the value changes.
  4. Overwrite the password field with `Gh!Pass-0001`. Fill Remarks `work acct`.
  5. Add 3 custom fields: `Recovery code` type Hidden = `RC-ALPHA-7788`; `Team` type Text = `platform`; `2FA on` type Boolean = Yes.
  6. Set TOTP secret to `JBSWY3DPEHPK3PXP`. Leave "Require master password to view" OFF. Folder: none. Favorite: off. Save.
  7. Confirm the drawer closes and `GitHub` appears in the list.
  8. Click into `GitHub`'s detail view.
- Expected outcome: All Items count = 1, Logins count = 1. Detail view shows Website (with a Copy
  URL control), username, a masked password, Remarks, a strength indicator, the 3 custom fields
  (Hidden one masked until revealed, Boolean showing "Yes"), and a live 6-digit TOTP code with a
  visible countdown.

**M3.2 — Add Secure Note**
- Steps: Add item → Secure Note. Name `Wifi`, body `SSID home / key wifi-secret-99`. Save.
- Expected outcome: Secure Notes count = 1; detail shows the note masked, with a reveal control
  that shows the full body text.

**M3.3 — Add Payment Card + hint validation**
- Steps:
  1. Add item → Payment Card. Name `Visa Main`, brand `Visa`, number `4111111111111111`, expiry `12/29`, CVV `123`. Save.
  2. Check the list row for a brand + last-4 hint.
  3. Open detail — check masking on card number and CVV, and that expiry shows in plain text; reveal the card number.
  4. Edit the card, change the number to the invalid short value `41111` (5 digits). Save.
- Expected outcome: step 2 — record whether the list row actually shows a "Visa •••• 1111"-style
  hint (Claude's pass found it does NOT — confirm or contradict). Step 3 — number and CVV masked,
  expiry plain, reveal shows the full 16-digit PAN. Step 4 — record the exact behavior: is the
  short number rejected with a validation error, or silently accepted and displayed as entered?
  (Claude's pass found it's silently accepted with no validation — confirm or contradict.)

**M3.4 — Add Identity**
- Steps: Add item → Identity. Name `Me`, full name `Uat Tester`. Save.
- Expected outcome: Identities count = 1; detail shows the full name.

**M3.5 — Add remaining logins (fixtures for later cases)**
- Steps: Add Login items `OldForum` (url `http://oldforum.test`, user `u1`, pw `password`), `ShopA`
  (pw `Reuse-Me-123`), `ShopB` (pw `Reuse-Me-123`).
- Expected outcome: All Items = 7, Logins = 4.

**M3.6 — Edit mode**
- Steps:
  1. Select `Visa Main` → Edit. Check that item type is shown read-only (can't be changed).
  2. Change Name to `Visa Primary`. Save.
  3. Lock the vault, log back in.
  4. Select `GitHub` → Edit (the site-level edit, not a credential's own edit).
- Expected outcome: step 2 — list and detail both update to the new name. Step 3 — name change and
  all card fields survived the lock/relogin cycle intact. Step 4 — the site-level drawer only
  exposes name/URL/notes/folder/favorite, with a note that username/password/other credential
  fields are edited from the credential block's own Edit button.

**M3.7 — Add second credential to a login**
- Steps: On `GitHub`, click "Add credential". Username `octo-alt`, password `Gh!Alt-0002`. Save.
- Expected outcome: detail shows the primary credential plus an expandable second credential block
  for `octo-alt`, each with its own Edit/Delete/Copy-username controls.

### M4 — Favorites & folders

**M4.1 — Favorites**
- Steps:
  1. On `GitHub` detail, click the star icon.
  2. Navigate to the sidebar's Favorites section.
  3. Unstar `GitHub`.
  4. Re-star it, then Lock and log back in.
- Expected outcome: starring turns the icon filled/yellow and its tooltip changes to "Unfavorite";
  Favorites section shows only `GitHub` while starred, and is empty once unstarred; the favorite
  status survives a lock/relogin cycle.

**M4.2 — Folder create**
- Steps:
  1. Before any folder exists, check the sidebar Folders section for placeholder text.
  2. Click Add folder (+), type `Work`, press Enter.
  3. Repeat for `Personal`.
- Expected outcome: both `Work` and `Personal` rows appear in the sidebar after creation. (Record
  whether an empty-state placeholder like "No folders yet" was shown before the first folder —
  Claude's pass found it was missing, just an empty header.)

**M4.3 — Folder validation**
- Steps:
  1. Try adding a folder with a blank or spaces-only name.
  2. Try adding a folder named `work` (lowercase, duplicate of `Work`).
- Expected outcome: step 1 rejected silently or with an error (no folder created either way). Step
  2 shows an explicit error: "A folder with that name already exists."

**M4.4 — Assign items to a folder**
- Steps: Edit `GitHub` → Folder `Work` → Save. Edit `Wifi` → Folder `Work` → Save. Click into the
  `Work` folder view.
- Expected outcome: the `Work` view shows exactly `GitHub` and `Wifi`; each item's detail shows a
  Folder chip reading `Work`.

**M4.5 — Folder rename**
- Steps:
  1. On the `Work` row, click Rename folder, type `Work2`, then press **Escape** (not Enter).
  2. Rename again, type `Job`, press **Enter**.
  3. Try renaming `Job` to `Personal` (an existing folder name).
- Expected outcome: step 1 — pressing Escape must cancel the rename; the folder name must stay
  `Work` (Claude's D pass found Escape incorrectly *commits* the rename instead — confirm or
  contradict this). Step 2 — row updates to `Job`, and `GitHub`'s detail folder chip updates too.
  Step 3 — rejected with a duplicate-name error; folder stays `Job`, no data corruption. (Also note
  whether the rename input box gets stuck open after the rejected rename — a known minor bug.)

**M4.6 — Folder delete unfiles items**
- Steps: while inside the `Job` folder view, click Delete folder, confirm the dialog.
- Expected outcome: a confirmation dialog appears before deletion (on Desktop this should be a
  native OS dialog, not a browser `confirm()` popup — note which kind you see, especially on W).
  After confirming: the folder is gone, the view falls back to All Items (not a broken/empty
  screen), and `GitHub`/`Wifi` still exist with no folder assigned. Item count is unchanged.

### M5 — Search

**M5.1 — Positive matches**
- Steps: type each of the following into the search box, one at a time, clearing between each:
  `git`, `octo@`, `work acct`, `RC-ALPHA`, `platform`, `wifi-secret`.
- Expected outcome: each search live-filters to the matching item — `GitHub` for the first five
  (site name, username, remarks, hidden custom field, text custom field), `Wifi` for the last
  (note body / sealed custom field).

**M5.2 — Negative matches**
- Steps:
  1. Search the exact password `Gh!Pass-0001`.
  2. Search `4111` (the card number prefix).
  3. Clear the search box.
- Expected outcome: step 1 — `GitHub` must NOT appear (passwords aren't searchable). Step 2 —
  record whether the card matches (by design the sealed custom-field-style card number IS
  searchable — confirm or contradict). Step 3 — the full list returns.

### M6 — Security gates

**M6.1 — Per-credential reprompt**
- Steps:
  1. Settings → Security → Reprompt mode = `Per-credential (only flagged items)`.
  2. Edit `GitHub`'s primary credential, enable "Require master password to view", Save.
  3. Click reveal on the password.
  4. Enter the wrong master password.
  5. Enter the correct master password (v1 or whatever is current).
  6. Reveal `ShopA`'s password (not flagged).
  7. Click Copy on `GitHub`'s password.
- Expected outcome: step 3 triggers a master-password prompt. Step 4 is rejected, password stays
  masked. Step 5 succeeds, password reveals. Step 6 needs no prompt. Step 7 also triggers the
  prompt (reprompt applies to copy, not just reveal).

**M6.2 — Global reprompt + grace window**
- Steps:
  1. Settings → Reprompt mode = `Global (every item)`, grace window = `15` seconds.
  2. Reveal `ShopA` — enter the master password when prompted.
  3. Immediately (well under 15s) reveal `ShopB`.
  4. Wait more than 15 seconds, then reveal `OldForum`.
  5. Set mode back to `Per-credential` afterward.
- Expected outcome: step 2 prompts. Step 3 must NOT prompt again (within the grace window). Step 4
  must prompt again (grace window expired). (Claude's pass found this genuinely worked on D but
  the grace window was never honored on W, and separately found a D-specific bug where clicking
  Cancel on an already-revealed field leaves it visible in plaintext — watch for both.)

**M6.3 — Mask length hiding**
- Steps: compare the masked password display for `OldForum` (8-character password) and `GitHub`
  (12-character password).
- Expected outcome: both show the identical number of mask characters (8 bullets) — the mask must
  not reveal the real password length.

**M6.4 — Clipboard 30-second auto-clear**
- Steps:
  1. Copy `ShopA`'s password. Paste into Notepad.
  2. Wait 35 seconds. Paste again.
  3. Repeat both steps for Copy Username.
- Expected outcome: step 1 pastes `Reuse-Me-123`. Step 2's second paste must NOT be the password —
  clipboard should be empty or changed. Same for username.

**M6.5 — TOTP**
- Steps: watch `GitHub`'s TOTP code and countdown for at least 30 seconds continuously.
- Expected outcome: a 6-digit code is shown with a visible countdown that decrements every second
  and the code changes when it hits 0. (Claude's W pass found the countdown display sometimes gets
  stuck for 10+ seconds even though the underlying code/timing is correct — watch specifically for
  smooth per-second countdown, not just eventual correctness.)

### M7 — Password history

**M7.1 — Cap at 5, newest first**
- Steps:
  1. Edit `ShopA`'s credential, change password to `Hist-1`, Save. Repeat for `Hist-2` through
     `Hist-6` (6 total changes).
  2. Expand Password history.
  3. Reveal one of the history entries.
- Expected outcome: exactly 5 entries shown, newest first: `Hist-5`, `Hist-4`, `Hist-3`, `Hist-2`,
  `Hist-1` — the original `Reuse-Me-123` has rolled off. Each entry has a timestamp. Revealing an
  entry goes through the same reprompt gate as the live password would.

**M7.2 — No history entry for username-only change**
- Steps: edit `ShopA`, change only the username, Save. Then check `Visa Primary`'s detail for a
  history section.
- Expected outcome: password history for `ShopA` is unchanged (still the same 5 entries) — a
  username-only edit must not create a history entry. `Visa Primary` (never had a password change)
  should show "No previous passwords recorded." or have no history section at all — record which.

### M8 — Trash

**M8.1 — Credential delete/restore**
- Steps:
  1. Delete `GitHub`'s second credential (`octo-alt`).
  2. Open Trash, select the entry.
  3. Restore it.
- Expected outcome: step 1 removes it from `GitHub`'s detail immediately, no confirm dialog needed
  for a credential-level delete. Step 2 shows it in Trash with its item type, and selecting it
  shows a Username/Email field plus a note about bulk Empty Trash. Step 3 puts it back under
  `GitHub`.

**M8.2 — Site cascade delete**
- Steps: delete the `OldForum` site (confirm the dialog), then restore its credential from Trash.
- Expected outcome: the confirm dialog explicitly mentions the credential(s) that will move to
  Trash. After confirming, `OldForum` is gone from All Items and its credential is in Trash.
  Restoring that credential brings `OldForum` back into the list.

**M8.3 — Empty Trash**
- Steps:
  1. Delete `ShopB`'s site (confirm).
  2. Open Trash, click Empty Trash, then Cancel on the confirmation.
  3. Click Empty Trash again, this time confirm.
  4. Lock and log back in.
- Expected outcome: step 2 — canceling leaves everything in Trash untouched. Step 3 — Trash shows
  "Trash is empty" and `ShopB` is not in All Items. Step 4 — `ShopB` does not come back.

### M9 — Master password change

Run M9.1–M9.3 once in Make, then again in Classic Settings (M9.6) before moving on.

**M9.0 — Pre-setup**
- Steps:
  1. Settings → Security → set PIN `2468`.
  2. D only: Settings → SSH Agent — **skip actually enabling the toggle** (see Section 2.5's crash
     warning); if you want to test it anyway, read `docs/HOW_TO_TEST_SSH_AGENT.md` first and have
     the config.json recovery step ready.
  3. Delete one credential (e.g. `ShopA`'s) so Trash is non-empty going into M9.4.
- Expected outcome: PIN set confirmation shown; Trash has at least one entry before continuing.

**M9.1 — Form validation**
- Steps:
  1. Settings → Security → Change master password, with all fields empty.
  2. Fill Current = v1, New = v2, Confirm = a mismatched value.
  3. Try submitting with New left empty.
- Expected outcome: step 1 — submit button stays disabled. Step 2 — error "New passwords don't
  match.", no change applied — and check whether this error is actually **visible on screen**
  (Claude's W pass found it rendered off-screen above the viewport — confirm or contradict on your
  browser/window size). Step 3 — rejected.

**M9.2 — Wrong current password**
- Steps: Current = `wrong`, New/Confirm = v2. Submit. Then Lock and log in with v1.
- Expected outcome: a clear, correctly-worded error about the invalid current password (not a
  generic "vault is locked" message — Claude's W pass found the wrong message shown here). Master
  password must be unchanged — v1 still logs in afterward.

**M9.3 — Success**
- Steps:
  1. Current = v1, New/Confirm = v2. Submit.
  2. Check PIN and (D) biometric settings state afterward.
  3. Check `%USERPROFILE%\.pwmgr\backups\` in File Explorer.
- Expected outcome: button shows a transitional "Changing…" state then a success message; you stay
  logged in (no forced logout). PIN reverts to "not set". A new backup file named
  `uatuser-premasterchange-<digits>.bak` appears in the backups folder.

**M9.4 — Post-change decrypt**
- Steps:
  1. Lock, log in with v1.
  2. Log in with v2.
  3. Reveal/check every item type's data.
  4. Look at the login screen for a "PIN" option.
- Expected outcome: step 1 fails with "Incorrect master password". Step 2 succeeds. Step 3 — every
  item decrypts correctly: GitHub password + `RC-ALPHA-7788` custom field + live TOTP; `ShopA`
  history entries; `Wifi` note body; card number + CVV; identity full name; the Trash item restores
  and reveals correctly; (D) the SSH key, if added, is still listed. Step 4 — no "Unlock with PIN
  instead" link (PIN was cleared by the password change).

**M9.5 — Same-as-old password (observation only)**
- Steps: change password from v2 back to v2 (no actual change).
- Expected outcome: just record what happens — does the backend silently accept it? Does the UI
  show any success or error message either way? (Claude's pass found it's a silent no-op with zero
  UI feedback — confirm or contradict.)

**M9.6 — Repeat in Classic**
- Steps: Settings → Interface → Classic, relaunch/reopen, repeat M9.1–M9.3 in Classic's Settings
  screen (changing v2 → v1 this time). Switch back to Make afterward.
- Expected outcome: same behavior as M9.1–M9.3 above, in Classic's layout.

### M10 — Data Hub

**M10.1 — Backup & restore**
- Steps:
  1. Data Hub → Create Backup.
  2. Add a new item `AfterBackup` (any Login).
  3. Data Hub → Restore Backup → select the file from step 1.
  4. Log in with the current master password.
- Expected outcome: step 1 — D writes a file into the backups folder; W triggers a browser
  download. Step 3 — the app locks and forces you to the login screen. Step 4 — `AfterBackup` is
  gone (rolled back to the pre-backup state); folders, favorites, item types, card hints, and
  history are all still intact from before the backup.

**M10.2 — CSV export**
- Steps:
  1. Data Hub → Export to CSV — read the gap-note text about what CSV excludes.
  2. Try exporting with a blank master password, then a wrong one.
  3. Export mode "Encrypted (this device)" with the correct master password.
  4. Export mode "Encrypted (with a password)": enter mismatched export passwords, then matching
     ones.
  5. Export mode "Plaintext": try without typing the confirmation phrase, then with it typed
     exactly as `EXPORT PLAINTEXT`.
  6. Switch to Classic, open its CSV Export screen.
- Expected outcome: step 1 — the note explicitly says CSV excludes favorites/folders/item
  type/custom fields/TOTP/history and recommends Backup instead. Step 2 — blank is blocked, wrong
  password is rejected with a visible error. Step 3 — file saved/downloaded. Step 4 — mismatch
  shows an error; matching passwords save successfully. Step 5 — blocked without the exact phrase;
  succeeds once typed correctly, and the resulting file has plaintext rows with no custom-field or
  TOTP columns. Step 6 — the same gap-note text appears in Classic too.

**M10.3 — CSV import**
- Steps:
  1. Data Hub → Import from CSV → source "Chrome / Chromium export" → choose
     `%TEMP%\uat\chrome.csv`.
  2. Review the preview (don't commit yet) and confirm nothing is saved.
  3. Edit `ImportB`'s username cell to `bobby`.
  4. Commit the import.
- Expected outcome: step 2 — preview shows all 3 rows, editable, and All Items count is unchanged
  before committing. Step 4 — `ImportA` and `ImportB` are each added exactly once (the duplicate
  row is skipped and reported as such); `ImportB`'s username is `bobby`; both are type Login.

### M11 — Restyled screens (Make UI only)

**M11.1 — Visual check per screen**
- Steps: for each of Health Dashboard, Password Generator, Data Hub, Settings, and Trash:
  1. View in dark theme.
  2. Toggle to light theme via the sidebar sun/moon icon.
  3. Settings → Appearance → cycle through all 5 accents: Indigo, Sky, Teal, Purple, Pink.
- Expected outcome: dark theme uses the Make palette consistently (not Classic's plain list/navy
  look), with headings matching expected text (e.g. "Health Dashboard / Security analysis of your
  vault"). Light theme stays readable with no dark-only leftover styling. Each accent change
  visibly updates the active nav pill and buttons.

**M11.2 — Health Dashboard content**
- Steps:
  1. Open Health Dashboard, check Weak/Reused/Unsecured/Old counts.
  2. Click into one flagged entry.
  3. With HIBP off, check the Breached card; enable HIBP in Settings, click Check Now.
  4. Change the "old password" threshold (90/180/365 days).
- Expected outcome: step 1 — `OldForum` counted as Weak (plaintext `password`) and Unsecured
  (http://); `ShopA`+`ShopB` grouped as Reused (same password); Old Passwords = 0 (nothing aged
  yet). Step 2 — navigates to that item's detail. Step 3 — with HIBP off, no network call fires,
  just a prompt to enable it; after enabling and clicking Check Now, `OldForum` is flagged as
  breached (this should be the only network call anywhere in the app). Step 4 — saves without
  error.

**M11.3 — Password Generator**
- Steps: try each generation method — Random, Memorable, Pattern (with a custom template like
  `Cvc#`), Pronounceable. Copy a generated value. Switch Settings → strength algorithm between
  Simple and Advanced.
- Expected outcome: each method fills the field with output matching its shape (e.g. Pattern `Cvc#`
  produces something like `Vaz8`); a strength bar/label updates. Copied value clears from clipboard
  after 30s (same check as M6.4). Switching the strength algorithm doesn't change the Health
  Dashboard's Weak-password count (that page always uses the advanced algorithm regardless of the
  Settings toggle — this is documented, intentional behavior, not a bug).

**M11.4 — Settings persistence**
- Steps: change auto-lock to 1 minute, and change theme, accent, mobile nav style, reprompt grace.
  Fully close and relaunch/reload. Then idle for 70+ seconds with auto-lock at 1 minute.
- Expected outcome: all changed settings persist across the relaunch/reload. After 70+ idle
  seconds, the app actually locks to the login screen (a real trigger, not just a visual freeze —
  confirm it requires a fresh login to get back in).

### M12 — Responsive layout

**M12.1 — Narrow (<768px, e.g. resize to ~390px wide, or DevTools device toolbar on W)**
- Steps:
  1. Resize to narrow width.
  2. Open the hamburger/menu icon.
  3. Select an item to view its detail.
  4. Open the Add Item drawer.
- Expected outcome: step 1 — a top bar with a title and a menu icon; sidebar hidden; list full
  width. Step 2 — sidebar slides over as an overlay with its own close control; selecting a
  destination closes the overlay. Step 3 — detail view replaces the list; a back control returns to
  the list. Step 4 — drawer is fully usable, no horizontal scrollbar.

**M12.2 — Wide (≥768px)**
- Steps: widen back past 768px with an item still selected.
- Expected outcome: sidebar + list + detail all show side by side, no top bar/hamburger; no crash
  during the resize; the previously-selected item stays selected and its detail stays rendered.

### M13 — Regression: Classic, New, Viewer

**M13.1 — Classic smoke test**
- Steps: add a site + credential, search for it, delete it (confirm it lands in Trash), restore
  it, run the password generator, change the theme. Open `GitHub`.
- Expected outcome: everything works exactly as before this branch's changes. `GitHub`'s custom
  fields and TOTP still render correctly (no errors from the newer item-type/folder schema).

**M13.2 — New UI**
- Steps: switch to Interface New. Check Favorites. Open `Wifi`, `Visa Primary`, and `Me`.
- Expected outcome: Favorites shows `GitHub`. The newer item types (Note/Card/Identity) open and
  render without crashing, even though this legacy UI predates them (they may render as generic
  rows — that's expected, not a bug).

**M13.3 — Viewer app**
- Steps: launch the Viewer app per `docs/testing/HOW_TO_TEST_VIEWER_APP.md`, log in, search `git`,
  reveal a credential, then click Lock.
- Expected outcome: search returns the right result; reveal shows the real password; clicking Lock
  should tear down the UI back to a login screen (Claude's pass found it does NOT — the list and
  revealed secret stay visible until a hard reload — confirm or contradict, this is `M13.3-viewer-lockbug`, a Blocker).

### D-only cases

**D1 — Windows Hello biometric**
- Steps: Settings → Security → enable biometric, complete Windows Hello enrollment, Lock the
  vault, then look at the Login screen **before typing anything in either field**.
- Expected outcome: a "Unlock with Windows Hello" button should be visible based on username +
  enrollment state alone. (Claude's pass found the button only appears once the Master password
  field has any content — try it with both fields empty first, and confirm or contradict this
  specific finding, `D1`.) If your machine has no biometric hardware, record `N/A`.

**D2 — SSH agent**
- Per Section 2.5: **recommended to skip actually enabling the toggle** given the confirmed crash.
  If you do want to test it, read `docs/HOW_TO_TEST_SSH_AGENT.md` fully first, and have this ready
  in case of a crash-loop: edit `%USERPROFILE%\.pwmgr\config.json`, set `"ssh_agent_enabled":
  false`, save, relaunch. Root cause is documented in the D2 row of the findings doc — record
  whether your result matches (a panic at `ssh_agent_service.rs:138`, "there is no reactor
  running...") or differs.

**D3 — Window title**
- Steps: just look at the title bar throughout your session.
- Expected outcome: it reads `tauri-app` (a known cosmetic placeholder, not a branded name) — just
  confirm it, this is a Minor/cosmetic item, not something to spend time on.

---

## 4. Expected outcome — summary

Every case above already states its own expected outcome inline (this is where "Expected Outcome"
lives for each case — see Section 3). At the whole-run level: your goal is a filled **User result**
column for every case ID in the findings doc, either matching Claude's result or explicitly
differing from it, with enough detail in Observation to tell the two apart.

---

## 5. Where to note your findings

Open `docs/testing/UAT_FINDINGS_2026-09-23.md`. It's a table with these columns:

| Column | What goes here |
|---|---|
| **ID** | Already filled — matches the case IDs in Section 3 above (e.g. `M3.1`, `D2`). |
| **Mode** | Already filled — `D` or `W`. |
| **Claude result** | Already filled — Claude's own Pass/Fail/N/A/Blocked from this session. Don't edit this. |
| **User result** | **Fill this in** — `Pass`, `Fail`, `N/A (reason)`, or `Blocked (by what)`. |
| **Observation** | **Fill this in** — free text. What you actually saw, especially for any Fail. If it matches an existing Claude finding, a short "reproduced, matches Claude's finding" is enough. If it's new or different, describe it fully: what you did, what happened, what you expected instead. |
| **Screenshot** | Optional — a filename/path if you captured one, otherwise leave blank. |
| **Severity** | Your judgment: `Blocker` / `Major` / `Minor` / cosmetic. Leave blank if unsure — Claude will help classify during joint review. |
| **Action** | Leave as `TBD` — this gets filled during joint review (fix now / defer / accept), not by you. |

**Extra feedback or observations that don't fit a specific case ID:** add them as new rows at the
bottom of the same table with a made-up ID prefixed `USER-` (e.g. `USER-01`), or just add a short
freeform "Additional User Notes" section at the very end of the findings doc — either is fine, the
important thing is it isn't lost.

When you're done with both D and W, tell Claude — that kicks off joint review (plan Process step
4): go through every finding together and decide fix now / defer to `docs/DEFERRED.md` / accept as
known behavior. No fixes happen before that step.
