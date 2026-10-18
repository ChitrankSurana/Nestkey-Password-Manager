# User Testing Feedback Plan (2 users) — 2026-10-18


## Context
Two users did live testing of the Make UI (`src/makeui/*`, React 19 + Tauri 2 + Rust core with SQLCipher). They reported 10 items. This plan fixes 9 of them. Decisions below are frozen. **U1-5 (multiple labelled passwords) is deferred.**

Scope rule: changes go in the Make UI **add drawer AND edit screens** (`ItemDrawer` edit mode + shared `src/screens/CredentialEditor.tsx`). Classic UI screens are out of scope, except `Register.tsx`, which Make reuses.

## Frozen decisions

| # | Change | Where |
|---|---|---|
| U1-1 | Folder field gets an inline "+ New folder" control: a name input + Create button. It calls `api.addFolder(name)` (`src/lib/api.ts:356`), refreshes the folder list, and selects the new folder. Folders have only a name (`folders(id, name UNIQUE COLLATE NOCASE)`), so the inline form is complete. Show a duplicate-name error. | `ItemDrawer.tsx:226-240` (add + edit) |
| U1-2 | Label "Username (optional)" → "Username". The username becomes **required**: Save is disabled or blocked with an error when the username is blank. | `ItemDrawer.tsx:256`, `CredentialEditor.tsx:142` |
| U1-3 | Make the History and Activity timestamps near-white in dark mode, and convert raw UTC `datetime('now')` strings to local time. Use a new readable class/token, not `--mk-text-faint` (that token is used everywhere). Add a small `formatUtcLocal()` helper and a unit test. | `CredentialBlock.tsx:245,259`, `src/makeui/makeui.css` |
| U1-4 | Dropdown label "Hidden" → "Hidden Text". The stored value `hidden` stays unchanged. | `ItemDrawer.tsx:310`, `CredentialEditor.tsx:234` |
| U1-5 | **DEFERRED by the user.** Not in this plan. Options recorded for later: A (label column per credential row) or B (`credential_passwords` sub-table). | — |
| U1-6 | Wrap `PasswordGeneratorPanel` in a collapsible section headed "Generate Random Password". It is collapsed by default and resets to collapsed each time the screen opens. | `ItemDrawer.tsx:263`, `CredentialEditor.tsx:172` (currently opens by default on add) |
| U2-1 | Register screen: add an (i) tooltip icon on the username label saying "Username is case-sensitive". **Bug fix:** registration refuses a username that differs only by case from an existing one, because Windows filenames ignore case (`alice.vault` vs `Alice.vault`). Add a Rust test. | `src/screens/Register.tsx:37-47`, backend register path |
| U2-2 | Four new Settings toggles, **all ON by default**: require the master password on (a) Move item to Trash, (b) Delete one credential, (c) Empty Trash, (d) Delete folder. When a toggle is ON, the action prompts via `MasterPasswordPrompt`, ignoring the reprompt grace window and any per-item bypass. Reuse the pattern from the SSH-key delete (`Settings.tsx:183` `requireMasterPassword`). Add the toggles with the local `Toggle`/`Field` components (`Settings.tsx:90-105`). | `CredentialBlock.tsx:114-127`, `DetailPanel.tsx:81-91`, `ItemDrawer.tsx:176-189`, `TrashView.tsx:56-69`, `Sidebar.tsx:82-93`, `core/src/config.rs` |
| U2-3 | Add the "Don't require master password to view" checkbox to the Secure Note, Payment Card and Identity tabs. Replace the hard-coded `false` with state. | `ItemDrawer.tsx:145,162,164` (copy `:333-336`) |
| U2-4 | Pass an `initialType` from the current `nav` to `ItemDrawer`. Logins/Secure Notes/Payment Cards/Identities open the matching tab. All/Favorites/folders open the Login tab. A folder view preselects that folder; the Favorites view preselects Favorite. Add a pure mapping function with a unit test. | `MakeVaultShell.tsx:34,153,193`, `ItemDrawer.tsx:44-46` |

## Verification
- `npm test` (Vitest): new tests for `formatUtcLocal`, the nav→initialType mapping, and the delete-policy helper.
- `cargo test --workspace`: test for the case-variant username rejection, and for any new config keys.
- Manual check in `npm run tauri:dev` with UI set to "make" (dark mode): walk all 10 items. Test each delete type with its toggle ON and OFF, and with the reprompt grace window active.
- No CI dispatch unless the user asks for a PR (project rule).
