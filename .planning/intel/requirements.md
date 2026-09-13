# Requirements (from PRDs)

Single PRD source for this ingest: `docs/PRD.md` ("Product Requirements Document: Local-First
Zero-Knowledge Password Manager", Status: Draft v1.0 — approved design, pre-implementation, owner
tpass0960@gmail.com, last updated 2026-07-01). No competing PRDs were found, so no competing
acceptance variants exist for this pass.

**Non-Goals (PRD §4, out of scope for this product):** cloud sync/hosted accounts; browser
extension/autofill; Android app build (design-only, see REQ-android-companion); recovering a
forgotten master password (impossible by design — zero-knowledge).

**Note for roadmapper:** other ingested docs (see `context.md` topic "Figma UI vs Current Features"
and topic "Testing Guides — Biometric Unlock / SSH Agent") describe shipped features — Windows Hello
biometric unlock, SSH agent (named pipe), PIN quick-unlock, LastPass CSV import — that have **no**
corresponding entry in this PRD and no ADR/SPEC of their own in this ingest set. See
`INGEST-CONFLICTS.md` WARNING "Shipped features have no PRD/ADR/SPEC provenance."

---

## REQ-local-only-vault

- source: docs/PRD.md
- description: Every credential is stored only on the user's own machine, protected by strong
  encryption, unlockable only with a master password the software never stores or transmits. No
  account, no cloud sync, no telemetry.
- acceptance: A vault file opened outside the app (hex viewer / raw SQLite) shows no readable
  plaintext (PRD §8 Success Criteria). Zero network calls unless the user explicitly opts into the
  HIBP breach check.
- scope: encrypted vault storage, zero-knowledge architecture, master password

## REQ-registration-and-login

- source: docs/PRD.md
- description: As a new user, register a master password once; it becomes the only key to the
  vault. As a returning user, log in with the master password and see saved sites/credentials.
  (User stories 1–2.)
- acceptance: Wrong master password never unlocks the vault, with no information leaked about why
  it failed beyond "incorrect." (PRD §8.)
- scope: registration, login, authentication, master password

## REQ-vault-crud

- source: docs/PRD.md
- description: Add a new site with one or more username/password pairs and free-text remarks.
  (User story 3.)
- acceptance: absent (not separately specified beyond the user story itself in this PRD)
- scope: vault CRUD, sites, credentials, remarks

## REQ-password-generator

- source: docs/PRD.md
- description: Generate a strong password with a chosen method (random, memorable, pattern,
  pronounceable) and see its strength/entropy before saving. (User story 4; delivered M2 per §7
  milestone table.)
- acceptance: absent (see constraints.md for the TRD/SPEC-level acceptance detail)
- scope: password generator, strength/entropy estimation

## REQ-search

- source: docs/PRD.md
- description: Search the vault in real time by site, username, or remarks. (User story 5.)
- acceptance: Search returns results as you type with no perceptible lag at hundreds of entries.
  (PRD §8.)
- scope: real-time search

## REQ-health-dashboard

- source: docs/PRD.md
- description: A health dashboard telling the user which passwords are weak, reused, old, or (if
  opted in) exposed in a known breach — and what to do about it. (User story 6; delivered M4 per §7
  milestone table.)
- acceptance: absent (see constraints.md for the SPEC-level acceptance detail, including a flagged
  conflict over "weak" classification — see INGEST-CONFLICTS.md)
- scope: health dashboard, weak/reused/old detection, HIBP breach check (opt-in)

## REQ-backup-restore

- source: docs/PRD.md
- description: Back up the entire vault and restore it in one click, on this machine or another.
  (User story 7; delivered M3 per §7 milestone table.)
- acceptance: absent (not separately specified beyond the user story itself in this PRD)
- scope: encrypted backup, restore

## REQ-csv-export-import

- source: docs/PRD.md
- description: Export the vault to CSV — encrypted by default, or plaintext only after an explicit
  typed confirmation — and import credentials from a Chrome/Edge or Firefox CSV export, or map any
  other CSV's columns manually. (User story 8; delivered M3 per §7 milestone table.)
- acceptance: Plaintext CSV export cannot happen without an explicit, unambiguous confirmation step.
  (PRD §8.)
- scope: CSV export, CSV import, plaintext-export confirmation gate

## REQ-readonly-viewer

- source: docs/PRD.md
- description: If the main app is ever broken or unavailable, open a separate read-only viewer,
  enter the master password, and see the data. (User story 9; delivered M5 per §7 milestone table.)
- acceptance: absent (see constraints.md, M5 SPEC, for the compile-time-enforced read-only
  guarantee)
- scope: read-only viewer app, emergency access

## REQ-theming

- source: docs/PRD.md
- description: Pick a light or dark theme and one of five accent colors, consistent everywhere.
  (User story 10; delivered M4 per §7 milestone table.)
- acceptance: absent (not separately specified beyond the user story itself in this PRD)
- scope: theming, light/dark mode, accent colors

## REQ-tooltips

- source: docs/PRD.md
- description: Hover any icon/button and get a short tooltip explaining what it does. (User story
  11.)
- acceptance: Every icon/button has a working tooltip. (PRD §8.)
- scope: tooltips, accessibility

## REQ-web-mode

- source: docs/PRD.md
- description: Access the vault from a browser on `localhost` when wanted, without exposing it to
  the network. (User story 12, marked "future" in the PRD; delivered M6 per §7 milestone table.)
- acceptance: absent (see constraints.md, M6 web-mode SPEC, for the session/CSRF/bind-address
  acceptance detail)
- scope: localhost web mode, browser access

## REQ-android-companion

- source: docs/PRD.md
- description: Future scope, documented but not built (PRD §9). Periodically syncs with the desktop
  vault. Sync transport (Bluetooth vs. LAN-discovery vs. QR/manual export-import) is an explicitly
  open decision to make when this phase starts.
- acceptance: absent
- scope: Android companion, future sync (not built)

## REQ-macos-linux-builds

- source: docs/PRD.md
- description: Future scope, documented but not built (PRD §9). Tauri supports this with modest
  per-OS packaging work.
- acceptance: absent
- scope: macOS/Linux builds (not built; confirmed still not done as of docs/KNOWN_LIMITATIONS.md §4)
