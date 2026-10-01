# Product Requirements Document (PRD)

> **Status note (2026-09-30):** Original v1.0 design doc. Shipped scope now also includes item types/folders, biometric + PIN unlock, SSH agent, JSON export/import, per-credential history/change log, and three UI variants (Classic/New/Make). See `docs/HOW_TO_USE_APP.md` for current behavior.

**Product:** Local-First Zero-Knowledge Password Manager
**Status:** Draft v1.0 — approved design, pre-implementation
**Owner:** tpass0960@gmail.com
**Last updated:** 2026-07-01

## 1. Purpose

Give the user a password manager where every credential is stored **only on their own machine**,
protected by strong encryption, and unlockable only with a master password that the software itself
never stores or transmits. No account, no cloud sync, no telemetry. The user should be able to trust
this tool with every password they own.

## 2. Problem Statement

Cloud-based password managers require trusting a third party's servers, are attractive breach targets,
and depend on network access. The user wants full local control: one encrypted file they own, that
they can back up, move, or inspect themselves, with a companion tool to view it if the main app is
ever unavailable.

## 3. Goals

- Store unlimited website/username/password/remarks entries, encrypted at rest, with **zero** plaintext
  persistence of the master password.
- Make everyday use fast: instant search, one-click copy, real-time strength feedback.
- Make trust verifiable: open documentation, a threat model, and a way to inspect that the vault file
  is actually encrypted.
- Ship something usable quickly (M1 desktop MVP), then layer generator, health dashboard, backup/CSV,
  theming, a read-only viewer, and a localhost web mode.

## 4. Non-Goals (out of scope for this product)

- Cloud sync or hosted accounts of any kind.
- Browser extension / autofill (not requested; may be considered later, not in this PRD).
- Android app build (design-only for now; see §9).
- Recovering a forgotten master password (zero-knowledge means this is impossible by design).

## 5. Target User

A single technically-comfortable user (you) running Windows 11, who wants a personal vault and is open
to eventually adding more users (each with their own independent vault) or a phone companion.

## 6. User Stories

1. As a new user, I register a master password once; it becomes the only key to my vault.
2. As a returning user, I log in with my master password and see my saved sites/credentials.
3. As a user, I add a new site with one or more username/password pairs and free-text remarks.
4. As a user, I generate a strong password with a method I choose (random, memorable, pattern,
   pronounceable) and see its strength/entropy before saving.
5. As a user, I search my vault in real time by site, username, or remarks.
6. As a user, I see a health dashboard telling me which passwords are weak, reused, old, or (if I opt
   in) exposed in a known breach — and what to do about it.
7. As a user, I back up my entire vault and restore it in one click, on this machine or another.
8. As a user, I export my vault to CSV — encrypted by default, or plaintext only after an explicit
   typed confirmation — and import credentials from a Chrome/Edge or Firefox CSV export, or map any
   other CSV's columns myself.
9. As a user, if my main app is ever broken or unavailable, I can open a separate read-only viewer,
   enter my master password, and see my data.
10. As a user, I pick a light or dark theme and one of five accent colors, and it's consistent
    everywhere.
11. As a user, I hover any icon/button and get a short tooltip explaining what it does.
12. As a user (future), I access my vault from a browser on `localhost` when I want, without exposing
    it to the network.

## 7. Feature Set by Milestone

| Milestone | Delivers |
|---|---|
| M1 | Register/login, encrypted vault, CRUD, soft-delete trash, real-time search, auto-lock, dark theme, tooltips |
| M2 | Password generator (4 methods), real-time strength + entropy |
| M3 | Encrypted backup/restore, CSV export (encrypted + confirmed-plaintext), CSV import (Chromium/Firefox/generic mapper) |
| M4 | Health dashboard (weak/reused/old + opt-in HIBP breach check), navigation rail, full theming (light/dark × 5 accents) |
| M5 | Read-only viewer app |
| M6 | Localhost-bound web mode, responsive layout |
| Later | Android companion + sync design; macOS/Linux builds |

Full requirement traceability (all 33 originally requested features, de-duplicated) lives in the
approved design plan; nothing was dropped, several overlapping asks were merged (e.g. "very good
encryption," "local only," and "stores encrypted" are one requirement).

## 8. Success Criteria

- A vault file opened outside the app (hex viewer / raw SQLite) shows no readable plaintext.
- Wrong master password never unlocks the vault, with no information leaked about why it failed
  beyond "incorrect."
- Search returns results as you type with no perceptible lag at hundreds of entries.
- Plaintext CSV export cannot happen without an explicit, unambiguous confirmation step.
- Every icon/button has a working tooltip.
- Each milestone has a runnable, demonstrable "Verify" scenario (see TRD/plan).

## 9. Future Scope (documented now, not built)

- **Android companion:** periodically syncs with the desktop vault. Sync transport (Bluetooth vs.
  LAN-discovery vs. QR/manual export-import) is an open decision to make when this phase starts;
  Bluetooth was the original ask but is fiddly and may be replaced by a safer/simpler alternative.
- **macOS/Linux builds:** Tauri supports this with modest per-OS packaging work.

## 10. Risks

- **Crypto correctness is the single highest-risk area** — mitigated by keeping the crypto/vault
  Rust modules small, isolated, and covered by mandatory automated tests (see TRD).
- **Scope creep** — mitigated by the milestone sequencing; later milestones do not block earlier ones
  from being genuinely useful.
- **HIBP opt-in feature is the only network path** — must be clearly labeled and off by default so the
  "local-only" promise isn't accidentally broken.
