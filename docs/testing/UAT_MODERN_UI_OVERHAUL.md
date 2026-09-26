# UAT Script — `feature/modern-ui-overhaul`

Verbatim copy of Part 0–3 from `docs/Plans/UAT_Modern_UI_Overhaul_Plan_2026-09-23.md`, frozen at UAT start (2026-09-24) as the script both Claude and the user run.

## Part 0 — Setup (both modes)

0.1 Open PowerShell at `E:\Coding_Projects-CS\Account_1\Password_Manager`.
0.2 Run `$env:PATH = "$env:USERPROFILE\.cargo\bin;C:\Strawberry\perl\bin;C:\Strawberry\c\bin;" + $env:PATH`.
0.3 Run `git branch --show-current` → **Check:** prints `feature/modern-ui-overhaul`.
0.4 Run `git status --short` → **Check:** empty (clean tree; UAT tests committed code only).
0.5 Close any running app / `pwmgr-web` instance.
0.6 Delete test data: `Remove-Item -Recurse -Force "$env:USERPROFILE\.pwmgr"` → **Check:** folder gone.
0.7 Create fixtures in `%TEMP%\uat\`:
   - SSH key: `ssh-keygen -t ed25519 -N '""' -f "$env:TEMP\uat\uat_key" -C uat` (D only).
   - `chrome.csv` with header `name,url,username,password,note` and 3 rows:
     `ImportA,https://a.example,alice,Imp0rt!A1,note a` / `ImportB,https://b.example,bob,Imp0rt!B2,` /
     duplicate of ImportA row (to test dedupe).

**Standard test data** (reused by later cases — enter exactly):

| Key | Value |
|---|---|
| User | `uatuser` |
| Master pw v1 | `Uat-Master-Pass-1!` |
| Master pw v2 | `Uat-Master-Pass-2!` |
| Login item | Name `GitHub`, URL `https://github.com`, user `octo@uat.test`, pw `Gh!Pass-0001`, remarks `work acct`, TOTP `JBSWY3DPEHPK3PXP` |
| Custom fields on GitHub | `Recovery code` Hidden `RC-ALPHA-7788`; `Team` Text `platform`; `2FA on` Boolean Yes |
| Insecure login | Name `OldForum`, URL `http://oldforum.test`, user `u1`, pw `password` |
| Reused login | Name `ShopA`, pw `Reuse-Me-123`; Name `ShopB`, pw `Reuse-Me-123` |
| Note | Name `Wifi`, body `SSID home / key wifi-secret-99` |
| Card | Name `Visa Main`, brand `Visa`, number `4111111111111111`, expiry `12/29`, CVV `123` |
| Identity | Name `Me`, full name `Uat Tester` |
| Folders | `Work`, `Personal` |

---

## Part 1 — Automated (Claude runs first; results recorded, manual continues regardless)

**A1** `cargo test --workspace` → **Check:** final line per crate `test result: ok`, 0 failed. Note count.
**A2** `npm test` → **Check:** all files pass (incl. `src/newui/lib/parity.test.ts`, `src/components/Spinner.test.ts`).
**A3** `npm run build` → **Check:** no `tsc` errors; `dist\` contains `index.html`, `viewer.html`, `newui.html`, `makeui.html`.

---

## Part 2 — Launch procedures

**Launch-D (desktop):** `npm run tauri:dev` → wait for window. **Check:** window title shows, no blank white window after 30s.
**Launch-W (web):** `npm run build` then `cargo run -p pwmgr-web` → **Check:** console prints
`pwmgr-web listening on http://127.0.0.1:7373`. Open `http://127.0.0.1:7373/makeui.html` in Brave (user: any Chromium browser; Claude: Playwright-launched Brave).
DevTools open (F12) on Console tab — **any red error during a case = record it**.

Mode switch between D and W runs: stop one before starting other (shared `~/.pwmgr` files).
Order: full Part 3 on **W** (Phase A), then repeat Part 0.6 wipe and full Part 3 on **D** (Phase B).
User's own independent run can use either order; wipe `.pwmgr` between modes.

---

## Part 3 — Manual cases

Each case: run steps in order; every **Check** must hold for Pass. Any failed Check = Fail, write
which step number in Observation.

### M1 — Interface switching & launch

**M1.1 Default interface and switch to Make**
1. Launch (fresh `.pwmgr`). → **Check:** Classic Register screen appears (default variant).
2. Register `uatuser` / pw v1 (see M2.1 for field details). Open Settings → Interface.
3. → **Check:** three options: `Classic`, `New`, `Make`.
4. Select `Make`. → **Check:** D: message says change applies on next launch. W: message names `/makeui.html`.
5. D: close window, relaunch. W: open `/makeui.html`. → **Check:** Make UI loads (sidebar header `LocalVault` / `Personal`) 📷.

**M1.2 Switch back and forth**
1. In Make: Settings → Interface → `New`. D relaunch / W open `/newui.html`. → **Check:** New UI loads.
2. Switch to `Classic`. D relaunch / W open `/index.html`. → **Check:** Classic loads.
3. Switch back to `Make`, relaunch/reopen. → **Check:** Make loads, still logged-out/login required after relaunch (D).

**M1.3 Missing makeui.html fallback (D only)**
1. Close app. Rename `dist\makeui.html` → `dist\makeui.html.bak`.
2. Ensure `ui_variant` = make. Launch-D.
3. → **Check:** Classic UI opens (not blank window); terminal shows fallback warning.
4. Close app, rename file back. Relaunch → **Check:** Make loads again.

**M1.4 Load-failure recovery button**
1. Only if a Make render error surfaces during UAT naturally: → **Check:** error screen shows "Switch to Classic UI"; clicking it opens Classic and next launch stays Classic.
2. Otherwise mark `N/A — not reproducible without code change`.

**M1.5 Offline fonts (W)**
1. DevTools → Network → throttle `Offline`. Hard reload `/makeui.html` (Ctrl+Shift+R).
2. → **Check:** no requests to `fonts.googleapis.com`/`fonts.gstatic.com` in Network list.
3. → **Check:** DevTools Elements → body computed `font-family` resolves to `Inter`; text looks identical to online 📷.
4. Restore throttle to `No throttling`.

### M2 — Auth (Make reuses Classic Login/Register screens)

**M2.1 Register** (if not already done in M1.1 — else wipe and redo)
1. Screen heading `Register`. → **Check:** "no recovery" warning visible.
2. `Choose a username`: `uatuser`. `Choose a master password`: pw v1. Submit.
3. → **Check:** lands on empty Make vault; list shows empty state; detail pane shows "Select an item to view details / Your vault is encrypted locally" 📷.

**M2.2 Wrong password**
1. Sidebar bottom → `Lock Vault` icon. → **Check:** `Log in` screen immediately, no confirm.
2. Username `uatuser`, Master password `wrong-pass`. Log in.
3. → **Check:** error exactly generic "Incorrect master password" (no hint which field wrong).
4. Username `nosuchuser`, any pw. → **Check:** same generic message.

**M2.3 Correct login**
1. `uatuser` / pw v1 → Log in. → **Check:** vault opens, spinner shown during login.

### M3 — Item types & Add/Edit drawer

**M3.1 Add Login with all fields**
1. Item list header → `Add item` (+). → **Check:** drawer slides in; type selector shows `Login`, `Secure Note`, `Payment Card`, `Identity`.
2. Type `Login`. Name `GitHub`, Website URL, Username per standard data.
3. Password: click Generate. → **Check:** password field fills with generated value (not blank); pressing Generate again gives different value.
4. Overwrite Password with `Gh!Pass-0001`. Remarks `work acct`.
5. Custom fields: add 3 rows per standard data, picking type `Hidden` / `Text` / `Boolean`.
6. Authenticator (TOTP) secret: `JBSWY3DPEHPK3PXP`. Leave "Require master password to view" OFF. Folder `No folder`. Favorite off. Save.
7. → **Check:** drawer closes; `GitHub` in list with Login (key) tile; All Items count = 1; Logins count = 1 📷.
8. Select `GitHub`. → **Check:** detail shows Website (with Copy URL), username, masked password, Remarks, Strength, custom fields (Hidden one masked, Boolean shows Yes), live 6-digit TOTP with countdown.

**M3.2 Add Secure Note**
1. Add item → `Secure Note`. Name `Wifi`, Note body per data. Save.
2. → **Check:** list shows note tile; Secure Notes count 1; detail shows Note masked; reveal shows full body.

**M3.3 Add Payment Card + hint validation**
1. Add item → `Payment Card`. Name `Visa Main`, Card brand `Visa`, Card number, Expiration date `12/29`, Security code `123`. Save.
2. → **Check:** list row shows brand + last-4 hint `1111` (no full number); Payment Cards count 1.
3. Detail → **Check:** Card number masked, CVV masked, expiry `12/29` plain text; reveal card number shows `4111111111111111`.
4. Edit card, change number to `41111` (short). Save. → **Check:** either rejected with error or hint not updated to invalid value — record exact behavior (backend requires exactly 4 digits, `vault.rs:316`).

**M3.4 Add Identity**
1. Add item → `Identity`. Name `Me`, Full name `Uat Tester`. Save.
2. → **Check:** Identities count 1; detail shows Full name.

**M3.5 Add remaining logins** (fixtures for later)
1. Add `OldForum`, `ShopA`, `ShopB` per standard data (type Login).
2. → **Check:** All Items = 7, Logins = 4.

**M3.6 Edit mode**
1. Select `Visa Main` → `Edit`. → **Check:** item type shown read-only (cannot change type).
2. Change Name to `Visa Primary`, Save. → **Check:** list + detail update.
3. Lock, log back in. → **Check:** name persisted, card fields intact.
4. Select `GitHub` → `Edit` (site-level). → **Check:** drawer edits site fields only (name/url/notes/folder/favorite); credential fields edited via credential block's own `Edit`.

**M3.7 Add second credential to a login**
1. `GitHub` → `Add credential`. Username `octo-alt`, password `Gh!Alt-0002`. Save.
2. → **Check:** detail shows primary + second credential blocks; each has own Edit/Delete/Copy username.

### M4 — Favorites & folders

**M4.1 Favorites**
1. `GitHub` detail → star icon (tooltip `Favorite`). → **Check:** star turns yellow/filled; tooltip now `Unfavorite`; Favorites count 1.
2. Sidebar → `Favorites`. → **Check:** only `GitHub` listed.
3. Unstar. → **Check:** Favorites count 0, list empty.
4. Re-star `GitHub` (fixture). Lock/login → **Check:** still favorite.

**M4.2 Folder create**
1. Sidebar Folders header → `Add folder` (+). → **Check:** inline input `Folder name`; before any exists sidebar showed "No folders yet".
2. Type `Work`, Enter. → **Check:** `Work` row appears. Add `Personal` same way.

**M4.3 Folder validation**
1. Add folder with blank / spaces-only name, Enter. → **Check:** rejected (error shown or nothing created).
2. Add folder `work` (lowercase). → **Check:** error "A folder with that name already exists."

**M4.4 Assign items to folder**
1. Edit `GitHub` → Folder `Work`, Save. Edit `Wifi` → Folder `Work`, Save.
2. Sidebar → `Work`. → **Check:** exactly `GitHub`, `Wifi` listed; detail shows Folder chip `Work`.

**M4.5 Folder rename**
1. `Work` row → `Rename folder`. Type `Work2`, press Escape. → **Check:** name stays `Work`.
2. Rename again, type `Job`, Enter. → **Check:** row shows `Job`; `GitHub` detail folder chip shows `Job`.
3. Rename `Job` → `Personal` (existing). → **Check:** duplicate error.

**M4.6 Folder delete unfiles**
1. Navigate into `Job` folder view. `Job` → `Delete folder`. → **Check:** confirm dialog appears. Confirm.
2. → **Check:** folder gone; view falls back to `All Items` (not empty/broken view); `GitHub` and `Wifi` still exist with Folder = none. All Items count unchanged.

### M5 — Search

**M5.1 Positive matches** — type each in `Search…`, → **Check:** listed item(s) appear live as you type:
1. `git` → GitHub (site name).
2. `octo@` → GitHub (username).
3. `work acct` → GitHub (remarks).
4. `RC-ALPHA` → GitHub (hidden custom field value).
5. `platform` → GitHub (text custom field).
6. `wifi-secret` → Wifi (note body, sealed custom field).

**M5.2 Negative matches**
1. `Gh!Pass-0001` → **Check:** GitHub NOT listed (main password not searchable).
2. `4111` → record whether card appears (card number is sealed custom field — expected match per design; record actual).
3. Clear box → **Check:** full list returns.

### M6 — Security gates

**M6.1 Per-credential reprompt**
1. Settings → Security/Vault → Reprompt mode `Per-credential (only flagged items)`.
2. `GitHub` primary credential → Edit → enable "Require master password to view". Save.
3. Click reveal on password. → **Check:** master-password prompt appears.
4. Enter `wrong`. → **Check:** rejected, password stays masked.
5. Enter pw v1. → **Check:** password revealed.
6. Reveal `ShopA` password (not flagged). → **Check:** no prompt.
7. Click copy on GitHub password → **Check:** prompt again (reprompt fires on copy too).

**M6.2 Global reprompt + grace**
1. Settings → Reprompt mode `Global (every item)`; grace window `15` s.
2. Reveal `ShopA`. → **Check:** prompt. Enter pw v1.
3. Within 15 s reveal `ShopB`. → **Check:** no prompt.
4. Wait 20 s, reveal `OldForum`. → **Check:** prompt again.
5. Set mode back to `Per-credential` (keeps later cases simple).

**M6.3 Mask length hiding**
1. With passwords masked, compare `OldForum` (`password`, 8 chars) and `GitHub` (12 chars). → **Check:** both show identical 8-bullet mask 📷.

**M6.4 Clipboard 30s clear**
1. Copy `ShopA` password. Paste into Notepad → **Check:** `Reuse-Me-123`.
2. Wait 35 s. Paste again → **Check:** clipboard empty/changed (not the password). Repeat for `Copy username`.

**M6.5 TOTP**
1. `GitHub` detail TOTP. → **Check:** 6 digits, countdown decrements, code changes at 0.
2. Optional: add `JBSWY3DPEHPK3PXP` to a phone authenticator → **Check:** codes match.

### M7 — Password history

**M7.1 Cap at 5, newest first**
1. `ShopA` credential → Edit → password `Hist-1`, Save. Repeat with `Hist-2` … `Hist-6` (6 changes).
2. Expand `Password history`. → **Check:** exactly 5 entries; newest first: `Hist-5`,`Hist-4`,`Hist-3`,`Hist-2`,`Hist-1` (original `Reuse-Me-123` rolled off); each with timestamp.
3. Reveal an entry. → **Check:** goes through same gate as live password (prompts if credential flagged / global mode).

**M7.2 No entry for username-only change**
1. `ShopA` → Edit → change username only. Save. → **Check:** history still 5, unchanged.
2. `Visa Primary` detail → **Check:** "No previous passwords recorded." or no history section (record which).

### M8 — Trash

**M8.1 Credential delete/restore**
1. `GitHub` second credential (`octo-alt`) → `Delete`. → **Check:** removed from detail.
2. Sidebar → `Trash`. → **Check:** entry listed with item type; select → "Select an item to restore it" replaced by detail showing `Username / Email` and note about bulk Empty Trash.
3. Restore. → **Check:** back under `GitHub`.

**M8.2 Site cascade delete**
1. `OldForum` → detail `Delete`. → **Check:** confirm dialog mentions credentials. Confirm.
2. → **Check:** `OldForum` gone from All Items; its credential in Trash.
3. Restore that credential. → **Check:** `OldForum` site reappears in list.

**M8.3 Empty Trash**
1. Delete `ShopB` site (confirm). Trash → `Empty Trash`. → **Check:** confirm dialog. Cancel → nothing removed.
2. Empty Trash → confirm. → **Check:** "Trash is empty"; `ShopB` not in All Items and doesn't return after lock/login.

### M9 — Master-password change (run in Make, then repeat M9.1–M9.3 in Classic Settings)

**M9.0 Pre-setup**
1. Settings → Security → set PIN `2468`. → **Check:** PIN set confirmation.
2. D only: add SSH key — Settings → SSH Agent → Label `uat`, paste contents of `%TEMP%\uat\uat_key`. Add. → **Check:** listed under `Stored Keys`.
3. Delete one credential so Trash is non-empty (e.g. `ShopA` credential).

**M9.1 Form validation**
1. Settings → Security → `Change master password`. → **Check:** button disabled with any field empty.
2. Current pw v1, New pw v2, Confirm `mismatch`. → **Check:** "New passwords don't match."; no change.
3. New empty (if form allows submit) → **Check:** rejected.

**M9.2 Wrong current password**
1. Current `wrong`, New/Confirm pw v2. Submit. → **Check:** error (invalid credentials); then Lock → login with pw v1 still works.

**M9.3 Success**
1. Current pw v1, New/Confirm pw v2. Submit. → **Check:** button shows "Changing…" then "Master password changed."; still in vault (no logout).
2. → **Check:** PIN now shown as not set; D: biometric shown unset.
3. Explorer: `%USERPROFILE%\.pwmgr\backups\` → **Check:** new file `uatuser-premasterchange-<digits>.bak`.

**M9.4 Post-change decrypt**
1. Lock. Log in with pw v1 → **Check:** "Incorrect master password".
2. Log in with pw v2 → **Check:** success.
3. Reveal each → **Check:** all show original values: GitHub password + `RC-ALPHA-7788` custom field + TOTP code ticking; `ShopA` history entries; `Wifi` note; card number + CVV; identity full name; Trash item restore works and its password reveals; D: SSH key still listed.
4. Login screen → **Check:** no "Unlock with PIN instead" option (PIN cleared).

**M9.5 Same-as-old (observation only)**
1. Change pw v2 → pw v2. → **Check:** record behavior (backend accepts; note if UI warns).

**M9.6 Classic form** — Settings → Interface Classic, relaunch/reopen, repeat M9.1–M9.3 with pw v2 → pw v1 on Classic Settings screen. Switch back to Make after.

### M10 — Data Hub

**M10.1 Backup & restore**
1. Sidebar → `Data Hub` → Create Backup (tooltip "Write an encrypted backup of this vault"). → **Check:** D: file in `backups\`; W: browser downloads `.bak`.
2. Add a new item `AfterBackup` (Login, any pw).
3. Restore Backup → choose the `.bak` from step 1. → **Check:** session locks → login screen.
4. Log in with current master pw. → **Check:** `AfterBackup` gone; folders, favorites (GitHub starred), item types, card hint, history all intact.

**M10.2 CSV export**
1. Data Hub → `Export to CSV`. → **Check:** gap note visible (CSV excludes favorites, folders, item type, custom fields, TOTP, history; recommends Backup).
2. Export with blank master pw → **Check:** blocked. Wrong master pw → **Check:** rejected.
3. Mode Encrypted (this device) + master pw → **Check:** file saved/downloaded.
4. Mode Encrypted (with a password): Export password `Exp-1`, confirm `Exp-2` → **Check:** mismatch error. Match → saved.
5. Mode Plaintext: without typing `EXPORT PLAINTEXT` → **Check:** blocked. Type it exactly → saved; open file → **Check:** plaintext rows, no custom fields/TOTP columns.
6. Classic: Settings → Interface Classic → CSV Export screen → **Check:** same gap note present (`src/screens/CsvExport.tsx:75-80`).

**M10.3 CSV import**
1. Data Hub → `Import from CSV` → source `Chrome / Chromium export` → choose `%TEMP%\uat\chrome.csv`.
2. Preview. → **Check:** 3 rows shown, editable; nothing saved yet (All Items unchanged).
3. Edit ImportB username cell → `bobby`. Commit Import.
4. → **Check:** `ImportA`, `ImportB` added once each (duplicate skipped, reported); ImportB username `bobby`; type Login.

### M11 — Restyled screens (mk-* tokens)

**M11.1 Visual per screen** — for each of `Health Dashboard`, `Password Generator`, `Data Hub`, `Settings`, `Trash`:
1. Open in dark theme. → **Check:** Make palette (not Classic navy/plain list); headings match (e.g. "Health Dashboard / Security analysis of your vault", "Settings / Preferences and security configuration") 📷.
2. Sidebar bottom sun/moon icon → light theme. → **Check:** readable contrast, no dark-only leftovers 📷.
3. Settings → Appearance → cycle accents `Indigo`,`Sky`,`Teal`,`Purple`,`Pink`. → **Check:** active nav pill + buttons change color each time.
4. Compare with `docs/make-ui-capture/dark|light/*.png` and `figma-make/screenshots/` → record visible diffs.

**M11.2 Health content**
1. Health Dashboard. → **Check:** `Weak Passwords` includes `OldForum` (`password`); `Reused Passwords` groups ShopA/ShopB if both still have same pw (recreate if deleted); `Unsecured Site (http://)` lists `OldForum`; `Old Passwords` 0.
2. Click an entry → **Check:** navigates to that item's detail.
3. `Breached` card: with HIBP off → **Check:** no network call, prompt to enable. Enable in Settings → `Check now` → **Check:** "Checking…" then result; `password` flagged breached. (Only network call in app.)
4. Settings → "old" threshold 90/180/365 → **Check:** saves without error.

**M11.3 Generator**
1. `Password Generator` → Generate. → **Check:** "Generated password" field fills; strength bar + label.
2. Try each method Random/Memorable/Pattern/Pronounceable with options → **Check:** output matches option (e.g. Pattern `Cvc#` → like `Bak4`).
3. Copy → **Check:** 30s clear (as M6.4).
4. Settings → strength algorithm `Simple (entropy)` vs `Advanced (zxcvbn)` → **Check:** meter label may change for `password1`; Health "Weak" unaffected.

**M11.4 Settings persistence**
1. Change each: auto-lock 1 min, theme, accent, mobile nav style, reprompt grace. → **Check:** control briefly disables while saving; values persist after relaunch.
2. Auto-lock 1 min: idle 70 s → **Check:** locked to login screen.

### M12 — Responsive (W: DevTools device toolbar; D: resize window)

**M12.1 Narrow (<768px, e.g. 390px)**
1. → **Check:** top bar with `LocalVault` + `Open menu` icon; sidebar hidden; list full width 📷.
2. `Open menu` → **Check:** sidebar slides over; `Close menu` closes it; selecting a section closes it.
3. Select item → **Check:** detail replaces list; back chevron ("Back to your vault") returns to list.
4. Open Add item drawer → **Check:** usable, no horizontal scroll.

**M12.2 Wide (≥768px)**
1. → **Check:** sidebar + list + detail side by side; no top bar.
2. Resize across 768px boundary with item selected → **Check:** no crash, layout switches cleanly.

### M13 — Regression: Classic, New, Viewer

**M13.1 Classic smoke** (Interface Classic)
1. Add site + credential, search, delete → Trash → restore, generator, change theme. → **Check:** all work as before.
2. Open `GitHub` → **Check:** custom fields/TOTP still show; no errors due to new item_type/folder columns.

**M13.2 New UI** (Interface New)
1. → **Check:** Favorites shows `GitHub`; `Wifi`/`Visa Primary`/`Me` open without crash (render as generic sites).

**M13.3 Viewer app**
1. Launch viewer (per `docs/testing/HOW_TO_TEST_VIEWER_APP.md`). Log in. Search `git`. → **Check:** result returned; before unlock search not possible.

### D-only cases

**D1 Windows Hello** — Settings → Security → enable biometric → setup → Lock → fingerprint/face button → **Check:** unlocks. No hardware → `N/A`. After M9.3 → **Check:** biometric cleared.
**D2 SSH agent** — Enable SSH agent → key `uat` added (M9.0) → run `ssh-add -l` with `SSH_AUTH_SOCK` per `docs/HOW_TO_TEST_SSH_AGENT.md` → **Check:** key fingerprint listed. Delete key → **Check:** master-password prompt required first.
**D3 Window title** — **Check:** title bar text (currently `tauri-app`, `lib.rs:54`) — record, cosmetic.
