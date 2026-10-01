# Technical Requirements Document (TRD)

> **Status note (2026-09-30):** Original v1.0 design doc. Newer modules (`totp`, `biometric`, `ssh_agent`, `ssh_keys`, `jsonio`, `hibp`) and the Make UI are described in `docs/ARCHITECTURE.md`.

**Product:** Local-First Zero-Knowledge Password Manager
**Status:** Draft v1.0 — approved design, pre-implementation
**Last updated:** 2026-07-01

## 1. Stack

| Layer | Choice | Notes |
|---|---|---|
| App shell | **Tauri 2.x** | Rust core process + native webview window |
| Core logic | **Rust** | Crypto, database, business logic — all secrets live here |
| Frontend | **React 18 + TypeScript + Vite + Tailwind CSS** | UI only; no plaintext secret storage in JS memory beyond what's on screen |
| Database | **SQLite via SQLCipher** (`rusqlite` + `sqlcipher` feature, or `sqlx` with a SQLCipher-compiled libsqlite3) | One `.vault` file per user |
| KDF | **Argon2id** (`argon2` crate) | Master password → 256-bit key |
| Symmetric encryption | **AES-256-GCM** (`aes-gcm` crate) | Field-level defense in depth on top of SQLCipher |
| Secure memory | `zeroize` crate | Wipe key material on drop/lock |
| Breach check | HIBP `range` API (k-anonymity), opt-in | Only network call in the whole app |

## 2. Non-Functional Requirements

- **NFR-1 Local-only by default:** the app makes zero network calls unless the user explicitly opts
  into the HIBP breach check.
- **NFR-2 Zero-knowledge:** the master password and derived key must never be written to disk, logs,
  crash reports, or telemetry (there is no telemetry).
- **NFR-3 Performance:** vault unlock ≤ 1s on typical hardware (Argon2id params tuned accordingly);
  search results update within one UI frame of keystroke for vaults up to ~5,000 entries.
- **NFR-4 Resilience:** a corrupted/partial write must not silently lose data — use SQLite transactions
  for every mutating operation.
- **NFR-5 Portability:** core Rust modules must not assume Windows-only APIs, so later macOS/Linux
  ports (and the Android door) stay open.
- **NFR-6 Accessibility:** all interactive controls keyboard-navigable; tooltips available on
  keyboard focus, not just mouse hover.

## 3. Cryptographic Design

### 3.1 Registration
1. User supplies `username` + `master_password`.
2. Generate random `kdf_salt` (16 bytes) and `verifier_salt` (16 bytes).
3. `vault_key = Argon2id(master_password, kdf_salt, params)` (256-bit output).
4. `verifier_hash = Argon2id(master_password, verifier_salt, params)` — stored, used only to check
   login attempts cheaply before touching the DB.
5. Generate `sentinel` (random 32 bytes), encrypt it with `vault_key` via AES-256-GCM →
   `encrypted_sentinel` + `sentinel_nonce`. Stored to later confirm a derived key is correct.
6. Create the user's SQLCipher database, keyed with `vault_key` (passed to `PRAGMA key`).
7. `vault_key` and `master_password` are held only in process memory for this session, then dropped
   (zeroized) on lock/logout.

### 3.2 Login / Unlock
1. Look up `kdf_salt`, `verifier_salt`, `verifier_hash`, `encrypted_sentinel` for `username`.
2. Recompute `Argon2id(input_password, verifier_salt)`, compare to `verifier_hash` (constant-time).
3. On match, derive `vault_key = Argon2id(input_password, kdf_salt)`, open the SQLCipher DB with it,
   and confirm by decrypting `encrypted_sentinel`. GCM authentication failure ⇒ reject (belt-and-braces
   against a corrupted or tampered vault, and against `verifier_hash` collisions).
4. On any failure return a single generic "incorrect master password" — no distinguishing information.

### 3.3 Record encryption
- SQLCipher encrypts the entire DB file at rest (page-level AES-256).
- Additionally, `credentials.secret` is sealed with AES-256-GCM using `vault_key`, a fresh random
  96-bit nonce per write, stored alongside the ciphertext. This is defense in depth: even a SQLCipher
  misconfiguration wouldn't expose secrets, and each secret is independently tamper-evident.

### 3.4 Session / auto-lock
- On unlock, start an idle timer (default 5 min; user-configurable to 1/5/15 min).
- Any UI interaction resets the timer.
- On expiry: close the SQLCipher handle, zeroize `vault_key` and any in-memory `master_password`
  buffer, and return the UI to the login screen.
- (M6 web mode) issue a cryptographically random 256-bit session token bound to the unlocked session;
  required on every IPC-over-HTTP call; HttpOnly cookie; server refuses to bind to any interface other
  than `127.0.0.1`.

### 3.5 Argon2id parameters (starting point, to be benchmarked on target hardware)
- Memory: 64 MiB, Iterations: 3, Parallelism: 4 (OWASP-recommended baseline; tune for ≤1s unlock).

## 4. Data Model (SQL)

```sql
CREATE TABLE users (
  id INTEGER PRIMARY KEY,
  username TEXT UNIQUE NOT NULL,
  kdf_salt BLOB NOT NULL,
  verifier_salt BLOB NOT NULL,
  verifier_hash BLOB NOT NULL,
  encrypted_sentinel BLOB NOT NULL,
  sentinel_nonce BLOB NOT NULL,
  created_at TEXT NOT NULL
);
-- Note: this table lives in a small unencrypted "registry" DB (usernames + KDF params only —
-- no secrets), separate from each user's own SQLCipher vault, so the login screen can list users
-- and locate the right vault file without decrypting anything.

-- Inside each user's <username>.vault (SQLCipher-encrypted):
CREATE TABLE sites (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  url TEXT,
  notes TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  deleted_at TEXT
);

CREATE TABLE credentials (
  id INTEGER PRIMARY KEY,
  site_id INTEGER NOT NULL REFERENCES sites(id),
  username TEXT,
  secret_enc BLOB NOT NULL,
  secret_nonce BLOB NOT NULL,
  remarks TEXT,
  strength_score REAL,
  password_updated_at TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  deleted_at TEXT
);

CREATE INDEX idx_sites_name ON sites(name);
CREATE INDEX idx_credentials_site ON credentials(site_id);
```

## 5. Module Breakdown (Rust core, `src-tauri/src/`)

| Module | Responsibility |
|---|---|
| `crypto` | Argon2id KDF, AES-256-GCM seal/open, zeroization helpers |
| `registry` | Unencrypted user registry (usernames, salts) — lookup only, no secrets |
| `vault` | SQLCipher open/close, CRUD for sites/credentials, search |
| `auth` | Register/login flow, verifier checks |
| `session` | In-memory key holder, idle timer, lock/unlock state machine |
| `generator` | 4 password generation methods |
| `health` | Entropy scoring, reuse/age detection, HIBP k-anonymity client (opt-in) |
| `backup` | Full encrypted backup file (versioned) + restore |
| `portio` | CSV import parsers (Chromium, Firefox, generic mapper) + CSV export (encrypted/plaintext-gated) |
| `commands` | Tauri `#[tauri::command]` IPC entry points exposed to the frontend |

## 6. IPC Contract (Tauri commands, high-level)

`register_user`, `login`, `lock`, `unlock_status`, `list_sites`, `search`, `add_site`,
`add_credential`, `update_credential`, `soft_delete_credential`, `restore_credential`,
`purge_trash`, `generate_password`, `analyze_strength`, `health_report`, `check_breach` (opt-in),
`create_backup`, `restore_backup`, `export_csv`, `import_csv_preview`, `import_csv_commit`.

Every command that touches vault data requires an active unlocked session; the frontend never passes
or receives raw key material — only opaque session state and plaintext record fields needed for
display.

## 7. Testing Requirements

- **Mandatory, automated:** crypto round-trip, wrong-password rejection, tamper detection (GCM auth
  failure), zeroization (best-effort verification), CRUD correctness, search correctness, CSV
  round-trip fidelity.
- **Lighter/manual acceptable:** UI/theme visuals, tooltip presence (checklist), responsive layout.

## 8. Dependency Checker

A `cargo`/npm script (`scripts/check-deps`) that verifies: Rust toolchain version, Node/npm version,
Tauri CLI, and that the SQLCipher-enabled SQLite build is available — printing clear pass/fail with
install hints for anything missing.
