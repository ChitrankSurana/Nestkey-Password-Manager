# Requirements (from PRD-type documents)

Single source for this entire file: `docs/PRD.md` (Product Requirements Document, "Draft v1.0 —
approved design, pre-implementation", last updated 2026-07-01, owner tpass0960@gmail.com). This was
the only document in the ingest set classified `type: PRD`, so there are no competing-variant
requirements to reconcile — every entry below traces to a single, internally-consistent source.

See `INGEST-CONFLICTS.md` → `[INFO] PRD.md predates several already-shipped or in-progress features`
for a note on features (biometric unlock, SSH agent, new item types) that exist in other docs but
are absent from this PRD.

---

## REQ-master-password-registration-login
- source: docs/PRD.md
- description: As a new user, register a master password once; it becomes the only key to the vault
  (User Story 1). As a returning user, log in with the master password to see saved sites/credentials
  (User Story 2). Zero plaintext persistence of the master password (§3 Goals); the software never
  stores or transmits the master password (§1 Purpose). Recovering a forgotten master password is
  explicitly impossible by design — zero-knowledge means there is nothing to reset (§4 Non-Goals).
- acceptance: A vault file opened outside the app (hex viewer / raw SQLite) shows no readable
  plaintext. Wrong master password never unlocks the vault, with no information leaked about why it
  failed beyond "incorrect." (§8 Success Criteria)
- scope: M1 — Register/login, encrypted vault (§7 Feature Set by Milestone).

## REQ-vault-crud
- source: docs/PRD.md
- description: As a user, add a new site with one or more username/password pairs and free-text
  remarks (User Story 3).
- acceptance: absent — no distinct §8 criterion beyond the general "no perceptible lag" search
  criterion and the vault-encryption criterion under REQ-master-password-registration-login.
- scope: M1 — encrypted vault, CRUD, soft-delete trash (§7).

## REQ-password-generator
- source: docs/PRD.md
- description: As a user, generate a strong password with a method I choose (random, memorable,
  pattern, pronounceable) and see its strength/entropy before saving (User Story 4).
- acceptance: absent — no distinct §8 criterion for this story specifically.
- scope: M2 — Password generator (4 methods), real-time strength + entropy (§7).

## REQ-live-search
- source: docs/PRD.md
- description: As a user, search the vault in real time by site, username, or remarks (User Story 5).
- acceptance: Search returns results as you type with no perceptible lag at hundreds of entries. (§8)
- scope: M1 (§7).

## REQ-health-dashboard
- source: docs/PRD.md
- description: As a user, see a health dashboard telling which passwords are weak, reused, old, or
  (if opted in) exposed in a known breach — and what to do about it (User Story 6).
- acceptance: absent — no distinct §8 criterion for this story specifically.
- scope: M4 — Health dashboard (weak/reused/old + opt-in HIBP breach check) (§7). §10 Risks flags
  the HIBP opt-in check as "the only network path" and requires it be clearly labeled and off by
  default so the local-only promise isn't accidentally broken.

## REQ-backup-restore
- source: docs/PRD.md
- description: As a user, back up the entire vault and restore it in one click, on this machine or
  another (User Story 7).
- acceptance: absent — no distinct §8 criterion for this story specifically.
- scope: M3 — Encrypted backup/restore (§7).

## REQ-csv-export-import
- source: docs/PRD.md
- description: As a user, export the vault to CSV — encrypted by default, or plaintext only after an
  explicit typed confirmation — and import credentials from a Chrome/Edge or Firefox CSV export, or
  map any other CSV's columns myself (User Story 8).
- acceptance: Plaintext CSV export cannot happen without an explicit, unambiguous confirmation step.
  (§8)
- scope: M3 — CSV export (encrypted + confirmed-plaintext), CSV import (Chromium/Firefox/generic
  mapper) (§7).

## REQ-readonly-viewer
- source: docs/PRD.md
- description: As a user, if the main app is ever broken or unavailable, open a separate read-only
  viewer, enter the master password, and see the data (User Story 9).
- acceptance: absent — no distinct §8 criterion for this story specifically.
- scope: M5 — Read-only viewer app (§7).

## REQ-theming
- source: docs/PRD.md
- description: As a user, pick a light or dark theme and one of five accent colors, and it's
  consistent everywhere (User Story 10).
- acceptance: absent — no distinct §8 criterion for this story specifically.
- scope: M4 — full theming (light/dark × 5 accents) (§7).

## REQ-tooltips
- source: docs/PRD.md
- description: As a user, hover any icon/button and get a short tooltip explaining what it does
  (User Story 11).
- acceptance: Every icon/button has a working tooltip. (§8)
- scope: M1 (§7).

## REQ-localhost-web-mode
- source: docs/PRD.md
- description: As a user (future), access the vault from a browser on localhost when desired, without
  exposing it to the network (User Story 12).
- acceptance: absent — no distinct §8 criterion for this story specifically; §8 does note "each
  milestone has a runnable, demonstrable 'Verify' scenario (see TRD/plan)" as a general criterion.
- scope: M6 — Localhost-bound web mode, responsive layout (§7).

## REQ-android-companion-future
- source: docs/PRD.md
- description: Android companion app that periodically syncs with the desktop vault (§9 Future
  Scope). Sync transport (Bluetooth vs. LAN-discovery vs. QR/manual export-import) is explicitly an
  open decision to be made when this phase starts; Bluetooth was the original ask but "is fiddly and
  may be replaced by a safer/simpler alternative."
- acceptance: absent — documented now, not built (§9).
- scope: "Later" row of §7 Feature Set — Android companion + sync design.

## REQ-macos-linux-builds-future
- source: docs/PRD.md
- description: macOS/Linux builds — "Tauri supports this with modest per-OS packaging work." (§9)
- acceptance: absent — documented now, not built (§9).
- scope: "Later" row of §7 Feature Set.

---

## Non-Goals (explicitly out of scope, PRD §4 — not requirements, recorded for traceability)

- source: docs/PRD.md
- Cloud sync or hosted accounts of any kind.
- Browser extension / autofill (not requested; may be considered later, not in this PRD).
- Android app build (design-only for now; see REQ-android-companion-future).
- Recovering a forgotten master password (zero-knowledge means this is impossible by design).
