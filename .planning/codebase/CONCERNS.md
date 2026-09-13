# Codebase Concerns

**Analysis Date:** 2026-09-13

## Tech Debt

### Settings Screen Monolithic Size
- Issue: `src/screens/Settings.tsx` is 589 lines, containing all user-facing configuration UI in a single component with 20+ state variables
- Files: `src/screens/Settings.tsx`
- Impact: Difficult to maintain, test, and modify. Adding new settings requires editing this massive file. Reuse of settings components across screens requires code duplication.
- Fix approach: Extract settings into smaller components (e.g., `SecuritySettingsSection`, `AppearanceSettingsSection`, `SshAgentSettings`) and consider moving complex sub-features (SSH key management, biometric setup) to dedicated screens. This would also make each piece testable in isolation.

### Commands Layer Complexity
- Issue: `src-tauri/src/commands.rs` (416 lines) and `core/src/commands.rs` (2217 lines) are monolithic command dispatchers that directly instantiate handlers. Adding new commands requires both IPC wrapper and core implementation.
- Files: `src-tauri/src/commands.rs`, `core/src/commands.rs`
- Impact: Hard to extend without modifying large files. No clear command hierarchy or logical grouping (auth, vault, config, import/export all mixed). Reuse of similar patterns (error handling, state mutation) is copy-pasted.
- Fix approach: Organize commands into modules by domain (auth::register, vault::add_credential, config::update_theme). Extract common error-handling and state-mutation patterns into shared helpers. Consider a command builder or macro to reduce boilerplate.

### Vault.rs Size and Complexity
- Issue: `core/src/vault.rs` is 1258 lines, bundling database schema, SQL generation, credential encryption, and CSV/backup serialization
- Files: `core/src/vault.rs`
- Impact: Difficult to understand flow and reason about invariants. Schema changes or SQL tweaks require careful review in a large file. CSV/backup logic is tightly coupled to in-memory vault representation.
- Fix approach: Split into modules: `vault/schema.rs` (CREATE TABLE, migrations), `vault/crypto.rs` (encryption/decryption), `vault/import.rs` (CSV/backup parsing), `vault/query.rs` (SELECT helpers), leaving `vault.rs` as a thin coordinator.

### Repetitive Error Handling in React Screens
- Issue: Pattern repeated 25+ times in Settings.tsx alone: `try { ... } catch (err) { setError(isCommandError(err) ? err.message : "Something went wrong...") }`
- Files: `src/screens/Settings.tsx`, `src/screens/CsvImport.tsx`, `src/screens/Login.tsx`, and others
- Impact: Error handling logic scattered across screens. Difficult to change error UI or add logging without touching 20+ files. No consistent error boundary or retry logic.
- Fix approach: Extract a custom hook `useAsyncHandler()` that wraps API calls, handles errors, sets loading state, and triggers toasts/UI feedback. Reduces boilerplate and centralizes error policy.

### State Management Fragmentation
- Issue: Settings.tsx has 20+ individual `useState` calls for related settings (e.g., `autoLockMinutes`, `theme`, `accentColor`, `oldPasswordDays`). Each setting's update is a separate handler function.
- Files: `src/screens/Settings.tsx` (lines 26-42, 78-266)
- Impact: Difficult to handle multi-field validation, atomic updates, or conditional visibility. Adding new settings requires new useState + handler. Hard to test because each state change is isolated.
- Fix approach: Consolidate related settings into a single `useReducer` or create a custom hook that batches updates. E.g., `const [config, updateConfig] = useConfig()` with `updateConfig({ autoLockMinutes: 5, theme: 'light' })`.

---

## Known Bugs

### SSH Agent Feature is Untested in Real Scenarios
- Symptoms: SSH agent over Windows named pipe cannot be verified against a real SSH client. Tooltip at `src/screens/Settings.tsx` line 447 warns "untested against a real SSH client - turn off if it misbehaves."
- Files: `src-tauri/src/ssh_agent_service.rs` (149 lines), `src/screens/Settings.tsx` (SSH Agent section, lines 442-507)
- Trigger: Any SSH client attempting to connect to the Windows named pipe when the SSH agent is enabled
- Workaround: Users can disable the SSH agent toggle in Settings if it causes issues
- Impact: Unknown failure modes with real SSH clients; feature may corrupt key material or hang on certain operations

### Biometric Unlock Feature is New and Needs Field Testing
- Symptoms: Windows Hello integration was recently added (commit 34b0fe0). Biometric failures might silently fall back to password unlock, or lock the account if there's an race condition between successful bio prompt and secondary auth check.
- Files: `src-tauri/src/biometric_provider.rs` (92 lines), `src/screens/Settings.tsx` (biometric section, lines 394-438)
- Trigger: User enables biometric unlock, then uses it on systems with Windows Hello misconfiguration or hardware issues
- Workaround: Disable biometric unlock in Settings and use master password or PIN
- Impact: Unlocking vault may fail silently or hang; potential for confused deputy if auth state is inconsistent after a failed biometric prompt

---

## Security Considerations

### Refer to SECURITY_CONCERNS.md for Detailed Audit
- Risk: This codebase has an existing, comprehensive security audit (dated 2026-07-19) documenting HIGH, MEDIUM, and LOW severity findings.
- Files: `SECURITY_CONCERNS.md` (top-level document)
- Current mitigation: Findings are ranked by severity and annotated with fix approaches. Some are marked as accepted design decisions.
- Recommendations:
  1. **Immediately address H1** (web mode arbitrary file read/write) — disable plaintext CSV export in web mode or confine paths to an app-controlled directory
  2. **Implement M1/M2 together** (rate limiting + memory-exhaustion DoS) — add per-username throttling and concurrency-bounded Argon2 KDF calls
  3. **Add M3 mitigations** (CSP + security headers) — both in axum response layer and tauri.conf.json
  4. **Recover from L1 mutex poisoning** — replace `.lock().unwrap()` with `.lock().unwrap_or_else(|e| e.into_inner())` in hot paths
  5. **L2 secrets zeroization** — wrap vault-layer decrypted secrets in `Zeroizing<String>` end-to-end

### L9 — Committed Repository Narrative Docs
- Risk: Large documentation files committed to repo (`CONVERSATION_EXPORT.md` 177 lines, `RESUME_HERE.md`, `REPO_CONTENTS.md`)
- Files: `CONVERSATION_EXPORT.md`, `RESUME_HERE.md`, `REPO_CONTENTS.md`
- Current mitigation: `.gitignore` excludes test outputs and build artifacts, but not these docs
- Recommendations: Review these files before any public repository sharing to ensure no real credentials, vault paths, or API tokens are embedded. Consider moving to a separate wiki or archive outside version control.

---

## Performance Bottlenecks

### Argon2 Key Derivation on Every Login (M2 Related)
- Problem: Every `login()`, `register_user()`, and password-protected `export_csv()` call runs an Argon2id derivation (64 MiB, t=3, p=4), allocating memory synchronously.
- Files: `core/src/crypto.rs::derive_key`, driven by `core/src/commands.rs` at login/register/export paths
- Cause: No async/concurrent KDF bounds. Burst of login requests from multiple users can allocate 64 MiB each in parallel.
- Improvement path: 
  1. Wrap Argon2 calls in a bounded semaphore to limit concurrent derivations (e.g., max 3 in-flight)
  2. Consider a brief async pause between throttled requests to avoid lock-step memory spikes
  3. Add telemetry to measure KDF wall-clock time and memory peak

### Large Settings State Synchronization
- Problem: Settings.tsx updates 15+ independent state variables, each triggering a separate API call and re-render.
- Files: `src/screens/Settings.tsx` (each `handleChange*` function does `api.update*()`)
- Cause: No batching or debouncing of setting updates. Changing theme, then accent color, then auto-lock triggers 3 separate network round-trips.
- Improvement path:
  1. Batch related settings into a single `updateConfig()` call
  2. Debounce user input (e.g., 500ms) before sending to backend
  3. Use optimistic UI updates so settings feel instant even if network is slow

### Vault Commands Hold Locks for Query Duration
- Problem: `list_sites()`, `search()`, `health_report()` all acquire vault mutex, execute SQL, then release. Long queries block concurrent operations.
- Files: `core/src/commands.rs` (all vault-reading commands), `core/src/state.rs` (mutex holder)
- Cause: No connection pooling or async/await within the lock. Mutex is held for the entire query time.
- Improvement path:
  1. Use async queries with a connection pool (e.g., `sqlx` with a pool) instead of synchronous `rusqlite` under a mutex
  2. Or, release the lock after fetching data and re-acquire only for state mutations
  3. Add query timeouts to prevent a malformed vault from hanging indefinitely

---

## Fragile Areas

### Web Mode Path Handling (H1 in SECURITY_CONCERNS.md)
- Files: `web-src-tauri/src/handlers.rs` (file I/O handlers), `core/src/commands.rs` (CSV/backup logic)
- Why fragile: Client-supplied file paths are passed directly to `std::fs::read`/`std::fs::write` with no canonicalization or confinement. Any path traversal escape (e.g., `../../../etc/passwd`) or symlink race condition can leak or corrupt files.
- Safe modification: 
  1. Never accept absolute or relative caller paths in web mode
  2. Always validate and canonicalize paths, then check they resolve within an app-controlled directory
  3. Use HTTP multipart upload/download instead of server-side paths for imports/exports
  4. Write new features with this constraint baked in from day one

### Biometric and PIN Unlock Flow Consistency
- Files: `src-tauri/src/biometric_provider.rs`, `core/src/auth.rs` (unlock logic), `src/screens/Login.tsx` (UI)
- Why fragile: Biometric and PIN unlock are new features with different code paths than master-password unlock. A race condition or error in one path could leave the session in an inconsistent state (e.g., vault open but session locked, or vice versa).
- Safe modification:
  1. Ensure all unlock paths (master password, PIN, biometric) converge on a single session-opening function
  2. Use integration tests that exercise each unlock method in sequence and verify vault state afterward
  3. Add explicit logging at entry/exit of each unlock path to catch inconsistencies in logs

### CSV Import Parsing
- Files: `core/src/commands.rs::do_import_csv_preview`, `core/src/commands.rs::do_import_csv_commit`
- Why fragile: CSV parsing assumes well-formed input and minimal error handling for malformed records. A corrupted CSV or encoding issue could cause a panic or silent data loss during import.
- Safe modification:
  1. Wrap CSV parsing in a try-catch that returns a detailed error for each malformed row
  2. Add a pre-flight validation step that checks encoding, column count, and sample data types before committing
  3. Test with real-world CSVs from Bitwarden, LastPass, 1Password (already has LastPass support per commit d0ce03b) to catch edge cases

### Mutation Under Session Lock
- Files: `core/src/commands.rs` (most write operations), `core/src/state.rs` (session mutex holder)
- Why fragile: Session lock is held during vault mutations. If a command panics mid-mutation, the session mutex is poisoned and the app is bricked.
- Safe modification:
  1. Minimize lock duration: acquire, check permissions, release, then do I/O
  2. Replace `.lock().unwrap()` with recovery logic (`.lock().unwrap_or_else(|e| e.into_inner())`) in handlers
  3. Audit all `expect()` and `parse().unwrap()` calls on the request path to ensure they cannot panic

---

## Scaling Limits

### Single-Connection Vault Database
- Current capacity: One vault can be open per app instance. All reads/writes are serialized through a single SQLCipher connection under a mutex.
- Limit: Concurrent queries will block each other. Large vaults with thousands of credentials + complex search queries may experience noticeable lag.
- Scaling path:
  1. Migrate from `rusqlite` + `Mutex` to an async query pool (e.g., `sqlx` with a pool of 5-10 connections)
  2. Add query result caching for frequently-accessed lists (e.g., `list_sites()`, recent searches)
  3. Implement incremental sync for web mode instead of full export/import (e.g., SQLite replication or delta compression)

### In-Memory Session Registry
- Current capacity: `registry::RegistryEntry` stores a few fields in RAM per user session. Registry is kept in SQLite, but loaded into memory on app startup.
- Limit: For web mode with many simultaneous users, the registry could grow unbounded if sessions are not reaped (e.g., inactive browser tabs holding open sessions indefinitely).
- Scaling path:
  1. Implement automatic session expiry (e.g., logout if inactive for 1 hour in web mode)
  2. Add a background reaper task that clears stale registry entries periodically
  3. For multi-user deployment, persist sessions to a proper store (Redis, Memcached) instead of in-process SQLite

### Argon2 Memory Footprint with Parallel Logins
- Current capacity: 64 MiB per derivation. No concurrency limit means a burst of 10 login attempts = 640 MiB allocated in parallel.
- Limit: On a machine with limited RAM (e.g., VPS with 1 GB), this could cause an OOM kill or thrashing.
- Scaling path: Implement per-IP and per-username rate limiting (M1) + concurrency cap on KDF (M2). See SECURITY_CONCERNS.md for details.

---

## Dependencies at Risk

### sqlcipher Vendored OpenSSL
- Risk: `rusqlite` is compiled with `bundled-sqlcipher-vendored-openssl` feature, bundling OpenSSL into the binary. This means security patches to OpenSSL require a full rebuild and release of the app.
- Impact: If a critical OpenSSL vulnerability is discovered, users may be at risk until an app update is released and installed.
- Migration plan:
  1. Monitor OpenSSL releases and security advisories
  2. Automate Cargo.lock updates and test builds weekly
  3. Consider switching to `rustls` (pure Rust TLS) if feasible to eliminate the OpenSSL dependency
  4. Publish security updates out-of-band if a critical vuln is found

### ssh-agent-lib and ssh-key Ecosystem
- Risk: SSH key handling relies on external crates (`ssh-agent-lib` 0.6.0, `ssh-key` 0.6). SSH is a cryptographic system; any library bug could leak or corrupt keys.
- Impact: Private key material could be read by a malicious SSH client or exposed in error messages if deserialization fails.
- Migration plan:
  1. Keep SSH agent feature gated behind a user-enabled toggle (already done)
  2. Audit `ssh-key` deserialization paths and add fuzz testing for malformed keys
  3. Ensure private keys are zeroized after use (check `drop` impls on `ssh-key::PrivateKey`)
  4. Consider committing to a long-term SSH key library (e.g., contributing to `openssh-keys` or pinning to a stable version long-term)

### Argon2 Library Updates
- Risk: `argon2` 0.5 is used for key derivation. A bug in the library could weaken password verification or allow timing attacks.
- Impact: Master password authentication could be bypassed or timing-attacked.
- Migration plan:
  1. Audit Argon2 call sites to ensure parameters are consistent with OWASP recommendations
  2. Monitor `argon2` releases and upgrade promptly if a vulnerability is found
  3. Keep `constant_time_eq` comparison in place (already using `subtle` crate, good)

### zxcvbn Password Strength
- Risk: `zxcvbn` 3 is used to estimate password entropy. The library is maintained by Dropbox but has infrequent updates.
- Impact: Password strength estimates may become inaccurate if the library is not updated when attack techniques evolve.
- Migration plan:
  1. Make password strength advisory only (already the case per SECURITY_CONCERNS.md L8)
  2. Monitor `zxcvbn` releases; upgrade annually or when major new attack patterns emerge
  3. Consider supplementing with local "common password list" if the library becomes stale

---

## Missing Critical Features

### No End-to-End Test Coverage for Auth Flows
- Problem: Only 2 test files in the entire project (`clipboard.test.ts`, `passwordColor.test.ts`). Zero tests for login, registration, unlock, biometric flow, or credential mutations.
- Blocks: Cannot confidently deploy changes to auth or unlock logic. Manual testing required for every security-sensitive change.
- Impact: Auth regressions could slip through undetected (e.g., a typo in session validation breaks unlock). Biometric and PIN features are untested in CI.
- Priority: **HIGH** — should be addressed before any production release

### No Integration Tests for Web Mode
- Problem: Web mode (`web-src-tauri`) handler layer has no automated tests. File path validation, session cookie handling, and HTTP endpoint security are not tested.
- Blocks: Cannot validate that H1 (file path vulnerabilities) is actually fixed. Security fixes are trust-based rather than evidence-based.
- Impact: Regressions in file path handling, CSRF, or session management could reintroduce HIGH-severity vulnerabilities.
- Priority: **CRITICAL** — must be in place before web mode is used with real data

### No Database Schema Migrations
- Problem: Vault and registry schemas are hardcoded in `vault.rs` and `registry.rs`. No migration system for schema changes.
- Blocks: Cannot safely evolve the database without manual user intervention or data loss (e.g., adding a column requires a backup, schema update, restore).
- Impact: Future features that require schema changes (e.g., sharing, multi-vault) are risky to implement.
- Priority: **MEDIUM** — not urgent, but should be in place before major features are added

### No Offline/Encrypted Sync
- Problem: No way to sync vault changes between devices. Backup/restore is manual export/import only.
- Blocks: Users cannot keep multiple devices in sync. Shared vaults or team features are not possible.
- Impact: Feature requests for multi-device sync cannot be fulfilled without a major rewrite.
- Priority: **LOW** — not needed for MVP, but limits long-term product vision

---

## Test Coverage Gaps

### React Screens Not Tested
- What's not tested: All 14 screen components (`Login.tsx`, `Register.tsx`, `Vault.tsx`, `SiteDetail.tsx`, `CredentialEditor.tsx`, `Settings.tsx`, `CsvImport.tsx`, `CsvExport.tsx`, `HealthDashboard.tsx`, `Trash.tsx`, `Backup.tsx`, `NavRail.tsx`, `NavDrawer.tsx`, `ViewerVault.tsx`)
- Files: `src/screens/*.tsx`
- Risk: UI regressions, state management bugs, or navigation flows could break undetected. Settings mutations could fail silently due to race conditions.
- Priority: **HIGH** — start with critical flows (Login, CredentialEditor, Vault list/search, Settings)

### Commands Layer Not Tested
- What's not tested: All 40+ API commands in `src-tauri/src/commands.rs` and `core/src/commands.rs` (auth, vault CRUD, health checks, CSV import/export, backup/restore, biometric, PIN, SSH, config)
- Files: `src-tauri/src/commands.rs`, `core/src/commands.rs`
- Risk: Command-layer bugs (off-by-one errors, missing permission checks, state mutations) are only caught by manual testing. Each release is high-risk.
- Priority: **CRITICAL** — must have unit tests for all commands before production use

### Vault Database Operations Not Tested
- What's not tested: Schema creation, SQL queries (list_sites, search, insert credential, soft delete), constraint enforcement, transaction safety
- Files: `core/src/vault.rs`
- Risk: Data corruption, orphaned records, or query failures could occur in production without being caught by tests.
- Priority: **CRITICAL** — need comprehensive integration tests using a temporary SQLite file

### CSV Import/Export Parsing Not Tested
- What's not tested: CSV header detection, mapping to vault fields, duplicate detection, encoding handling, malformed row handling for each supported format (Chromium, Firefox, LastPass, generic)
- Files: `core/src/commands.rs::do_import_csv_preview`, `do_import_csv_commit`, `do_export_csv`, `do_read_csv_headers`
- Risk: Import with real-world CSVs (from Bitwarden, LastPass, 1Password) could fail silently or lose data without test coverage.
- Priority: **HIGH** — add test fixtures for each import format before release

### Crypto Operations Not Tested
- What's not tested: Key derivation, encryption/decryption round-trips, constant-time comparison, nonce generation, edge cases (empty credential, max-length field)
- Files: `core/src/crypto.rs`, `core/src/vault.rs` (crypto layer)
- Risk: A subtle bug in encryption could lead to plaintext leaks or inability to decrypt vaults.
- Priority: **CRITICAL** — need comprehensive cryptographic tests (unit + property-based)

### Biometric and PIN Unlock Not Tested
- What's not tested: Windows Hello provider interactions, PIN hashing, biometric fallback logic, session state after unlock, race conditions between multiple unlock attempts
- Files: `src-tauri/src/biometric_provider.rs`, `core/src/auth.rs::unlock_with_pin`, `core/src/auth.rs::unlock_with_biometric`
- Risk: Unlock could fail or leave session in inconsistent state without tests. Feature is known to be untested.
- Priority: **CRITICAL** — cannot ship this feature without integration tests

### SSH Agent Not Tested
- What's not tested: Named pipe creation, key serialization, SSH protocol compliance, connection handling, error cases (invalid key, client disconnect)
- Files: `src-tauri/src/ssh_agent_service.rs`, `core/src/ssh_keys.rs`
- Risk: Feature is explicitly marked "untested against a real SSH client." Real-world SSH clients may hang or corrupt key material.
- Priority: **HIGH** — feature should remain experimental (toggle-only) until tested with at least one real SSH client

---

*Concerns audit: 2026-09-13*
