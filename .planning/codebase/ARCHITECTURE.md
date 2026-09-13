<!-- refreshed: 2026-09-13 -->
# Architecture

**Analysis Date:** 2026-09-13

## System Overview

```text
┌──────────────────────────────────────────────────────────────────┐
│                    React Frontend (TypeScript)                    │
│  App.tsx → AppShell → Navigation + Content Screens                │
│  `src/App.tsx`, `src/screens/AppShell.tsx`                       │
└──────────────┬───────────────────────────────────────────────────┘
               │
               │ Tauri IPC / HTTP API
               │
┌──────────────▼───────────────────────────────────────────────────┐
│              Tauri Backend (Rust) — Command Layer                 │
│  `src-tauri/src/commands.rs` — wraps core commands                │
│  Registers ~40 IPC handlers, manages SSH agent service            │
└──────────────┬───────────────────────────────────────────────────┘
               │
               │ Direct dependencies
               │
┌──────────────▼───────────────────────────────────────────────────┐
│               Core Library (Rust) — Business Logic                │
│  `core/src/` — vault, auth, crypto, commands, state              │
│  Shared by: src-tauri, viewer-src-tauri, web-src-tauri           │
└──────────────┬───────────────────────────────────────────────────┘
               │
               │
       ┌───────┴──────────┐
       │                  │
   ┌───▼────┐      ┌──────▼──────┐
   │  Data  │      │ External    │
   │ Layer  │      │ Services    │
   └───┬────┘      └──────┬──────┘
       │                  │
   SQLite ◄──────────► HIBP API
   ~/.pwmgr/             Windows Hello
   - registry.db         SSH Agent
   - vault.db            (file I/O)
```

## Component Responsibilities

| Component | Responsibility | File |
|-----------|----------------|------|
| **React App** | Auth flow, screen routing, theme management | `src/App.tsx` |
| **AppShell** | Main vault UI shell, navigation layout, screen coordination | `src/screens/AppShell.tsx` |
| **Vault Screen** | Display credentials, search, site management | `src/screens/Vault.tsx` |
| **SiteDetail** | Side panel for credential viewing/editing | `src/screens/SiteDetail.tsx` |
| **Auth Screens** | Login, register, PIN/biometric unlock | `src/screens/Login.tsx`, `Register.tsx` |
| **Settings Screen** | User configuration, theme, auto-lock, HIBP opt-in | `src/screens/Settings.tsx` |
| **Generator** | Password/passphrase generation UI | `src/screens/Generator.tsx` |
| **Health Dashboard** | Show weak, reused, old, compromised passwords | `src/screens/HealthDashboard.tsx` |
| **Backup/Export/Import** | Data portability screens | `src/screens/Backup.tsx`, `CsvExport.tsx`, `CsvImport.tsx` |
| **API Layer** | Tauri IPC / HTTP dispatch abstraction | `src/lib/api.ts` |
| **Tauri Commands** | IPC handler thin layer, calls core | `src-tauri/src/commands.rs` |
| **Core Vault** | Encrypt/decrypt vault, manage schema | `core/src/vault.rs` |
| **Auth Module** | Master password hashing, PIN/biometric setup | `core/src/auth.rs` |
| **Session** | Track unlock state, auto-lock timer | `core/src/session.rs` |
| **Commands** | Business logic for all IPC operations | `core/src/commands.rs` |
| **Crypto** | Encryption, key derivation, nonce generation | `core/src/crypto.rs` |
| **State** | AppState container (registry, vault, session, config) | `core/src/state.rs` |
| **SSH Agent** | Windows named pipe SSH server | `core/src/ssh_agent.rs`, `src-tauri/src/ssh_agent_service.rs` |
| **Biometric** | Windows Hello credential storage & verification | `core/src/biometric.rs`, `src-tauri/src/biometric_provider.rs` |

## Pattern Overview

**Overall:** Layered monolith with three tiers:

1. **Frontend** (React/TypeScript): UI state, navigation, event handling
2. **Backend** (Tauri commands): Stateless handlers that call core functions
3. **Core** (Rust library): Stateful business logic, database operations, crypto

**Key Characteristics:**
- **Separation of concerns:** Frontend knows only about the API contract; backend is a thin Tauri wrapper; core owns all state and rules
- **Shared core library:** Business logic (`core/`) is reusable across three Tauri/web binaries (main app, viewer, web variant)
- **Tauri IPC as API:** Frontend–backend contract is function calls with JSON serialization (DTOs defined in `core/src/dto.rs`)
- **Mutex-protected state:** AppState uses Arc<Mutex<T>> so Tauri's async runtime can safely access it
- **Desktop + Web capable:** Same core library can serve both Tauri (desktop) and HTTP (web) frontends via the `isTauri` flag in `api.ts`

## Layers

**Frontend Layer:**
- Purpose: Render UI, handle user input, fetch/display data, manage UI state
- Location: `src/`
- Contains: React components, screens, hooks, utility functions (clipboard, password strength color, toast)
- Depends on: Tauri API (`@tauri-apps/api`), crypto for local operations (none currently; all crypto is backend)
- Used by: Browser / Tauri window renderer

**Command Layer (Tauri/Backend):**
- Purpose: Accept IPC requests, validate inputs, delegate to core, return results
- Location: `src-tauri/src/commands.rs`
- Contains: Tauri command handlers that wrap `do_*` functions from core
- Depends on: Core library (`pwmgr_core`), Tauri plugins (clipboard, dialog), platform-specific modules (biometric_provider, ssh_agent_service)
- Used by: Frontend via IPC

**Core Business Logic:**
- Purpose: Own application state, execute all business rules, manage databases and crypto
- Location: `core/src/`
- Contains:
  - **State management** (`state.rs`): AppState container, Mutex-wrapped connections
  - **Database** (`vault.rs`): SQLite vault schema, encrypt/decrypt logic
  - **Authentication** (`auth.rs`, `biometric.rs`): Master password, PIN, Windows Hello
  - **Session** (`session.rs`, `autolock.rs`): Unlock state, auto-lock timer
  - **Commands** (`commands.rs`): Business logic for 50+ operations
  - **Crypto** (`crypto.rs`): AES-GCM encryption, Argon2 key derivation
  - **Features**: Password generation, TOTP, CSV import/export, HIBP integration
  - **Data Access Objects** (`dto.rs`): API contract structures

**Platform Integration:**
- Windows Hello (biometric): `core/src/biometric.rs` + `src-tauri/src/biometric_provider.rs` (Windows API calls)
- SSH Agent: `core/src/ssh_agent.rs` + `src-tauri/src/ssh_agent_service.rs` (named pipe server)
- File dialogs: Tauri plugin wrapper (`tauri_plugin_dialog`)
- Clipboard: Tauri plugin wrapper + web fallback (`src/lib/clipboard.ts`)

## Data Flow

### Primary Request Path: Credential Lookup

1. **User types in vault search** (`src/screens/Vault.tsx:54`) — triggers `setQuery()`
2. **Query state updates** → `runSearch(q)` runs (debounce-free, immediate IPC)
3. **Frontend calls API** (`src/lib/api.ts:210`) — `api.search(query)` → `call("search", {query})`
4. **Tauri dispatch** → `isTauri ? invoke("search", ...) : httpPost("/api/search", ...)`
5. **Backend handler** (`src-tauri/src/commands.rs:search`) → calls `do_search(&state, &query)`
6. **Core logic** (`core/src/commands.rs`) — SQL query: search site_name, username, remarks across vault
7. **Database reads** → vault.lock().unwrap() → Connection executes SQL
8. **Results serialized** → `SearchResultDto[]` → JSON back to frontend
9. **Frontend receives** (`Vault.tsx:42`) → `setResults(await api.search(q))`
10. **UI re-renders** — show matching credentials

**State transitions:**
- Vault is locked → search returns error ("unlock_status: false")
- User deletes a credential while search is open → next keystroke runs fresh search, excludes deleted item

### Authentication Flow

1. **User clicks "Login"** → `Login.tsx` form submission
2. **Call backend** → `api.login(username, masterPassword)`
3. **Core logic** (`do_login`):
   - Check: Is there a registry entry for this username?
   - Load: User's vault.db path from registry
   - Derive: Master key from password via Argon2 + stored salt
   - Open: vault.db with that key — if key is wrong, SQLCipher rejects it
   - Set: `AppState.session.unlock()`, `AppState.set_vault(conn, path)`
4. **Frontend receives success** → `setScreen("vault")`, `setUsername(u)`
5. **AppShell renders** → User is in the vault

**Lock flow:**
- Auto-timeout: Background task in `src-tauri/lib.rs:28-54` ticks every 2 seconds, checks `session.tick()`
- Suspend detection: If OS sleep/resume detected (elapsed > 2 ticks), force immediate lock
- Manual lock: User clicks lock button → `api.lock()` → `do_lock(&state)` → clear_vault + lock session
- Event broadcast: Backend emits `vault-locked` event → frontend hears it → `setScreen("login")`

### State Management Pattern

**Backend:**
```
AppState {
  registry: Mutex<SQLite>      ← user account list (username, salt, PIN, biometric, vault_path)
  vault: Mutex<Option<(SQLite, Path)>>  ← active user's encrypted vault (only set when unlocked)
  session: Mutex<Session>      ← is_locked(), last_activity, auto_lock_timeout
  config: Mutex<Config>        ← theme, auto_lock_minutes, accent_color, HIBP opt-in, etc.
}
```

**Frontend:**
- Screens use `useState()` for local state (search query, form inputs, expanded/collapsed sections)
- AppShell manages: current screen, selected site ID, whether nav is collapsed, mobile nav style
- No Redux/Zustand; all server state fetched on demand via `api.*` calls
- Refresh signals: SiteDetail → onBack → AppShell increments `vaultRefreshSignal` → Vault.tsx re-runs search

## Key Abstractions

**AppState:**
- Purpose: Single point of access for all app-scoped data, thread-safe via Mutex
- Examples: `src-tauri/lib.rs:20` (injected into all commands via Tauri State)
- Pattern: Tauri's dependency injection; each IPC command receives `State<AppState>` parameter

**VaultKey (newtype wrapper):**
- Purpose: Type-safe encryption key, prevents accidental plaintext mixing
- Pattern: Zero-copy, securely zeroized on drop (via Zeroizing wrapper in crypto module)

**CommandError (DTO):**
- Purpose: Consistent error format across frontend-backend boundary
- Pattern: Serializable struct with `code` and `message` fields, frontend extracts via `isCommandError()` guard

**Session (state machine):**
- Purpose: Track "unlocked" vs "locked" state and auto-lock timeout
- States: Locked (initial) → Unlocked (after login) → Locked (after timeout or manual lock)
- Pattern: Duration-based timer; `tick()` checks elapsed time since last activity

**DTO Layer:**
- Purpose: Explicit serialization schema between frontend and backend
- Examples: `SiteDto`, `CredentialDto`, `ConfigDto` defined in `core/src/dto.rs`
- Pattern: Separate DTO types from internal models (if any); allows schema evolution without breaking frontend

## Entry Points

**Desktop Application:**
- Location: `src-tauri/src/main.rs` → calls `tauri_app_lib::run()`
- Triggers: `npm run tauri dev` (dev) or `tauri build` (release)
- Responsibilities:
  1. Initialize Tauri app
  2. Load AppState (registry DB, config.json)
  3. Start SSH agent if enabled in config
  4. Spawn background auto-lock task
  5. Register IPC handlers
  6. Render React app via Vite dev server or bundled static files

**Web Frontend:**
- Location: `src/main.tsx` → React app entry point
- Triggers: Browser loads `index.html` or `viewer.html`
- Responsibilities: Render App.tsx tree, attach to DOM

**Library Binaries:**
- `viewer-src-tauri/` and `web-src-tauri/`: Separate Tauri/web variants using same core library, not active in main dev flow

## Architectural Constraints

- **Threading:** Tauri spawns commands in a threadpool; AppState fields are Mutex-wrapped for thread safety. Background auto-lock task runs in a separate Tokio task spawned in `setup()`.
- **Global state:** AppState is the sole application-level singleton, injected by Tauri. No module-level statics except config singletons (Mutex-wrapped).
- **Circular imports:** None expected; core/ is pure library code with no back-dependencies on backend layer.
- **Vault access:** Only one vault can be open at a time (AppState.vault is `Option<(Connection, PathBuf)>`). On login, existing vault is cleared before opening the new one.
- **Encryption:** All sensitive data (passwords, TOTP secrets, custom fields, SSH keys) are encrypted with AES-GCM before storing in vault DB. Encryption keys are derived from master password via Argon2.
- **Auto-lock:** Frontend never initiates auto-lock; backend emits `vault-locked` event on timeout/manual lock/suspend. Frontend listens and switches screen.
- **Database schema:** Vault schema is initialized/migrated on-open in `vault.rs:init_schema()` and `add_credential_columns_if_missing()` for backward compatibility.
- **Cross-platform:** Code uses `dirs` crate for platform-aware home dir; vault stored at `~/.pwmgr/vault.db` (can be different per user on shared machine). Windows-specific features (biometric, SSH agent on named pipe) are gated behind `#[cfg(windows)]`.

## Anti-Patterns

### Anti-Pattern: Unvalidated Backend State in Frontend

**What happens:** Frontend assumes vault is unlocked (e.g., renders credential without calling `unlockStatus()` first); backend allows operations that should fail if session is locked.

**Why it's wrong:** Race condition between user activity on frontend, auto-lock timeout on backend, or manual lock from another window. Frontend can flash data that isn't actually available, or crash on API error.

**Do this instead:** Always call `api.unlockStatus()` before rendering sensitive screens. Backend `do_*` commands check `session.is_locked()` first and return error if true. Frontend catches errors and re-navigates to login screen.

### Anti-Pattern: Plaintext Secrets in Transit

**What happens:** Logging password values, storing them in React state longer than needed, or passing them through non-encrypted IPC.

**Why it's wrong:** Secrets could leak to logs, memory dumps, or devtools. Tauri IPC is local-only (not network), so this is lower risk than web apps, but still dangerous.

**Do this instead:** 
- Use `Zeroizing<String>` wrapper in Rust to explicitly erase sensitive data on drop
- Frontend never stores passwords in state; only receive them from user input, pass to backend immediately
- Backend zeros master password from memory after deriving vault key
- Vault DB is encrypted; plaintext secrets exist in memory only while a command is executing

### Anti-Pattern: Missing Vault Unlock on SSH Agent Startup

**What happens:** SSH agent service tries to access vault credentials without checking if vault is open.

**Why it's wrong:** Backend crashes or returns confusing "no vault" errors. SSH agent feature becomes unreliable.

**Do this instead:** SSH agent checks `state.vault.lock().unwrap().is_some()` before accessing. If vault is locked, agent returns "no keys available" gracefully. Vault unlock automatically re-populates SSH agent keyring.

## Error Handling

**Strategy:** Result-based (Rust's `Result<T, E>`); errors propagate to IPC boundary, serialize as CommandError DTOs.

**Patterns:**

1. **Core validation** → `Err(CommandError)`:
   ```rust
   if !valid_username(&username) {
       return Err(CommandError { 
           code: "invalid_username", 
           message: "Username must be 3-32 characters" 
       });
   }
   ```

2. **Database errors** → Convert via `?` operator:
   ```rust
   let site = conn.query_row(...)?;  // rusqlite::Error → CommandError via From impl
   ```

3. **Frontend handling** (`src/lib/api.ts:13-15`):
   ```typescript
   if (isCommandError(err)) {
       // err has .code and .message
       setError(err.message);
   } else {
       setError("Something went wrong. Please try again.");
   }
   ```

**No custom exceptions in Rust:**
- Use `#[derive(Error)]` from `thiserror` crate
- Each module defines its own error enum (e.g., `VaultError`, `RegistryError`, `CryptoError`)
- Top-level `CommandError` is the serializable form sent to frontend

## Cross-Cutting Concerns

**Logging:** 
- Backend: No structured logging library (Sentry/tracing). Print to stderr for errors: `eprintln!("warning: ...")` in lib.rs auto-lock background task.
- Frontend: `console.error()` for unhandled promise rejections; Tauri dev console visible in dev mode.

**Validation:**
- Frontend: Basic HTML5 validation (required, minlength); error messages from backend.
- Backend: All inputs validated before touching database. Username format, password strength (via zxcvbn), field lengths checked in `do_*` functions.

**Authentication:**
- Primary: Master password → Argon2 key derivation → SQLCipher vault unlock
- Secondary: PIN → stored in registry, user sets after login (requires master password re-entry)
- Tertiary: Windows Hello → credential stored in Windows Credential Manager, backend calls platform API
- Logout/Lock: Session.lock() clears vault connection, emits event

**Secrets Management:**
- Vault encryption key: Derived from master password, never stored
- PIN: Hashed (Argon2) and stored in registry, compared on unlock attempt
- SSH private keys: Encrypted in vault with vault key
- TOTP secrets: Encrypted in vault with vault key
- `.env` and credentials files: Not used in this project; all config is in config.json and platform vaults

---

*Architecture analysis: 2026-09-13*
