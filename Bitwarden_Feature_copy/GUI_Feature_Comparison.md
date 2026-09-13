# Comparative Study: Bitwarden GUI Feature Plan vs. Current Implementation

Compares [`GUI_Feature_Plan.md`](./GUI_Feature_Plan.md) (22 candidate features from Bitwarden's
clients repo) against what's actually built in this repo today. Status verified by reading
`src-tauri/`, `core/`, and `src/` directly (not assumed from the plan).

---

## Status Table

| Feature | Plan Fit | Status | Evidence | Gap |
|---|---|---|---|---|
| Password generator | Easy | **Done** | `core/src/generator.rs:1-349`, `src/screens/Generator.tsx` | none — random/memorable/pattern/pronounceable modes all present |
| Vault export | Easy | **Done** | `core/src/backup.rs:1-30`, `src/screens/CsvExport.tsx`, `src/screens/Backup.tsx` | none |
| Clipboard-clear-on-timeout | Easy | **Done** | `src-tauri/src/commands.rs:195-209` (30s, check-before-clear), `src/lib/clipboard.ts:1-23` | none |
| Vault timeout / auto-lock | Easy–Med | **Done** (idle-timer part) | `core/src/autolock.rs:6-19`, `src/hooks/useActivityWatcher.ts:1-38`, `src/screens/Settings.tsx:25,60-61,167` | no OS sleep/lock-screen hook, inactivity-only |
| Vault import | Medium | **Done** (partial formats) | `core/src/portio.rs:55,135`, `core/src/dto.rs:211-215` (Chromium/Firefox/Generic), `src/screens/CsvImport.tsx` | no LastPass/1Password/KeePass parsers |
| Vault Health Report expansion | Easy–Med | **Partial** | `core/src/health.rs:16-37`, `core/src/commands.rs:531-620`, `src/screens/HealthDashboard.tsx` | Weak/Reused/Old/opt-in HIBP breach all work; **unsecured http:// check missing** |
| Account switching | Medium | **Partial** | `core/src/registry.rs:15-39` (multi-user, one vault file per username) | works as logout→login only; no in-app switcher UI |
| TOTP / authenticator field | Easy | Not started | — | no totp crate, no schema field |
| PIN unlock | Easy | Not started | — | no PIN-wrapped-key path |
| Master-password re-prompt | Easy | Not started | — | no per-item flag, no schema field |
| Custom fields on items | Easy | Not started | — | `credentials` table is fixed-shape (id/site_id/username/secret/remarks) |
| Folders | Easy | Not started | — | `sites` table has no parent/group column |
| Username generator | Easy–Med | Not started | — | no plus-address/catch-all/forwarder logic |
| Biometric unlock | Medium | Not started | — | no OS credential API / keyring crate |
| Account switching (in-app UI) | Medium | see above (Partial) | — | — |
| Vault item attachments | Medium | Not started | — | no blob storage table |
| SSH agent | Med–Hard | Not started | — | no `ssh-agent` crate/protocol code |
| Autotype | Med–Hard | Not started | — | no global hotkey / keystroke injection |
| Passkey/FIDO2 | Hard | Not started | — | no WebAuthn crate |
| Send | Hard | Not started | — | no share/link item type |
| Autofill URI matching | N/A (no browser ext) | Not started | — | out of scope without a browser extension |
| Organizations / Collections | Hard | Not started | — | out of scope, server-dependent |
| Emergency access | Hard | Not started | — | out of scope, server-dependent |

---

## Score

- **Done:** 6 / 22 (password generator, export, clipboard-clear, auto-lock idle-timer, vault import, and — call it done-enough — the core Health Dashboard)
- **Partial:** 2 / 22 (Health Report needs the unsecured-site check; account switching needs an in-app switcher)
- **Not started:** 14 / 22

Of the plan's own "top 3 first" picks (Health Report expansion, TOTP, PIN unlock): Health Report is
**half-done already**, TOTP and PIN unlock are both untouched.

---

## Revised Near-Term Priority (given what's already built)

1. **Unsecured-site check** — smallest possible addition, finishes the Health Report expansion
   the plan already flagged as top priority. Add an `http://` string check next to the existing
   reused-password check in `core/src/health.rs`.
2. **TOTP field** — still the highest-value untouched item; self-contained, no server dependency,
   fits directly into the existing `credentials` schema in `core/src/vault.rs`.
3. **PIN unlock** — reuses the session/lock machinery already built in `core/src/autolock.rs` /
   `session.rs`; just needs a second locally-wrapped-key path alongside master-password auth.
4. **In-app account switcher** — the hard part (per-user vault files via `registry.rs`) already
   exists; this is now a UI-only task (list known usernames, switch without a full logout screen).

Everything else in the plan is unchanged from the original prioritization.
