# Figma UI design vs. current feature set

A feature-by-feature comparison between the vault UI designed in Figma Make
("Password manager UI design") and the features this application currently
implements.

The Figma design was produced independently of the codebase, so it assumes
some capabilities the backend does not have, and omits several that the
application already ships. This document records both directions of that gap
and explains why each one exists.

Status column meanings:

- **Closing** — scheduled for implementation as part of the new-UI port.
- **Open** — deliberately deferred; see "Deferred items" at the bottom.
- **Aligned** — already equivalent on both sides.

---

## 1. In the Figma design, missing or different in the application

| Feature | What the design shows | What the application has today | Why the gap exists | Status |
|---|---|---|---|---|
| Item types | Four distinct types — Login (key icon, blue), Secure Note (note icon, green), Payment Card (card icon, orange), Identity (person icon, purple). Each type has its own icon colour, sidebar filter with a count badge, and its own set of detail fields. | Logins only. The vault stores `sites` (name/url/notes), each owning one or more `credentials` (username, encrypted secret, custom fields, optional TOTP). There is no type discriminator anywhere in the schema. | The schema was built around the single use case of storing website logins. Nothing in it distinguishes one kind of entry from another. | Closing |
| Favourites | A star toggle on each list row and in the detail header. The sidebar has a "Favorites" entry that filters to starred items and shows a count. | No concept of a favourite or starred item. | No column exists to hold the flag, and no command exists to set it. | Closing |
| Folders | A collapsible "Folders" section in the sidebar listing user-created folders (e.g. Development, Finance, Design). Each folder acts as a filter, and every item belongs to at most one folder. | No folders. Entries are a flat list, organised only by the search box. | No folders table and no folder reference on any entry. | Closing |
| Per-item password health | The detail panel shows a health card for the selected item: a five-segment strength meter, "Exposed in breaches: None found", and "Reused: Unique". | Health analysis exists but is **vault-wide only**. The Health screen sweeps every credential and returns categorised lists (weak, reused, old, unsecured, breached). There is no way to ask about a single credential. | The existing report decrypts the whole vault in one pass, which is the wrong shape for a panel that re-renders on every selection. | Closing |
| Sync | A "Sync" button in the sidebar footer, implying remote synchronisation. | Nothing. The application is entirely local: an encrypted SQLite (SQLCipher) file on disk, with no server, no account system, and no network sync path. | Implementing this means designing a sync service, an authentication system, and a conflict-resolution strategy — a project in its own right, not a UI feature. | Open |
| Item granularity | Every vault entry is one flat row carrying a single username and a single password. | An entry is two-level: a **site** owns **one or more credentials**. A single site can legitimately hold several separate username/password pairs. | The design was drawn against a simpler mental model. The real model is richer, and the UI has to accommodate it rather than the reverse. | Closing (UI adapts) |
| TOTP presentation | A static box reading "TOTP key configured", with explanatory text. | A live authenticator display: the current six-digit code, refreshed every second, with the seconds remaining and a copy button. | Here the application is ahead of the design. The port keeps the working implementation rather than the mockup's placeholder. | Aligned (app is ahead) |

---

## 2. In the application, absent from the Figma design

These are all shipping features with no counterpart in the mockup. Each one
needs a home in the new UI.

| Feature | What it does | Where it must land in the new UI |
|---|---|---|
| Health dashboard | Reports weak, reused, and old passwords (against a configurable age threshold), entries with no TOTP configured, and — if the user opts in — passwords found in known breaches via a Have I Been Pwned k-anonymity lookup. Each result links back to the entry it refers to. | A sidebar entry in the "Other" section. |
| Password generator | A standalone screen with a length slider, character-class toggles, an exclude-ambiguous-characters option, and live regeneration. The same panel is also embedded in the credential editor. | A sidebar entry in the "Other" section; the embedded panel is unaffected. |
| Data: backup, CSV export, CSV import | A hub screen offering a full encrypted vault backup and restore, a CSV export, and a CSV import that understands both the native format and LastPass exports. | A sidebar entry in the "Other" section. |
| Settings — security | Auto-lock timeout, the age threshold that feeds the Health screen's "old password" category, the breach-check opt-in, PIN quick-unlock, and Windows Hello biometric unlock. | The existing Settings screen, reached from the sidebar footer. |
| Settings — SSH agent | Runs an SSH agent over a Windows named pipe so other applications can use keys held in the vault. Desktop build only. | Settings, unchanged. |
| Settings — appearance | Light/dark theme, a five-option accent colour picker, and a choice of navigation style for narrow windows. | Settings. Note the design assumes one fixed accent colour, so the picker's future needs a decision. |
| Settings — strength algorithm | Chooses between a fast local entropy estimate and a more accurate backend analysis. | Settings, unchanged. |
| Login screen | Master password entry, plus optional PIN quick-unlock and Windows Hello biometric unlock. | Unchanged — the design begins after unlock and does not cover this screen. |
| Reprompt on reveal | A per-credential flag. When set, revealing or copying that secret requires re-entering the master password first. | Must be carried into the new detail panel; the design's reveal and copy buttons are unconditional. |
| Custom fields | Arbitrary extra fields on a credential, typed as text, hidden (masked like a password), or boolean. | Must be added to the new detail panel and editors. |
| Clipboard auto-clear | Copied secrets are cleared from the clipboard after 30 seconds if untouched. | Must be preserved — the design's copy button uses the raw browser clipboard API with no clearing. |
| Auto-lock on suspend | The vault locks immediately when the operating system reports it is going to sleep, independently of the idle timer. | Background behaviour; unaffected by the UI port. |
| Multiple credentials per site | A site can hold several username/password pairs, each with its own custom fields and TOTP. | The new detail panel must list them all. |

---

## 3. Present and equivalent on both sides

| Feature | In the design | In the application |
|---|---|---|
| Search | A filter box above the item list, matching as the user types. | A live query across site names and usernames, issued on every keystroke. |
| Reveal / hide a secret | An eye icon toggling between masked and plain text. | The same, with the additional reprompt gate described above. |
| Copy to clipboard | A copy icon with a brief confirmation tick. | The same, plus the 30-second clipboard clear. |
| Light / dark theme | A sun/moon toggle in the sidebar footer. | The same, persisted to the user's configuration. |
| Item detail panel | The selected item's fields, with edit and delete actions in the header. | The same, opening the existing editor dialogs. |
| Adding an entry | A "+" button in the list header. | Equivalent actions for adding a site and adding a credential. |

---

## Deferred items

The following gaps are **not** being closed in this pass:

1. **Sync.** Out of scope for the reasons given above. The button is removed
   from the ported UI rather than shown as a disabled placeholder, so the
   interface does not advertise a capability that does not exist.
2. **CSV import and export for the new item types.** Import and export
   continue to cover logins only. Secure notes, payment cards, and identities
   will not appear in an export, and cannot be imported. Worth revisiting once
   the new types have settled.
3. **The read-only viewer build.** The viewer application exposes a small
   read-only subset of commands and will not display secure notes, payment
   cards, or identities. Extending it is a separate piece of work.
4. **The accent colour picker.** The design specifies a single fixed accent
   colour, while the application offers five. The new UI keeps the existing
   picker working for now; whether to keep, reduce, or drop it is an open
   design question.
