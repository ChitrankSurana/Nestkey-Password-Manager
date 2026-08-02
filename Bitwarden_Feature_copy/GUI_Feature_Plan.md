# Bitwarden GUI Feature Survey & Port Plan (Phase 1: Clients-only)

**Scope note:** This is a planning document only — no code has been written or changed. It surveys
operational workflows and features (not CI/CD, not GitHub Actions) from the `bitwarden/clients`
monorepo (web vault, browser extension, desktop app — Electron) and, where relevant, the
`bitwarden/design` component library, and evaluates each for portability into our single-user,
local-vault Tauri v2 + React 19 + Rust app. The app currently has no multi-device sync or server
backend, so anything that assumes an org/account server is marked accordingly. CLI, mobile, and
server/SDK repos are explicitly out of scope for this pass and only get a one-line mention at the
bottom. This is step 1 of a larger effort — a follow-up pass can revisit sync-dependent features
once (if) the project grows a backend.

Source repos consulted: `bitwarden/clients` (`apps/{web,browser,desktop,cli}`, `libs/*`),
`bitwarden/design` (component/story catalogue), plus Bitwarden help-center docs for
behavioral detail not visible from file listings alone.

---

## Prioritized Feature Table

| Feature | What it does | Bitwarden source location | Fit | Adaptation notes |
|---|---|---|---|---|
| Password generator | Rule-based random password/passphrase generation (length, char sets, word count/separator) | `libs/tools/generator/{core,components,extensions}` | Easy | Pure logic, no server; port generation rules as a Rust or TS module, no protocol changes |
| Username generator | Plus-addressed email, catch-all domain, random word, and forwarder-service (SimpleLogin/AnonAddy/Fastmail/DuckDuckGo) usernames | `libs/tools/generator/core` | Easy–Medium | Plus-address/catch-all/word are pure string logic (Easy); forwarder integrations need outbound API calls + stored API keys (Medium) |
| TOTP / authenticator field | Stores a TOTP seed on a login item; generates a rotating 6-digit code (SHA-1, 30s) shown/copied from the item, usable offline once vault is unlocked | `libs/vault/src` (cipher model + display), TOTP calc in shared crypto/common libs | Easy | Needs a TOTP crate (`totp-lite`/`otpauth`-equivalent) in Rust or TS; no server dependency, purely local computation |
| Vault Health Report (expansion) | Flags weak, **reused**, and exposed/breached passwords plus logins on **unsecured (http://)** sites and items missing 2FA | `libs/dirt` (reporting), web app reports UI | Easy–Medium | We already do weak-password scoring via zxcvbn (Health Dashboard). Reused-password and unsecured-site checks are local list comparisons (Easy). Breach check via HaveIBeenPwned k-anonymity API is optional/opt-in networked call (Medium) |
| Vault timeout / auto-lock | Configurable inactivity timer with platform triggers (idle, system sleep/lock, app close) and a chosen action: Lock (keep encrypted data, unlock via password/PIN/biometrics) vs Log out (wipe session) | `libs/auth`, `libs/unlock`, `libs/key-management-ui` | Easy–Medium | We already have session/lock handling; add idle-timer + OS sleep/lock hooks (Tauri window/OS events) and a lock-vs-logout choice |
| PIN unlock | Short PIN as a faster local-only unlock alternative to the master password, decrypts a locally-wrapped key | `libs/auth/src/common` | Easy | Store PIN-wrapped vault key locally (as Bitwarden does); no server round-trip needed even in their design |
| Biometric unlock | OS biometric prompt (Windows Hello/Touch ID/polkit) unwraps a locally stored, OS-keystore-protected vault key | `apps/desktop/desktop_native/biometric`, `libs/key-management-ui` | Medium | Bitwarden's version also handles browser-extension↔desktop native-messaging handoff — **we don't need that split** since we're one app; call Windows Hello directly from the Rust/Tauri side |
| Clipboard-clear-on-timeout | Configurable delay after a copy (e.g. password field); clears clipboard only if its contents still match what was copied, to avoid clobbering unrelated copies | Desktop/browser platform services (`apps/desktop/src/platform`) | Easy | Tauri clipboard API + a timer + a "did the clipboard change" guard before clearing |
| Master-password re-prompt | Per-item flag requiring master password re-entry to view/copy a sensitive field even while the vault is unlocked | `libs/vault/src/cipher-view`, `cipher-form` | Easy | Add a boolean flag on cipher model + a re-auth modal gate before reveal/copy actions |
| Custom fields on items | Arbitrary text/hidden/boolean/linked fields attached to a cipher, beyond the fixed login schema | `libs/vault/src/cipher-form`, `models` | Easy | Extend cipher schema + form renderer; local-only, no protocol change |
| Vault import (multi-format) | Parses 40+ competitor/browser export formats (1Password, LastPass, KeePass CSV/XML, Chrome/Firefox CSV, etc.) into Bitwarden's internal model | `libs/importer` | Medium | Each format parser is simple in isolation; effort scales with how many formats you choose to support — start with 2-3 common ones |
| Vault export | Exports vault to JSON/CSV/encrypted-JSON | `libs/tools/export-vault-core`, `export-vault-ui` | Easy | Mostly serialization of data we already hold |
| SSH Agent | Desktop app acts as a local SSH agent (Pageant/OpenSSH-agent-compatible socket/pipe); stores Ed25519/RSA keys as a vault item type, signs SSH requests on demand with a UI approval prompt | `apps/desktop/desktop_native/ssh_agent`, `secure_memory` | Medium–Hard | Real value-add since our backend is already Rust; needs an agent-protocol implementation (`ssh-agent` crate exists) and a named pipe/socket per OS |
| Autotype | Global hotkey types a matched login's username/password (and TOTP) directly into whatever window has focus, instead of via browser autofill | `apps/desktop/desktop_native/autotype` (present in repo tree; not yet a stable/announced feature) | Medium–Hard | High value for us since we have **no** browser extension — this is the natural "autofill" equivalent for a bare desktop app. Needs OS-level global hotkey registration + synthetic keystroke injection (different APIs per OS) |
| Folders | Simple flat/nested personal grouping of vault items | `libs/vault` | Easy | Likely trivial if not already present; just a parent-id field + tree UI |
| Vault item attachments | Encrypted file blobs attached to a cipher (e.g. a scanned ID) | `libs/vault` (cipher model), platform storage services | Medium | Needs local encrypted blob storage alongside the vault DB; format/size limits are a design choice |
| Autofill URI matching | Configurable match strategies (base domain, host, starts-with, exact, regex, never) used to decide which saved logins apply to a given site | `apps/browser` (content scripts), `libs/vault` (login URI model) | N/A for us directly | Only meaningful with a browser extension, which is out of scope; the *matching strategy* concept is reusable if we ever build one |
| Passkey/FIDO2 storage & platform authenticator | Stores WebAuthn passkeys in vault items; acts as a platform authenticator for site logins / Windows sign-in | `apps/desktop/desktop_native/win_webauthn`, browser passkey provider code | Hard | Requires implementing OS platform-authenticator provider APIs (Windows Hybrid/FIDO APIs) — substantial native surface for a single-user app |
| Send | End-to-end encrypted one-off text/file sharing via a link, with deletion date, expiration, max-access-count, and optional password | `libs/tools/send` (`send-core`, `send-ui`) | Hard | Genuinely needs *some* transport/hosting for the recipient to fetch the blob — inherently a "share with someone else" feature. Only portable as a local "export an encrypted, self-contained file the recipient decrypts with a password" (drops the link/expiry/access-count server logic) |
| Organizations / Collections | Multi-user shared vaults with role-based collections and permissions | `libs/admin-console` | Hard | Fundamentally server + multi-account; not applicable to a single local vault |
| Emergency access | Grants a trusted contact delayed/approved access to your vault in case of emergency | `libs/admin-console`, `libs/auth` | Hard | Needs a second account + a server to mediate the time-delayed grant/approval handshake |
| Account switching | Multiple logged-in accounts/profiles in one app instance | `apps/*` platform/auth services | Medium | Could be reinterpreted as "multiple local vault profiles" (separate encrypted DB files) rather than multiple server accounts |

---

## Detailed Plans: Top Easy/Medium Fit Features

### 1. Vault Health Report expansion (reused + unsecured-site checks, optional breach check)
We already run zxcvbn for "Weak" on the Health Dashboard. Add two more local, zero-network checks
alongside it: (a) **Reused passwords** — group vault entries by password hash/equality, flag any
group with size > 1; (b) **Unsecured websites** — flag any login URI starting with `http://`
instead of `https://`. Both are pure Rust/TS logic over data we already have in memory — no new
storage. As a stretch, add an **opt-in** "Check for breaches" button that hashes a password with
SHA-1, sends only the first 5 hex chars to the HaveIBeenPwned range API (k-anonymity, matches
Bitwarden's own approach), and checks the full hash against the returned list client-side. Keep it
manual/opt-in (button, not automatic) to preserve the "local-first, no silent network calls" ethos.

### 2. TOTP / authenticator field on login items
Add an optional `totp_secret` field to the login item model. On display, compute the current
6-digit code client-side (standard TOTP: HMAC-SHA1, 30s step) and show a countdown ring plus a
copy button, refreshing every second. A small Rust crate (e.g. an RFC 6238 implementation) run from
the Tauri backend keeps the secret out of the WebView JS context, consistent with how the master
password/vault key are already handled. QR-code entry can be deferred; manual secret paste is
enough for v1.

### 3. Unlock methods: PIN + biometric unlock
Add PIN unlock as a second local unlock path: derive a key-wrapping key from the PIN (with a slow
KDF) and use it to locally wrap the existing vault encryption key, stored on disk. Unlocking with
PIN just unwraps that local blob — never touches the master password. Biometric unlock follows the
same shape but the wrapping key comes from the OS credential/secure-enclave API instead of a KDF
(e.g. Windows Hello via `windows-rs`, Touch ID via a macOS crate). Because we're a single desktop
app (no separate browser extension needing a handoff), this is simpler than Bitwarden's
native-messaging-based design — call the OS API directly from the Rust side and unwrap in-process.

### 4. Auto-lock idle timer + clipboard-clear-on-timeout
Extend the existing session/lock handling with: (a) an idle timer that resets on any Tauri window
input event and triggers the existing lock/log-out flow after N minutes (configurable, "Never"
allowed but flagged as risky in the UI, mirroring Bitwarden's own warning copy); (b) OS
sleep/lock-screen hooks (Tauri has window/session events, or a small Rust listener) that trigger
the same flow immediately instead of waiting for the idle timer; (c) a clipboard-clear timer set
whenever a sensitive field is copied, which re-reads the clipboard right before clearing and only
clears it if the contents still match what was copied (avoids nuking something the user copied
from elsewhere in the meantime).

### 5. SSH Agent
Since our backend is already Rust, this is a natural fit conceptually, even though the native
surface is non-trivial. Add an "SSH Key" item type to the vault model (private key + public key +
key type), and a Rust module that speaks the standard `ssh-agent` protocol over a local named
pipe (Windows) / Unix socket (macOS/Linux), using the `ssh-agent`/`ssh-key` crates rather than
hand-rolling the wire protocol. On a signing request, show a native approval prompt before signing
with the in-memory (unlocked-vault) private key. Ship Windows-first given the dev environment, gate
behind a settings toggle since it's a background service, not just UI.

### 6. Vault import (start with 2-3 formats)
Add an import pipeline: pick a file → detect/select format → parse into our internal item model →
preview/dedupe → commit. Start with the highest-value formats for a personal migration story:
generic CSV (username/password/url/notes columns), Chrome/Firefox password CSV export, and one
competitor JSON/CSV (e.g. LastPass or 1Password 1pux). Each parser is a small, isolated
mapping function — no shared server-side validation logic to replicate, since Bitwarden's
importer already runs entirely client-side too.

### 7. Custom fields + master-password re-prompt on items
Two small, independent cipher-model extensions that are easy to ship together since both touch the
same item-detail form: (a) custom fields — let a user attach arbitrary text/hidden/boolean fields
to any item, rendered dynamically in the item form; (b) re-prompt flag — a per-item checkbox
("Require master password to view") that gates the reveal/copy actions behind a quick re-auth
modal even while the vault is otherwise unlocked. Both are schema + form-rendering changes only.

---

## Deferred / Out of Scope for GUI Phase

- **CLI** (`apps/cli`) — scriptable `bw` command, `serve` local-API mode, `bw send` — revisit
  later if a scripting/automation interface is wanted.
- **Mobile** (`bitwarden/ios`, `bitwarden/android`) — OS-native autofill integration
  (AutofillService/Credential Provider) — not applicable to a desktop app; later if mobile is ever
  in scope.
- **Server / multi-user** (`bitwarden/server`, `libs/admin-console`, `bitwarden/directory-connector`) —
  organizations, collections, groups, SSO, emergency access, device-approval login, Send hosting —
  all assume a backend and multiple accounts; revisit only if the project grows sync/multi-device
  support.
- **SDK** (`bitwarden/sdk`, `bitwarden/sdk-internal`) — shared Rust crypto core across all
  clients — worth a glance later for crypto implementation *patterns*, not a direct port target.
- **Design system** (`bitwarden/design`) — component/story catalogue is mainly useful as visual
  reference if a UI refresh is planned; not a functional-feature source.
