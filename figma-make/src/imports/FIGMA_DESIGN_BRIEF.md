# Figma Design Brief — LocalVault Password Manager

**Purpose:** give Figma Make everything it needs to design the screens that are missing from the
current prototype, in the visual language the prototype already established.

**How to use this document:** paste it into Figma Make alongside the existing
"Password manager UI design" project, then ask for the screens listed in section 7. Design them one
at a time — a request for ten screens at once produces ten stubs.

**Companion documents.** Two others already exist and were used to seed the first prototype:

- `UI_REDESIGN_CAPABILITY_CHECKLIST.md` — what the backend can do (commands, data shapes)
- `FIGMA_UI_VS_CURRENT_FEATURES.md` — where design and implementation currently disagree

Those describe *capabilities*. This document describes *what to draw*. That distinction is why the
first prototype finished four surfaces and left the rest as placeholders: it knew what the app could
do, but not what each screen should contain or where it belongs on screen.

---

## 1. What the product is

LocalVault is a **desktop password manager that never talks to a network**. The user's vault is a
single encrypted SQLite file on their own machine. There is no server, no account, no cloud, no
sync. A master password is the only key, and there is no recovery path if it is forgotten — by
design.

The practical consequence for design: **every screen is a local, single-user screen.** No sharing,
no collaborators, no team spaces, no "last synced" timestamps, no online/offline indicator, no
profile menu with an email address. Nothing that implies another person or another device.

It also runs as a localhost-only web page and as a stripped-down read-only viewer app, but the
design target is the desktop app.

---

## 2. Constraints that shape the interface

These are not preferences. They are security behaviours already built and shipped, and a design that
ignores them cannot be implemented as drawn.

| Constraint | What it means for the design |
|---|---|
| **Reprompt on reveal** | Individual credentials can be flagged "requires master password". Revealing *or copying* one must first show a master-password prompt. Every reveal and copy control needs a gated variant, and the item needs a visible badge saying it is protected. |
| **Clipboard auto-clears after 30 seconds** | Copy confirmations should say so. "Copied — clears in 30s" is more honest than a bare tick. |
| **Auto-lock** | The vault locks on an idle timeout and immediately when the OS sleeps. The UI can be interrupted by a lock at any moment, from any screen. A lock screen needs to be reachable from every state. |
| **Lazy decryption** | An item's secrets are decrypted only when that item is opened. **Never design a list that shows passwords, strength bars, or per-item health for every row at once** — populating it would require decrypting the whole vault, which defeats the security model. Per-item health belongs in the detail panel only. |
| **Masked secrets do not reveal length** | A hidden password renders as a fixed-width mask, never one dot per character. Do not draw masks that vary with password length. |
| **No network** | No gravatars, no favicon fetching, no remote fonts, no breach badges that imply live lookups. (One exception: an explicitly opt-in breach check the user turns on in Settings.) |

---

## 3. The data model — please design against this shape

The first prototype assumed one item = one username + one password. **The real model is two levels**,
and this is the single most important correction:

```
Site  (name, url, notes)
  └── Credential  (username, password, TOTP, reprompt flag, custom fields, remarks)
  └── Credential  …            ← a site can own several
```

One vault item can legitimately hold **several username/password pairs**. An AWS entry might hold a
daily-driver login and a break-glass root login, each with its own TOTP and its own reprompt flag.

The detail panel must therefore render **a repeating list of credential cards**, not a single fixed
set of fields. The current prototype already handles this correctly (it has both a flat layout and a
nested `credentials[]` layout) — please keep that and carry it into any new screen that edits an item.

**Custom fields** are arbitrary extra fields on a credential, each typed `text`, `hidden` (masked
like a password, and subject to the same reprompt gate), or `boolean`.

### Four item types

The design introduced four types, and they are being built. Each needs its own field set:

| Type | Icon | Fields beyond name/folder/favourite |
|---|---|---|
| Login | key | username, password, TOTP, website, custom fields |
| Secure Note | note | a single long free-text body |
| Payment Card | card | cardholder, number, expiry, CVV, brand |
| Identity | person | full name, and the usual identity-document fields |

---

## 4. Navigation and information architecture

The sidebar structure from the prototype is correct and should be kept:

```
Vault      → All Items · Favorites
Types      → Logins · Secure Notes · Payment Cards · Identities
Folders    → (user-created, flat — no nesting)
Other      → Health Dashboard · Password Generator · Data Hub · Trash
Footer     → Lock · theme toggle · Settings
```

Two changes from the current prototype:

1. **Remove the Sync button.** There is no sync and there never will be — see section 8.
2. **Folders are flat.** No nested folders, no drag-to-reorder. Every item belongs to at most one.

---

## 5. Layout system — and the bug to fix

The three-pane shell is right:

| Pane | Width | Contents |
|---|---|---|
| Sidebar | `220px`, fixed | Navigation |
| List | `280px`, fixed | Item rows, search, add button |
| Detail | fills remaining | The selected item |

**The bug:** in the current prototype, Health Dashboard, Password Generator and Data Hub are rendered
*inside the 280px list pane*, wrapped in a `max-w-2xl` container inside a `h-40` box. They are
unreadable, and they are in the wrong place.

**The fix — please apply this to every screen in section 7:**

> Health Dashboard, Password Generator, Data Hub, Trash and Settings are **full-width screens**. When
> one is open, the sidebar stays, and the list + detail panes are **replaced by a single full-width
> content area**. They are destinations, not item filters.

The one exception is **Trash**, which genuinely is a list of items — it keeps the list + detail
layout, but the detail pane shows Restore and Delete Permanently instead of the normal actions.

---

## 6. Visual language already established

Please reuse these exactly so new screens match the four finished ones. All values are taken from the
existing prototype source.

### Colour

Accent `#175ddc` throughout, with `#3b7ef8` / `#5b9cf6` for accent text on dark and `#1252b8` for
accent text on light.

| Token | Dark | Light |
|---|---|---|
| Base background | `#13151c` | `#f4f5f9` |
| Sidebar | `#1a1d27` | `#ffffff` |
| List pane | `#1e2130` | `#f0f1f6` |
| Card | `#252839` | `#ffffff` |
| Input | `#13151c` | `#f4f5f9` |
| Border | `rgba(255,255,255,0.06)` | `rgba(0,0,0,0.07)` |
| Border hover | `rgba(255,255,255,0.14)` | `rgba(0,0,0,0.14)` |
| Text primary | `#dde1ef` | `#1a1c2a` |
| Text secondary | `#c0c6e0` | `#383c54` |
| Text muted | `#7c82a0` | `#6b6f88` |
| Text faint | `#3a3f58` | `#b0b4cc` |
| Row hover | `rgba(255,255,255,0.03)` | `rgba(0,0,0,0.03)` |
| Row selected | `rgba(23,93,220,0.18)` | `rgba(23,93,220,0.08)` |

Status colours, identical in both themes: green `#2dcc8f`, yellow `#f7b731`, red `#f84e4e`.

**Both themes are required.** Every new screen needs a light and a dark version.

### Type

Inter (400/500/600/700) for UI, JetBrains Mono for passwords, codes and card numbers.

| Role | Style |
|---|---|
| Section label | 10px, semibold, uppercase, `tracking-widest`, faint |
| Field label | 10px, medium, uppercase, `tracking-widest`, faint |
| Body / metadata | 12px |
| Nav item, item title, button | 14px |
| Screen heading | 20px bold |
| Hero value (generated password, health count) | 24–30px |

### Geometry

- Pane headers are **56px** tall with a bottom border
- `8px` radius on nav items, buttons, inputs and small chips
- `12px` radius on cards and panels
- Cards: `16px` padding, 1px border, card background
- Buttons: `16px` horizontal / `8px` vertical padding, accent fill, white text, semibold
- Icons: 13–16px in chrome, 24px in screen headers
- Screen headers use a `48px` rounded-square icon chip filled `accent @ 10%` with an accent icon
- Transitions 150–200ms

### Components that already exist — reuse, do not redesign

Sidebar nav item (with count badge) · item row · search input · copy button · reveal/hide toggle ·
field label + value pair · credential card · tab bar · empty state (centred icon + one line)

---

## 7. Screens to design

Ordered by how much they block the build. **Please design them one at a time.**

### 7.1 Settings — highest priority, largest screen

Full-width. Five collapsible sections. This is the biggest gap: it does not exist at all today, and
it has roughly thirty controls.

| Section | Controls |
|---|---|
| **Security** | Auto-lock timeout · "old password" age threshold · breach-check opt-in (explains it makes a network call) · PIN quick-unlock set/clear · Windows Hello biometric unlock toggle |
| **SSH Agent** | Enable toggle · list of stored keys with label and public key · add key · delete key. *Desktop only — needs a "not available here" state.* |
| **Appearance** | Light/dark theme · accent colour · navigation style for narrow windows |
| **Interface** | UI variant picker (Classic / New / this one) |
| **Vault** | Password strength algorithm · reprompt mode · reprompt grace window |

Needs: a section that is currently unavailable, a destructive action confirm, and a "saved"
acknowledgement pattern.

### 7.2 Add / edit item

Full-screen or large modal — your call, but please pick one and use it consistently.

Must handle: choosing the item type, **adding and removing multiple credentials on one item**,
per-credential TOTP and reprompt flag, adding and removing custom fields with a type picker
(text / hidden / boolean), folder and favourite, and an embedded password generator for filling the
password field.

This is the most structurally demanding screen. The repeating-credential pattern is the crux.

### 7.3 Health Dashboard

Full-width. Five categories: **weak**, **reused**, **old**, **no TOTP configured**, **breached**.

Each category needs a count and an expandable list of the items in it, where each row links to that
item. Breached is opt-in and needs both an "off" state explaining why it is empty, and a loading
state (it performs a privacy-preserving lookup that takes a moment).

The current stub shows four static numbers in a 2×2 grid. The real screen needs drill-down.

### 7.4 Password Generator

Full-width. Four generation methods, each with its own controls:

| Method | Controls |
|---|---|
| Random | length slider, uppercase / lowercase / numbers / symbols toggles, exclude ambiguous characters |
| Memorable | word count, separator |
| Pattern | a template string |
| Pronounceable | length |

Plus: the generated password in large monospace, a regenerate button, a copy button, and a live
strength meter (five segments, using the status colours).

The same panel appears embedded inside the add/edit screen — please design it so it works in both
contexts.

### 7.5 Data Hub

Full-width. Four actions, each needing its own flow, not just a card:

- **Create backup** — encrypted copy, choose a save location
- **Restore backup** — file picker, then a destructive-action confirm, since it replaces the vault
- **Export CSV** — three modes: this-device-only, password-protected, plaintext. The plaintext option
  needs a serious warning; it writes every password in the clear.
- **Import CSV** — a multi-step wizard: pick a file, pick a source format (Chrome/Chromium, Firefox,
  LastPass, or a generic format with manual column mapping), preview the parsed rows, then confirm.
  Needs a duplicate-handling state and a result summary.

### 7.6 Trash

Keeps the list + detail layout. Rows show what was deleted and when. Actions: restore, and purge
everything (destructive confirm). Needs an empty state.

### 7.7 Register / first run

What a brand-new user sees. Master password with confirmation, a strength meter, and — prominently —
an explanation that **there is no recovery if this password is lost**. This screen carries the single
most consequential warning in the product; please give it real weight rather than fine print.

### 7.8 Lock screen states

The unlock screen exists, but needs its other states: wrong password, PIN entry, Windows Hello
prompt, and "locked automatically because you were idle".

### 7.9 Missing states on screens that already exist

- Empty vault, before any item is added
- Search with no results
- An item selected that has several credentials
- A credential that requires a master password to reveal
- The master-password prompt itself
- Toast notifications (copied / saved / error)
- Destructive confirmation dialog

---

## 8. Please do not design

**Sync, and anything implying it.** No sync button, no cloud icon, no "last synced", no device list,
no account or profile menu, no sign-in-with, no sharing, no team or organisation concepts, no
"upgrade to premium".

This is not a scope deferral. The product is structurally incapable of sync — there is no server to
sync to. A sync control in a security product misleads the user about where their data lives, which
is worse than a missing feature.

Also skip: onboarding tours, a dashboard home screen with activity charts, notification centres, and
anything requiring a network call other than the opt-in breach check.

---

## 9. Checklist for each screen you produce

- [ ] Light and dark versions
- [ ] Uses the tokens in section 6, no new colours
- [ ] Inter for UI, JetBrains Mono for secrets and codes
- [ ] Full-width for destination screens; list + detail only for item collections
- [ ] Empty, loading and error states included
- [ ] Destructive actions have a confirmation step
- [ ] No sync, no accounts, no network
- [ ] Secret masks are fixed-width and do not reveal length
- [ ] Reveal and copy controls have a master-password-gated variant
