# Requirements: Local-First Zero-Knowledge Password Manager

**Defined:** 2026-09-13
**Core Value:** The vault must stay unreadable to anyone without the master password —
zero-knowledge and local-only by default, with no path to recover a forgotten master password.

Source: `docs/PRD.md` (M1-M6, 13 requirements) plus two categories of gap found during brownfield
doc-ingest and codebase mapping on 2026-09-13 — see `.planning/intel/SYNTHESIS.md` for the full
ingest trail. Checkboxes reflect actual shipped state as of this writing (verified against
`.planning/codebase/`), not planning intent.

## v1 Requirements

Requirements for this project. Each maps to exactly one roadmap phase.

### Authentication (AUTH)

- [x] **AUTH-01**: User can register a master password once and log in with it; a wrong password
  always returns one generic "incorrect master password" message, and the master password /
  derived vault key is never stored, logged, or transmitted.
- [x] **AUTH-02**: User can enable Windows Hello biometric unlock (desktop only, opt-in, defaults
  off) as an alternative to typing the master password.

### Vault (VAULT)

- [x] **VAULT-01**: User can add, edit, and soft-delete (trash/restore) a site with one or more
  username/password pairs and free-text remarks.
- [x] **VAULT-02**: User can search the vault in real time by site, username, or remarks, with
  results updating within one UI frame of each keystroke at up to ~5,000 entries.
- [ ] **VAULT-03**: User can create/view Login, Secure Note, Payment Card, and Identity items,
  each with its own icon/color and detail fields, filterable by type with a count badge.
- [ ] **VAULT-04**: User can star any item as a favorite and filter to a "Favorites" view.
- [ ] **VAULT-05**: User can create folders, assign an item to at most one folder, and filter the
  vault by folder.

### Password Generator (GEN)

- [x] **GEN-01**: User can generate a password with a method of choice (random, memorable,
  pattern, pronounceable) and see its strength/entropy before saving.

### Data Portability (DATA)

- [x] **DATA-01**: User can create an encrypted backup of the entire vault and restore it in one
  click, on this machine or another.
- [x] **DATA-02**: User can export the vault to CSV — encrypted by default, plaintext only after
  an explicit typed confirmation — and import credentials from Chrome/Edge, Firefox, or LastPass
  CSV exports, or map any other CSV's columns manually, with a preview step before committing.

### Health (HEALTH)

- [x] **HEALTH-01**: User can view a Health Dashboard showing weak, reused, and old passwords,
  plus an opt-in (default off) Have I Been Pwned breach check that only ever sends a 5-character
  hash prefix.
- [ ] **HEALTH-02**: User can view a per-item health card (strength, breach status, reuse status)
  directly in that item's detail panel, without triggering a full vault-wide sweep.

### UX / Appearance (UX)

- [x] **UX-01**: Every icon/button has a working tooltip, reachable on both mouse hover and
  keyboard focus.
- [x] **UX-02**: User can pick a light or dark theme and one of five accent colors, applied
  consistently everywhere, including a narrow-width (<640px) responsive layout with a choice of
  nav pattern (icon rail / bottom tabs / drawer).

### Viewer (VIEWER)

- [x] **VIEWER-01**: User can open a separate read-only Viewer app, log in with the master
  password, and see/search/copy vault data; the Viewer binary has no add/edit/delete/
  generator/health/backup commands compiled into it at all.

### Web Mode (WEB)

- [x] **WEB-01**: User can access the full feature set from a browser tab at
  `http://127.0.0.1:7373`, which never binds to any interface beyond localhost.

### Platform Integrations (INTEG)

- [x] **INTEG-01**: User can enable an SSH agent over a Windows named pipe (opt-in, default off,
  desktop only) so other applications can use Ed25519/RSA keys held in the vault.

### Security Hardening (SEC)

- [x] **SEC-01**: Web-mode export/import/restore endpoints reject any client-supplied path
  outside the app's own backups/exports directory — no arbitrary file read or write is possible.
- [x] **SEC-02**: Web-mode login/register attempts are throttled (backoff/lockout), and
  concurrent Argon2 derivations are bounded so a request burst cannot exhaust server memory.
- [x] **SEC-03**: The web server sends a strict Content-Security-Policy and
  `X-Content-Type-Options: nosniff` on every response, and the desktop Tauri shell no longer
  ships with `csp: null`.

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Future Scope (FUTURE)

- **FUTURE-01**: Android companion app that periodically syncs with the desktop vault. Sync
  transport (Bluetooth vs. LAN-discovery vs. QR/manual export-import) is an explicitly open
  decision for when this phase starts.
- **FUTURE-02**: macOS/Linux builds — Tauri supports this with modest per-OS packaging work.

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| Cloud sync / hosted accounts | Breaks the zero-knowledge, local-only core value (PRD §4 Non-Goal) |
| Browser extension / autofill | Not requested; may be reconsidered later (PRD §4 Non-Goal) |
| Recovering a forgotten master password | Zero-knowledge means there is nothing to reset, by design (PRD §4 Non-Goal) |
| CSV import/export for Secure Note/Payment Card/Identity items | New item types haven't settled yet; revisit after Phase 8 ships (FIGMA_UI_VS_CURRENT_FEATURES.md, deferred #2) |
| Read-only Viewer support for the new item types | Separate future work (FIGMA_UI_VS_CURRENT_FEATURES.md, deferred #3) |
| Standalone CLI binary | New attack surface, no user story (Bitwarden-comparison research, explicitly not recommended) |
| Folders/collections/org-sharing beyond the single-folder-per-item model | Speculative scope beyond what Phase 8 schedules (Bitwarden-comparison research, explicitly not recommended) |
| PBKDF2 fallback / dual-KDF / RSA org-key wrapping | Solves problems this project doesn't have (Bitwarden-comparison research, explicitly not recommended) |

## Traceability

Which phases cover which requirements.

| Requirement | Phase | Status |
|-------------|-------|--------|
| AUTH-01 | Phase 1 | Complete |
| VAULT-01 | Phase 1 | Complete |
| VAULT-02 | Phase 1 | Complete |
| UX-01 | Phase 1 | Complete |
| GEN-01 | Phase 2 | Complete |
| DATA-01 | Phase 3 | Complete |
| DATA-02 | Phase 3 | Complete |
| HEALTH-01 | Phase 4 | Complete |
| UX-02 | Phase 4 | Complete |
| VIEWER-01 | Phase 5 | Complete |
| WEB-01 | Phase 6 | Complete |
| AUTH-02 | Phase 7 | Complete |
| INTEG-01 | Phase 7 | Complete |
| VAULT-03 | Phase 8 | Pending |
| VAULT-04 | Phase 8 | Pending |
| VAULT-05 | Phase 8 | Pending |
| HEALTH-02 | Phase 8 | Pending |
| SEC-01 | Phase 9 | Complete |
| SEC-02 | Phase 9 | Complete |
| SEC-03 | Phase 9 | Complete |

**Coverage:**

- v1 requirements: 20 total
- Mapped to phases: 20
- Unmapped: 0 ✓

---
*Requirements defined: 2026-09-13*
*Last updated: 2026-09-13 after brownfield doc-ingest and initial roadmap creation*
