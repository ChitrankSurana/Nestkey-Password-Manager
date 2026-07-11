# M1 Phase B — Manual UI Verification Walkthrough

**Why this exists:** everything else in Phase B (85/85 automated tests, per-task and per-slice
spec-compliance/code-quality reviews, a full cross-module review) was verified by a subagent or an
automated command. None of those can drive a live Tauri desktop window — there is no visual/UI test
harness in this repo (no Playwright, no `*.test.tsx`, no `npm test` script). This checklist is the
one remaining Task 28 step (plan doc `docs/superpowers/plans/2026-07-05-m1-phase-b.md`, `## Task 28:`
Step 2) and it requires a human actually running the app. TRD §7 explicitly allows this class of
UI/theme/tooltip/session behavior to be "lighter/manual acceptable" rather than automated.

**How to use this doc:** work through the steps in order — later steps build on state created by
earlier ones (a registered user, an added site/credential). Each step has an **Action** and an
**Expected result**. If a step doesn't match its expected result, stop and note exactly what
happened (screen, what you clicked, what you saw) before continuing — that's a real finding, not
something to work around.

## Setup

1. Open a terminal in the worktree root: `E:\Coding_Projects-CS\Account_1\Password_Manager\.worktrees\m1-phase-b`
2. Every command needs Rust/Perl on PATH first (fresh shells don't have it):
   ```powershell
   $env:PATH = "$env:USERPROFILE\.cargo\bin;C:\Strawberry\perl\bin;C:\Strawberry\c\bin;" + $env:PATH
   ```
3. Start the app: `npm run tauri dev`
4. Wait for the Tauri window to open. First launch creates `~/.pwmgr/` (registry + config); if
   you've run this before and want a truly clean first-run test, close the app and delete
   `%USERPROFILE%\.pwmgr\` first — otherwise you'll land on whatever state a prior run left.

---

## 1. Registration and generic-error login security

**Why this matters:** TRD §3.2 / THREAT_MODEL require that a wrong password never reveals whether
the *username* or the *password* was wrong — only ever one generic message.

1. **Action:** On the Register screen, enter a username (e.g. `alice`) and a master password (e.g.
   `correct horse battery staple`). Click **Register**.
   **Expected:** You land on the Vault screen (empty — no sites yet). No error shown.
2. **Action:** Click **Lock** (top of Vault) to return to Login. Enter username `alice` with the
   **wrong** password. Click **Log in**.
   **Expected:** Exactly one message: **"Incorrect master password."** — no hint about which part
   (username vs. password) was wrong, no different message for "user doesn't exist" vs. "wrong
   password" (try a nonexistent username too, e.g. `nobody`, and confirm you get the *same* generic
   message, not a different one).
3. **Action:** Log in again with the correct username/password.
   **Expected:** Returns to the (still-empty) Vault screen.

## 2. Adding a site and credential, then live search

1. **Action:** Click **Add site**. Fill in a name (e.g. `GitHub`), a URL (optional), and notes
   (optional). Save.
   **Expected:** The new site appears in the Vault's site list immediately.
2. **Action:** Open the site, click **Add credential**. Fill in a username, a secret/password, and
   remarks. Save.
   **Expected:** The credential appears under that site immediately.
3. **Action:** Go back to Vault, type into the search box — try the site name, then clear it and try
   the credential's username, then clear it and try a word from the remarks.
   **Expected:** Results update **live, within one keystroke** for all three search types (TRD
   NFR-3 — no perceptible lag, no need to press Enter or click a search button).

## 3. Editing a credential

1. **Action:** Open the site, open the credential, click **Edit**, change the username or remarks,
   Save.
   **Expected:** The change is reflected immediately in the site's credential list — no stale data,
   no need to navigate away and back.

## 4. Delete → Trash → Restore → Empty Trash

1. **Action:** Open the credential again, click **Delete**.
   **Expected:** It disappears from the site's credential list immediately.
2. **Action:** Go back to Vault, click **Trash**.
   **Expected:** The deleted credential appears in Trash, correctly labeled with its site's name.
3. **Action:** Click **Restore** on it.
   **Expected:** It reappears in the site's credential list; Trash no longer shows it.
4. **Action:** Delete the same credential again, go to Trash, click **Empty Trash** (confirm the
   `window.confirm()` dialog).
   **Expected:** Trash is now empty, and the credential can no longer be restored (it's gone for
   good — this is the one destructive, unconfirmed-at-the-backend operation in the app, by design;
   the confirmation dialog is the only safety net).

## 5. Settings — auto-lock, and confirming it actually takes live effect

**Why this matters:** this is the single most safety-relevant behavior Slice 4 added — a user who
shortens their auto-lock timeout for security needs it to apply to the *running* session, not just
on the next login.

1. **Action:** From Vault, click **Settings**. Change the auto-lock dropdown to **1 minute**.
   **Expected:** The dropdown reflects "1 minute" immediately; no error shown; the control is
   briefly disabled (grayed out) while the change is saving, then re-enabled.
2. **Action:** Click **Back** to return to Vault. **Do not touch the mouse or keyboard again — wait
   more than 1 minute.**
   **Expected:** Without any further interaction, the app automatically returns to the Login screen
   on its own once the idle timeout elapses. This proves the live `Session::set_timeout` reconfigure
   path actually took effect on the already-running session (not just persisted to disk for next
   time).
3. **Action:** Log back in, go to Settings, confirm the dropdown still shows **1 minute** (proves
   the value round-trips through `get_config` correctly after a restart of the session).
   Optionally: change it back to 5 or 15 minutes so the rest of this walkthrough isn't interrupted
   by another auto-lock.

## 6. Manual Lock button

1. **Action:** From Vault, click **Lock**.
   **Expected:** Returns to the Login screen immediately — no delay, no confirmation dialog.

## 7. Dark theme consistency

1. **Action:** Log back in and click through every screen: Vault, Add Site, a site's detail view,
   Add/Edit Credential, Trash, Settings.
   **Expected:** Every screen renders in the dark theme throughout — no flash of unstyled content,
   no screen or modal that's accidentally left in a default light background.

## 8. Keyboard-accessible tooltips (TRD NFR-6)

**Why this matters:** Task 27/27.5 replaced every button's native `title=` (hover-only, unreliable
on keyboard focus in most webviews) with a custom `Tooltip` component specifically so keyboard users
get the same affordance mouse users do.

1. **Action:** On each screen, use **Tab** (not the mouse) to move focus through every button and
   the Settings auto-lock `<select>`. Don't click — just Tab and pause on each control.
   **Expected:** A small dark tooltip bubble appears above each focused control, showing text that
   describes what it does (e.g. "Lock the vault now", "Add a new site", "Move this credential to
   Trash"). It should disappear when you Tab away (on blur).
2. **Action:** Now hover the mouse (without clicking) over a few of the same buttons.
   **Expected:** The same tooltip appears on hover too, and disappears on mouse-leave.
3. **Action:** Try hovering over a **disabled** control specifically — e.g. click Settings' dropdown
   to trigger a save (briefly disabling it) and try to hover it during that brief window, or hover
   the Login/Register submit button while `submitting` is true (hard to catch — it's fast; a slow
   network isn't simulated here, so this may not be practically observable, which is fine).
   **Expected (best-effort, not a hard requirement):** The tooltip should still show text explaining
   the disabled control, matching how native `title=` used to behave on disabled elements. This was
   a specific regression Task 27.5 fixed by moving the hover listeners off the (possibly-disabled)
   child element onto its wrapper — if you can catch a disabled control in this state and the
   tooltip doesn't appear, that's worth reporting, but the timing window is narrow enough that this
   step is genuinely difficult to observe manually.
4. **Action:** Confirm no button anywhere still shows the plain browser-native tooltip style (small
   yellow/gray OS tooltip after a ~1 second hover delay) — every button's tooltip should be the
   custom dark styled one, appearing instantly.

---

## If something doesn't match

Note exactly: which step, which screen, what you did, what you expected, what you actually saw
(and a screenshot if easy to grab). That's a real, reportable finding — everything up to this
checklist was verified by automated tests and code review, but none of those can see what actually
renders on screen, so this walkthrough is the last line of defense for anything purely visual or
timing-dependent.

## If everything matches

Phase B's manual verification is complete. Nothing further is required before considering M1 Phase B
fully done from a QA perspective — the plan doc's Task 28 Step 2 checklist (which this document
mirrors) can be checked off.
