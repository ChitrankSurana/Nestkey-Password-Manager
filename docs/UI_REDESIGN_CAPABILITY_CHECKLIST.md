# UI Redesign Capability Checklist

Ground-truth floor for a redesigned vault UI: everything the current Rust/Tauri
backend (`pwmgr-core`) already exposes, and everything the current frontend
already ships a screen for. Scoped to *today's* backend only — no new backend
work assumed. Derived from a full read of `core/src/`, `src-tauri/src/commands.rs`,
`web-src-tauri/src/handlers.rs`, `viewer-src-tauri/`, and `src/`.

## 1. Screens/flows that must exist somewhere in the redesign

| Flow | Backend commands it needs | Currently shipped as |
|---|---|---|
| Register | `register_user` | `Register.tsx` |
| Login (password, PIN, biometric) | `login`, `has_pin`, `unlock_with_pin`, `has_biometric`, `unlock_with_biometric` | `Login.tsx` |
| Vault list + search | `list_sites`, `search` | `Vault.tsx` |
| Item detail (view/reveal/copy) | `list_credentials_for_site`, `verify_master_password`, `copy_to_clipboard` | `SiteDetail.tsx` + `SecretField.tsx` |
| Add/edit site | `add_site`, `update_site`, `soft_delete_site` | `SiteEditor.tsx` |
| Add/edit credential (custom fields, TOTP, reprompt) | `add_credential`, `update_credential`, `set_totp_secret`, `clear_totp_secret` | `CredentialEditor.tsx` |
| Password generator | `generate_password`, `analyze_strength` | `Generator.tsx` + `PasswordGeneratorPanel.tsx` |
| Health dashboard (weak/reused/old/unsecured/breached) | `health_report`, `check_breach` | `HealthDashboard.tsx` |
| Trash (restore/purge) | `list_trash`, `restore_credential`, `purge_trash` | `Trash.tsx` |
| Data hub: backup/restore | `create_backup`, `restore_backup` | `Backup.tsx` |
| Data hub: CSV export | `export_csv` | `CsvExport.tsx` |
| Data hub: CSV import | `read_csv_headers`, `import_csv_preview`, `import_csv_commit` | `CsvImport.tsx` |
| Settings — security | `update_auto_lock_minutes`, `update_old_password_days`, `update_hibp_opt_in`, `set_pin`/`clear_pin` | `Settings.tsx` |
| Settings — biometric (desktop only) | `set_biometric`, `clear_biometric`, `update_biometric_enabled` | `Settings.tsx` |
| Settings — SSH agent (desktop only) | `add_ssh_key`, `list_ssh_keys`, `delete_ssh_key`, `update_ssh_agent_enabled` | `Settings.tsx` |
| Settings — appearance | `update_theme`, `update_accent_color`, `update_nav_collapsed`, `update_mobile_nav_style` | `Settings.tsx` |
| Settings — strength algorithm | `update_strength_algorithm` | `Settings.tsx` |
| Lock / idle timeout / suspend lock | `lock`, `touch`, `unlock_status` | `AppShell.tsx` + `useActivityWatcher.ts` + `vault-locked` event |

## 2. Full backend command inventory (54 commands, `src-tauri/src/commands.rs`)

Legend: M = main/desktop, V = viewer, W = web portal.

### Auth / session — M only unless noted
| Command | Purpose | M | V | W |
|---|---|---|---|---|
| `unlock_status` | is the vault currently unlocked | ✓ | | |
| `register_user` | create a new user + vault | ✓ | | ✓ |
| `login` | master-password login | ✓ | ✓ | ✓ |
| `has_pin` | does this user have a PIN set | ✓ | | ✓ |
| `unlock_with_pin` | PIN quick-unlock | ✓ | | ✓ |
| `set_pin` / `clear_pin` | manage PIN | ✓ | | |
| `lock` | manual lock, emits `vault-locked` | ✓ | ✓ | |
| `touch` | reset idle timer | ✓ | ✓ | |
| `verify_master_password` | reprompt re-auth gate | ✓ | | ✓ |

### Biometric (Windows Hello) — desktop only
`has_biometric`, `unlock_with_biometric`, `set_biometric`, `clear_biometric` — M only.

### SSH agent — desktop only
`add_ssh_key`, `list_ssh_keys`, `delete_ssh_key`, `update_ssh_agent_enabled` — M only. Runs a named-pipe agent (`\\.\pipe\pwmgr-ssh-agent`).

### Vault CRUD — sites
| Command | Purpose | M | V | W |
|---|---|---|---|---|
| `list_sites` | list all sites (no decryption) | ✓ | ✓ | ✓ |
| `add_site` / `update_site` | create/edit name, url, notes | ✓ | | ✓ |
| `soft_delete_site` | delete; refuses if live credentials remain | ✓ | | ✓ |

### Vault CRUD — credentials
| Command | Purpose | M | V | W |
|---|---|---|---|---|
| `add_credential` / `update_credential` | username, secret, remarks, custom fields, reprompt flag | ✓ | | ✓ |
| `list_credentials_for_site` | decrypts and returns every credential for one site | ✓ | ✓ | ✓ |
| `search` | shallow LIKE over site name / username / remarks | ✓ | ✓ | ✓ |
| `soft_delete_credential` | move to trash | ✓ | | ✓ |

No `get_credential(id)` and no "list all credentials across all sites" command exist.

### Trash
`list_trash`, `restore_credential`, `purge_trash` — M, W. Credential-level only, no site-level restore.

### TOTP
`set_totp_secret`, `clear_totp_secret`, `totp_code` — M, W. RFC 6238, 30s step, 6 digits. Listings only expose a `hasTotp` boolean; the seed is fetched on demand.

### Generator
`generate_password` (4 methods: random/memorable/pattern/pronounceable), `analyze_strength` (zxcvbn, infallible), `copy_to_clipboard` (30s auto-clear) — M, W (clipboard uses browser API on web, not this command).

### Health
`health_report` (weak/reused/old/unsecured, vault-wide sweep), `check_breach` (HIBP k-anonymity, opt-in gated) — M, W.

### Backup / CSV
`create_backup`, `restore_backup`, `export_csv`, `read_csv_headers`, `import_csv_preview`, `import_csv_commit` — M, W.

### Settings
`get_config` (infallible), plus one setter per field: `update_auto_lock_minutes`, `update_strength_algorithm`, `update_theme`, `update_accent_color`, `update_old_password_days`, `update_hibp_opt_in`, `update_nav_collapsed`, `update_mobile_nav_style`, `update_biometric_enabled`, `update_ssh_agent_enabled` — M, W (minus the two biometric/ssh setters, which are M only).

### Per-binary summary
- **Viewer (9 commands total)**: `unlock_status, login, lock, touch, list_sites, list_credentials_for_site, search, get_config, copy_to_clipboard`. **No `verify_master_password` → no reprompt gate possible in the viewer.** Any redesign sharing a detail-pane component between main and viewer must fail closed there, not silently skip the gate.
- **Web portal (43 routes)**: everything except biometric, SSH, and `copy_to_clipboard` (browser clipboard API used client-side instead).
- **Main/desktop**: all 54.

## 3. Data shapes the UI must render (`core/src/dto.rs`, all camelCase over the wire)

```ts
AuthSuccess       { username }
SiteDto           { id, name, url, notes }
CustomField       { label, value, fieldType: "text"|"hidden"|"boolean" }
CredentialDto     { id, siteId, username, secret, remarks, customFields[], reprompt, hasTotp }
SearchResultDto   { siteId, siteName, credentialId, username }
TrashedCredentialDto { credentialId, siteId, siteName, username, deletedAt }
TotpCodeDto       { code, secondsRemaining }
ConfigDto         { theme, autoLockMinutes, strengthAlgorithm, accentColor,
                     oldPasswordDays, hibpOptIn, navCollapsed, mobileNavStyle,
                     biometricEnabled, sshAgentEnabled }
SshKeyDto         { id, label, publicKey }
StrengthDto       { score (0-4), guessesLog10, label }   // guessesLog10 unused by any current screen
HealthEntryDto    { credentialId, siteId, siteName, username }
ReusedGroupDto    { entries: HealthEntryDto[] }
HealthReportDto   { weak[], reused[], old[], unsecured[] }
BreachReportDto   { breached: HealthEntryDto[] }
ImportRecordDto   { siteName, url, notes, username, secret, remarks }
ImportSummaryDto  { imported, duplicates }
GenerateRequest   tagged union: random | memorable | pattern | pronounceable
ExportMode        tagged union: thisDevice | withPassword{password} | plaintext
ImportFormat      tagged union: chromium | firefox | lastPass | generic{mapping}
```

Known trap: `credentials.strength_score` exists as a DB column but is **dead** —
never written, never read by any command. Do not assume it's populated.

## 4. Cross-cutting behaviors that must survive any redesign

These are non-negotiable — currently implemented in places that are easy to
drop during a rewrite:

- **Clipboard auto-clear (30s)** — must route through one clipboard call, never
  a raw `navigator.clipboard.writeText`. Implemented separately on desktop
  (Rust, `copy_to_clipboard`) and web (browser API), both with the same
  "only clear if unchanged" check.
- **Reprompt-on-reveal per credential** — when `credential.reprompt` is true,
  intercept both reveal *and* copy, re-authenticate via `verify_master_password`
  before proceeding. Applies to the main secret **and** hidden-type custom
  fields — easy to apply to one and forget the other.
- **Auto-lock on idle + suspend** — frontend needs a throttled "activity" ping
  (every ~5s while active) plus backend enforces idle timeout and immediate
  lock on OS suspend. The UI only learns it happened via a push event — losing
  that listener silently breaks the visible lock (data stays protected either
  way, but the screen won't show it).
- **Theme + accent persistence** — applied at the document root, not just React
  state, with static defaults baked into the HTML shell so there's no flash of
  wrong theme before config loads. 5 accent options.
- **Responsive nav style** — three interchangeable nav presentations
  (icon rail / bottom tabs / drawer), user-selectable, breakpoint-driven.
- **Lazy decryption invariant** — credentials for a site are fetched only when
  that site is opened, never prefetched for the list view. A redesign that
  eagerly loads all credentials to populate a sidebar/list regresses this and
  defeats the point of per-site decryption.

## 5. Data-model facts the redesign must accept as given (current backend)

- **Two-level model**: a "site" (name/url/notes) owns one or more "credentials"
  (username/secret/custom fields/TOTP/reprompt). Not flat — there is no
  1-item-1-password entity. A credential cannot be moved to another site.
- **No item-type discriminator** — every row is implicitly a login. No
  secure-note/card/identity storage exists in the schema.
- **No folders, no favorites, no tags** — none of these columns exist anywhere.
- **Search is shallow** — matches only site name, credential username, and
  credential remarks. Never secrets, URLs, notes, or custom fields.
- **Health is vault-wide only** — no per-item health query exists today.
  `analyze_strength(password)` takes a raw password string, so a redesign
  *can* show a live per-item strength meter using the plaintext already fetched
  for that item — but reused/old/breached are all full-vault sweeps with no
  per-item equivalent.
- **Trash is credential-level only** — no site-level restore.
- **Notes exist at two levels already**: site-level `notes` and credential-level
  `remarks`, both plaintext (the whole file is encrypted at rest; these two
  columns aren't separately sealed the way the secret/TOTP/custom-fields are).

## 6. Explicit non-goals (already-decided, keep them out)

- Sync / cloud accounts, multi-device — product is structurally local-only, no
  server, no account system. Already decided against.
- Standalone CLI binary, org-sharing, multi-tenant key wrapping — already
  flagged not-recommended for this project.
