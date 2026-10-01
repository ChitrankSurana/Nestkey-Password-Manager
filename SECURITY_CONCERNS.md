# Security Concerns

> **Status note (2026-09-30):** H1 (file read/write via client paths), M1 (login throttle), M2 (Argon2 flood) and M3 (CSP/headers) were closed in Phase 9 — see `.planning/phases/09-web-mode-security-hardening/09-SECURITY.md` (`threats_open: 0`). The audit below is the original 2026-07-19 snapshot; L-items were not re-audited.

Audit date: 2026-07-19. Scope: `pwmgr-core`, `src-tauri` (desktop), `viewer-src-tauri`,
`web-src-tauri` (localhost web mode), and the React frontend. Findings are ranked by severity.

Each item lists **where**, **what**, **impact**, and a **fix**. Items already documented as accepted
risks in `docs/THREAT_MODEL.md` are marked *(accepted)* and included for completeness.

The crypto core is well built (Argon2id + AES-256-GCM, constant-time verifier compare, decoy KDF to
kill the username-timing oracle, authenticated encryption, zeroize on the key type). Most of the real
risk lives in **web mode (M6)**, where commands designed for a native desktop file dialog were exposed
verbatim over HTTP.

---

## HIGH

### H1 — Web mode: arbitrary file read/write via client-supplied `path`
**Where:** `web-src-tauri/src/handlers.rs` — `restore_backup`, `export_csv`, `read_csv_headers`,
`import_csv_preview`; core impls in `core/src/commands.rs` (`do_export_csv`, `do_read_csv_headers`,
`do_import_csv_preview`, `do_restore_backup`).

**What:** these handlers take a raw `path: String` from the HTTP request body and pass it straight to
`std::fs::read` / `std::fs::write` with no validation, canonicalization, or confinement to the app
data directory. On desktop the path comes from an OS native file dialog the user drove; in web mode
the browser sends an arbitrary string.

**Impact:** any authenticated web session (or any XSS in the SPA — see H2/M3) can:
- **Read any file** the server process can read. `read_csv_headers` returns file contents as headers;
  `import_csv_preview` parses and returns file contents as records. → arbitrary file disclosure
  (e.g. `~/.ssh/id_rsa`, other users' files the process can reach).
- **Write/overwrite any file** the process can write. `export_csv` writes to the attacker-chosen
  `path`, and in `Plaintext` mode that content is **the decrypted vault** — arbitrary-location
  plaintext-secret drop. `restore_backup` reads an arbitrary path and can overwrite a vault file.

**Fix:** in web mode, do not accept absolute/relative caller paths. Confine reads/writes to a fixed
app directory (e.g. only filenames under `backups_dir()` / a downloads dir), canonicalize and reject
anything that escapes the root (`..`, symlinks, drive-absolute). Better: for web mode, deliver exports
as an HTTP download body and accept imports as an uploaded body, so no server-side path is ever named
by the client.

---

## MEDIUM

### M1 — Web mode: no rate limiting or lockout on login (+ open registration)
**Where:** `web-src-tauri/src/handlers.rs` — `/api/login`, `/api/register_user`; `build_router`.

**What:** `/api/login` has no attempt throttling, backoff, or account lockout. `/api/register_user` is
a public (pre-auth) route, so anyone who can reach `127.0.0.1:7373` can create users.

**Impact:** online brute-force of the master password, bounded only by Argon2 cost. `THREAT_MODEL` A4
covers eavesdropping and CSRF but not a local process hammering the login endpoint. On a shared
machine (A2) another local user can both enumerate/attack logins and register accounts.

**Fix:** add per-username/per-IP attempt throttling with exponential backoff and a temporary lockout.
Consider gating registration behind a first-run/desktop-only flow rather than an always-open HTTP
route.

### M2 — Web mode: Argon2 flood → memory-exhaustion DoS
**Where:** `core/src/crypto.rs::derive_key` (`m=64 MiB, t=3, p=4`), driven by `/api/login`,
`/api/register_user`, `/api/export_csv` (with-password mode).

**What:** every login/register/password-export attempt runs an Argon2id derivation allocating ~64 MiB.
No rate limit (M1) and no concurrency cap.

**Impact:** a burst of unauthenticated login/register requests allocates 64 MiB each in parallel →
local denial of service (memory pressure, request pile-up). Cheap to trigger, no auth required.

**Fix:** same throttling as M1, plus a bounded semaphore around KDF-bearing handlers so concurrent
derivations can't exhaust RAM.

### M3 — Web mode: no CSP / security headers on the served SPA
**Where:** `web-src-tauri/src/handlers.rs::build_router` (`ServeDir`/`ServeFile` fallback);
`src-tauri/tauri.conf.json` has `"csp": null`.

**What:** the axum static server sends no `Content-Security-Policy`, `X-Content-Type-Options`,
`Referrer-Policy`, or `X-Frame-Options`. The Tauri desktop shell also disables CSP (`csp: null`).

**Impact:** no defense-in-depth against XSS. React auto-escapes by default, but any injection sink
(dangerouslySetInnerHTML, a future markdown/notes renderer, a dependency bug) becomes full vault
compromise — decrypted secrets are in the DOM while unlocked, and an XSS can also drive H1's file
read/write. `csp: null` in Tauri means an injected script has the full IPC surface.

**Fix:** set a strict `Content-Security-Policy` (`default-src 'self'`, no inline where avoidable) as an
axum response-header layer, and a real CSP in `tauri.conf.json` instead of `null`. Add
`X-Content-Type-Options: nosniff`.

---

## LOW

### L1 — Mutex poisoning turns any panic into a persistent DoS
**Where:** pervasive `*.lock().unwrap()` (`core/src/state.rs`, `core/src/commands.rs`,
`web-src-tauri/src/*`).

**What:** if any thread panics while holding one of these mutexes, the lock is poisoned and every
later `.lock().unwrap()` panics too.

**Impact:** one panic (e.g. a `.parse().unwrap()` on a header, an unexpected `expect`) permanently
bricks the vault/session/registry state for the process — a stuck app (desktop) or downed server (web)
until restart.

**Fix:** recover from poisoning (`.lock().unwrap_or_else(|e| e.into_inner())`) where continuing is
safe, or ensure no code panics while holding these locks. Audit `unwrap()`/`expect()` on the request
path in `handlers.rs`.

### L2 — Decrypted secrets are not zeroized at the vault/command layer
**Where:** `core/src/vault.rs` `Credential.secret: String` (has a `TODO(M1 Phase B)`);
`core/src/commands.rs` `HealthRow.secret`, `build_csv_rows` `CsvRow.secret`.

**What:** plaintext secrets decrypted for listing, health, breach-check, and CSV export are held in
plain `String`s and dropped without zeroizing, unlike the generator output (which uses `Zeroizing`).

**Impact:** plaintext password fragments linger in freed heap memory, widening the exposure window to
O1 (memory-reading malware) beyond what's strictly necessary. Reduces, not eliminates, an
already-accepted risk.

**Fix:** wrap decrypted secrets in `zeroize::Zeroizing<String>` end-to-end (the TODO already commits to
this at the IPC layer).

### L3 — SQLCipher key hex leaves an un-zeroizable copy *(documented)*
**Where:** `core/src/vault.rs::open_vault`.

**What:** the raw key is hex-formatted and passed to `PRAGMA key`; rusqlite copies that literal into
its own SQL buffer, which this crate can't reach or zeroize. Already documented in-code as at the
practical ceiling for PRAGMA-based keying.

**Fix (optional):** use `sqlcipher`'s raw-key binding via a bound parameter if/when the binding path
supports it; otherwise accept and keep the note.

### L4 — `ServeDir::new("dist")` is a relative, CWD-dependent path
**Where:** `web-src-tauri/src/handlers.rs::build_router`.

**What:** the SPA is served from `./dist` relative to the process working directory.

**Impact:** if the server is ever launched from an unexpected CWD, it serves whatever `./dist` resolves
to there — potentially attacker-controlled or stale assets.

**Fix:** resolve `dist` against a known base (the executable dir or a configured absolute path) at
startup.

### L5 — Windows app-directory permission restriction is best-effort *(partially documented)*
**Where:** `core/src/paths.rs::restrict_to_owner` (Windows).

**What:** owner-only ACL is applied via `icacls` after `create_dir_all`; if `icacls` fails it only logs
a warning and continues, and there is a brief window between create and restrict.

**Impact:** on Windows, the A2 (shared-machine) file-permission mitigation can silently be weaker than
on Unix (where the create is atomic at `0o700`). Vault files stay encrypted regardless, so this is
defense-in-depth only.

**Fix:** verify the ACL actually applied and surface a real warning to the user if not; consider
failing closed for the vaults directory specifically.

### L6 — Session cookie has no `Secure` attribute *(acceptable on localhost)*
**Where:** `web-src-tauri/src/handlers.rs::issue_session` —
`session=<token>; HttpOnly; SameSite=Strict; Path=/`.

**What:** no `Secure` flag. Correct for plain-HTTP localhost (Secure would prevent the cookie from
being sent), but worth noting so it isn't lost if the server ever binds anything but loopback HTTP.

**Fix:** none needed while localhost-only; add `Secure` if TLS/non-loopback is ever introduced, and
keep the `127.0.0.1`-only bind hard-coded (currently enforced in `web-src-tauri/src/main.rs`).

### L7 — Plaintext CSV export leaves unencrypted secrets on disk *(accepted, by design)*
**Where:** `core/src/commands.rs::do_export_csv` (`ExportMode::Plaintext`).

**What:** gated behind explicit confirmation and logged locally, but the resulting file is unencrypted
and persists until the user deletes it. In web mode this combines with H1 (attacker-chosen path).

**Fix:** none for the desktop design; ensure web mode cannot reach the plaintext branch with an
arbitrary path (see H1).

### L8 — No master-password strength enforcement *(accepted, by design)*
**Where:** `core/src/auth.rs::register_user`.

**What:** registration accepts any master password; strength is advisory (a nudge, not a gate), per
`THREAT_MODEL` A1.

**Fix:** none required by design; consider a minimum-entropy soft floor if product goals change.

### L9 — Repo hygiene: committed export/notes files
**Where:** `CONVERSATION_EXPORT.md`, `RESUME_HERE.md`, `REPO_CONTENTS.md` at repo root.

**What:** `.gitignore` correctly excludes `target/`, `dist/`, `node_modules/`, and local settings, but
several large narrative/export docs are committed.

**Impact:** low — verify none embed real credentials, vault paths with secrets, or tokens before the
repo is shared/published.

**Fix:** review these files for sensitive content; move working notes out of version control if not
needed.

---

## Notes on things that are done well (not concerns)

- Constant-time verifier comparison (`subtle::ConstantTimeEq`) and a decoy Argon2 derivation for
  unknown usernames — closes the login timing oracle.
- Authenticated encryption (AES-256-GCM): tamper in ciphertext *or* nonce fails loudly.
- Separate salts for vault key vs. verifier; master password never persisted.
- Generic sanitized `CommandError` mapping — no raw SQLite/internal detail leaks to the frontend.
- HIBP uses k-anonymity (5-char SHA-1 prefix), off by default, server-side opt-in gate.
- Manual `Debug` impls redact secrets on `Credential`/`ImportRecord`; `VaultKey` is `ZeroizeOnDrop`.
- CSRF defense in depth: `SameSite=Strict` cookie + required custom `x-pwmgr-csrf` header on non-GET.
- SQL uses bound parameters throughout; `search()` escapes LIKE wildcards.
