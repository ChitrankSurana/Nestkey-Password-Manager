# External Integrations

**Analysis Date:** 2026-09-13

## APIs & External Services

**Password Breach Detection:**
- HaveIBeenPwned (HIBP) - Detects compromised passwords
  - SDK/Client: `reqwest@^0.12` (blocking HTTP client in `core/`)
  - API Base URL: `https://api.pwnedpasswords.com`
  - Implementation: K-anonymity range API (only 5-char SHA-1 prefix sent, never full password)
  - Location: `core/src/hibp.rs`
  - Usage: Triggered by `check_breach` command in `src-tauri/src/commands.rs:414`
  - Opt-in: User-controlled via `hibpOptIn` config setting
  - Protocol: HTTPS with rustls TLS

## Data Storage

**Databases:**
- SQLite with SQLCipher encryption
  - Provider: Local file-based SQLite
  - Encryption: AES-256-GCM via bundled SQLCipher
  - Client: `rusqlite@^0.31` with `bundled-sqlcipher-vendored-openssl` feature
  - Location: `~/.pwmgr/registry.db` (encrypted vault database)
  - Path resolution: `dirs@^5` for home directory
  - Code: `core/src/registry.rs`, `core/src/auth.rs`
  - Connection pooling: None (single connection per session)

**File Storage:**
- Local filesystem only
  - Backup location: `~/.pwmgr/backups/`
  - CSV imports: Loaded from user-selected files via file dialogs
  - SSH keys: Stored in encrypted SQLite database (not on filesystem)
  - Implementation: `core/src/backup.rs`, `core/src/portio.rs`

**Caching:**
- None - No caching layer detected
- In-memory session state: `core/src/session.rs` (session-scoped, not persistent)

## Authentication & Identity

**Auth Provider:**
- Custom implementation (no external OAuth/OIDC)
  - Location: `core/src/auth.rs`, `src-tauri/src/biometric_provider.rs`
  - Password-based: Master password with Argon2id hashing
  - Optional PIN: Stored encrypted in database
  - Biometric (Windows only): Windows Hello via DPAPI
  - Biometric Provider: `windows@^0.58` crate with Security_Credentials_UI
  - Implementation: Custom Windows Hello wrapper in `src-tauri/src/biometric_provider.rs`

**Credential Storage:**
- Master password: Derived key via Argon2id
- Database key: Encrypted with derived key
- Individual secrets: AES-256-GCM authenticated encryption
- Sensitive data: Cleared from memory via `zeroize@^1.7` crate

## SSH Integration

**SSH Agent:**
- SSH Agent Protocol Implementation
  - Library: `ssh-agent-lib@^0.6.0`
  - Transport: Windows named pipe (`\\.\pipe\openssh-agent` pattern)
  - Keys: Ed25519 and RSA (2048-bit+) with optional encryption
  - Key parsing: `ssh-key@^0.6` with `ed25519`, `rsa`, `encryption` features
  - Location: `src-tauri/src/ssh_agent_service.rs`
  - Control: Settings toggle `sshAgentEnabled` in `core/src/config.rs`
  - Availability: Windows platform only

## Monitoring & Observability

**Error Tracking:**
- None - No external error reporting detected

**Logs:**
- stderr: Warning/error messages printed to console
  - Example: `eprintln!("warning: failed to emit vault-locked event ({e})")`
  - Location: `src-tauri/src/lib.rs:50`
- No structured logging framework

**Diagnostics:**
- Health report: Local analysis only
  - Command: `health_report` in `src-tauri/src/commands.rs:409`
  - Implementation: `core/src/health.rs`
  - Checks: No external requests

## CI/CD & Deployment

**Hosting:**
- Desktop application (no backend server)
- End-user machines (Windows 10+, macOS, Linux)

**CI Pipeline:**
- Not detected in codebase
- No GitHub Actions, GitLab CI, or other automation files found

**Code Signing:**
- Tauri v2 supports platform-specific signing
- Configuration available but not explicitly configured in codebase

## Environment Configuration

**Required Environment Variables:**
- `TAURI_DEV_HOST` (optional) - Override dev server host binding
  - Used in `vite.config.ts` for HMR configuration
  - Default: localhost if not set

**Secrets Management:**
- No `.env` files in repository
- No environment variable injection for API keys/credentials
- HaveIBeenPwned API: Public, no authentication required (k-anonymity design)
- Database: Encryption key derived from master password at runtime

**Configuration Files:**
- `tauri.conf.json` - Tauri app manifest and build configuration
- `tsconfig.json` - TypeScript compiler options
- `vite.config.ts` - Vite build tool configuration
- `vitest.config.ts` - Test runner configuration

## Webhooks & Callbacks

**Incoming:**
- None - Desktop application, no inbound webhooks

**Outgoing:**
- None - No outbound webhook implementations detected
- Events system: Internal Tauri event emitter only
  - Event: `vault-locked` emitted when vault auto-locks
  - Location: `src-tauri/src/lib.rs:49`

## Data Import/Export

**Import Formats Supported:**
- LastPass CSV - Via `core/src/portio.rs`
- Chromium (Chrome/Edge) - Generic CSV format
- Firefox - Generic CSV format
- Generic CSV - With custom field mapping

**Export Formats:**
- CSV - Plain text export
- Backup (encrypted) - Encrypted vault snapshot
  - Password-protected option available
  - Plaintext option available for backup
  - Location: `core/src/backup.rs`

**CSV Processing:**
- Library: `csv@^1` crate
- Implementation: `core/src/portio.rs`
- Commands: `import_csv_preview`, `import_csv_commit`, `export_csv`
- No external services: All processing local to app

## Security Integrations

**Cryptography:**
- Encryption: AES-256-GCM via `aes-gcm@^0.10`
- Hashing: Argon2id via `argon2@^0.5`
- SHA-1: Via `sha1@^0.10` (for HIBP k-anonymity only)
- HMAC: Via `hmac@^0.12`
- Random: Cryptographically secure via `rand@^0.8`
- Constant-time comparison: Via `subtle@^2.5` (for PIN/password verification)

**Platform-Specific Security:**
- Windows Hello: Multi-factor capable via DPAPI
- SSH keys: RSA and Ed25519 with optional password encryption
- Auto-lock: Session timeout + OS suspend/resume detection (`core/src/autolock.rs`)

## Plugin Dependencies

**Tauri v2 Plugins:**
- `tauri-plugin-clipboard-manager@^2` - Access system clipboard
  - Usage: `copy_to_clipboard` command
  - Platform: All platforms supported

- `tauri-plugin-dialog@^2` - Native file/folder dialogs
  - Usage: Backup/restore file selection, CSV import dialogs
  - Platform: All platforms supported

- `tauri-plugin-opener@^2` - Launch URLs and files
  - Usage: Open URLs from vault entries
  - Platform: All platforms supported

---

*Integration audit: 2026-09-13*
