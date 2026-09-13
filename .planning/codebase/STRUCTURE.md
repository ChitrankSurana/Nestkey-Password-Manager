# Codebase Structure

**Analysis Date:** 2026-09-13

## Directory Layout

```
Password_Manager/
├── src/                        # React frontend (TypeScript)
│   ├── App.tsx                 # Root component, auth flow router
│   ├── main.tsx                # React DOM entry point
│   ├── App.css                 # Global styles
│   ├── screens/                # Full-page UI components
│   │   ├── AppShell.tsx        # Main vault shell, nav + content
│   │   ├── Login.tsx           # Login/unlock screen
│   │   ├── Register.tsx        # Account registration
│   │   ├── Vault.tsx           # Credential list + search
│   │   ├── SiteDetail.tsx      # Side panel for credential details
│   │   ├── SiteEditor.tsx      # Inline site add/edit modal
│   │   ├── CredentialEditor.tsx# Inline credential add/edit modal
│   │   ├── Settings.tsx        # User config panel
│   │   ├── HealthDashboard.tsx # Password strength analysis
│   │   ├── Generator.tsx       # Password generation UI
│   │   ├── Backup.tsx          # Backup creation UI
│   │   ├── CsvExport.tsx       # CSV export dialog
│   │   ├── CsvImport.tsx       # CSV import preview/confirm
│   │   ├── Data.tsx            # Data management hub
│   │   ├── Trash.tsx           # Soft-deleted credentials
│   │   ├── NavRail.tsx         # Desktop navigation sidebar
│   │   ├── NavDrawer.tsx       # Mobile drawer navigation
│   │   ├── BottomTabBar.tsx    # Mobile bottom tab bar
│   │   └── ViewerVault.tsx     # Separate viewer app vault UI
│   ├── components/             # Reusable UI components
│   │   ├── Icon.tsx            # Bootstrap icon wrapper
│   │   ├── SecretField.tsx     # Password field with show/hide toggle
│   │   ├── PasswordGeneratorPanel.tsx  # Generator options UI
│   │   ├── TotpDisplay.tsx     # TOTP code + timer display
│   │   ├── ToastHost.tsx       # Toast notification container
│   │   └── Tooltip.tsx         # Hover tooltip
│   ├── hooks/                  # Custom React hooks
│   │   └── useActivityWatcher.ts# Detect user inactivity for auto-lock
│   ├── lib/                    # Utility functions
│   │   ├── api.ts              # Tauri IPC / HTTP dispatch, DTO contracts
│   │   ├── clipboard.ts        # Copy-to-clipboard abstraction
│   │   ├── passwordColor.ts    # Color code password strength
│   │   ├── strength.ts         # Password strength display logic
│   │   ├── toast.ts            # Show toast notifications
│   │   └── *.test.ts           # Unit tests (co-located)
│   └── assets/                 # Static images/SVGs
│
├── src-tauri/                  # Tauri backend (Rust)
│   ├── src/
│   │   ├── main.rs             # App entry point (calls lib::run())
│   │   ├── lib.rs              # Tauri app initialization, background tasks
│   │   ├── commands.rs         # Tauri IPC command handlers (~50 commands)
│   │   ├── biometric_provider.rs# Windows Hello provider
│   │   └── ssh_agent_service.rs# SSH agent over named pipe
│   ├── Cargo.toml              # Dependencies
│   └── capabilities/           # Tauri security capabilities
│
├── core/                       # Shared Rust library (reusable across binaries)
│   ├── src/
│   │   ├── lib.rs              # Module exports
│   │   ├── main.rs             # (none; this is a library)
│   │   ├── state.rs            # AppState container
│   │   ├── commands.rs         # Business logic for ~50 IPC operations
│   │   ├── vault.rs            # Vault DB schema, encrypt/decrypt
│   │   ├── auth.rs             # Master password, PIN auth
│   │   ├── session.rs          # Session state machine
│   │   ├── autolock.rs         # Auto-lock timeout logic
│   │   ├── crypto.rs           # AES-GCM, Argon2, key derivation
│   │   ├── dto.rs              # Data Transfer Objects (API contracts)
│   │   ├── error.rs            # Error types
│   │   ├── registry.rs         # Registry DB (user list)
│   │   ├── paths.rs            # Platform paths (~/.pwmgr)
│   │   ├── config.rs           # Config.json model
│   │   ├── generator.rs        # Password generation
│   │   ├── health.rs           # Password health analysis
│   │   ├── totp.rs             # TOTP generation
│   │   ├── hibp.rs             # Have I Been Pwned integration
│   │   ├── backup.rs           # Backup/restore logic
│   │   ├── ssh_agent.rs        # SSH agent protocol core
│   │   ├── ssh_keys.rs         # SSH key management
│   │   ├── biometric.rs        # Biometric auth abstraction
│   │   ├── portio.rs           # CSV import/export
│   │   └── tests/ (or *.rs with #[test]) # Unit tests
│   ├── Cargo.toml              # Dependencies (crypto, DB, HTTP)
│   └── tests/                  # Integration tests
│
├── viewer-src-tauri/           # Separate viewer app (uses same core)
│   ├── src/
│   │   ├── main.rs
│   │   ├── lib.rs
│   │   └── commands.rs
│   └── Cargo.toml
│
├── web-src-tauri/              # Web variant (uses same core, HTTP backend)
│   ├── src/
│   │   ├── main.rs
│   │   └── ...
│   └── Cargo.toml
│
├── public/                     # Static assets (index.html, favicon, etc.)
│   └── (Vite static files)
│
├── docs/                       # Documentation
│   ├── FIGMA_UI_VS_CURRENT_FEATURES.md  # UI design comparison
│   ├── Plans/                  # Planning docs
│   ├── testing/                # Test documentation
│   └── ...
│
├── scripts/                    # Build/utility scripts
│   └── (custom tooling, if any)
│
├── target/                     # Rust build output (debug, release)
│   ├── debug/
│   └── release/
│
├── dist/                       # Frontend build output
│   ├── (compiled React + static assets)
│
├── .planning/                  # GSD planning artifacts
│   └── codebase/               # Codebase analysis docs (ARCHITECTURE.md, STRUCTURE.md, etc.)
│
├── .vscode/                    # VS Code workspace settings
│   └── (launch configs, etc.)
│
├── .claude/                    # Claude Code project settings
│   └── (settings.json, skills, etc.)
│
├── package.json                # Frontend dependencies (React, Tauri, Vite)
├── tsconfig.json               # TypeScript config
├── vite.config.ts              # Vite build config
├── index.html                  # Main app entry
├── viewer.html                 # Viewer app entry
└── Cargo.lock                  # Rust dependency lock
```

## Directory Purposes

**`src/`:**
- Purpose: React TypeScript frontend application code
- Contains: Components, screens, hooks, utilities, styles
- Key files: `App.tsx` (router), `screens/AppShell.tsx` (layout), `lib/api.ts` (IPC abstraction)

**`src/screens/`:**
- Purpose: Full-screen UI views
- Contains: Login, Vault, Settings, HealthDashboard, Generator, Backup/Export/Import, Trash, Navigation components
- Note: Each file is a React functional component that manages its own state and calls backend APIs

**`src/components/`:**
- Purpose: Reusable UI building blocks
- Contains: Icon wrapper, SecretField (password input), TotpDisplay, ToastHost, Tooltip
- Pattern: Small, focused, no API calls

**`src/hooks/`:**
- Purpose: Custom React hooks
- Contains: `useActivityWatcher` for auto-lock timeout detection
- Pattern: Encapsulate side effects and stateful logic

**`src/lib/`:**
- Purpose: Utility functions and API abstraction
- Key: `api.ts` is the single dispatch point for all backend communication (Tauri IPC or HTTP)
- Contains: DTOs, error guards, clipboard utilities, password strength coloring, toast system

**`src-tauri/src/`:**
- Purpose: Tauri backend entry point and command handlers
- Key files:
  - `lib.rs`: App initialization, AppState creation, background auto-lock task, Tauri plugin setup
  - `commands.rs`: ~50 IPC handlers that wrap `do_*` functions from `pwmgr_core`
  - `biometric_provider.rs`: Windows Hello integration
  - `ssh_agent_service.rs`: SSH agent server over named pipe
- Pattern: Thin wrapper layer; all business logic lives in core/

**`core/src/`:**
- Purpose: Shared Rust business logic library, reusable across multiple binaries (src-tauri, viewer-src-tauri, web-src-tauri)
- Key modules:
  - **State:** `state.rs` (AppState), `session.rs` (auth state), `config.rs` (user settings)
  - **Vault:** `vault.rs` (database schema, encrypt/decrypt), `registry.rs` (user list DB)
  - **Auth:** `auth.rs` (master password, PIN), `biometric.rs` (Windows Hello)
  - **Commands:** `commands.rs` (50+ business logic functions, each prefixed `do_*`)
  - **Crypto:** `crypto.rs` (encryption, key derivation, nonce generation)
  - **Features:** `generator.rs` (password generation), `totp.rs` (TOTP), `health.rs` (password analysis), `hibp.rs` (breach checking), `backup.rs`, `portio.rs` (CSV)
  - **Data:** `dto.rs` (API contract types), `error.rs` (error types)
- Pattern: Pure business logic; no Tauri or framework dependencies

**`public/`:**
- Purpose: Static web assets served by Vite / Tauri
- Contains: `index.html` (main app), `viewer.html` (viewer app), favicon, manifest, etc.

**`docs/`:**
- Purpose: Project documentation
- Contains: UI design comparisons, testing guides, planning notes

**`target/` and `dist/`:**
- Purpose: Build artifacts (do not commit)
- `target/`: Rust compilation output
- `dist/`: Compiled React + JavaScript + assets

**`.planning/codebase/`:**
- Purpose: GSD tool output — codebase analysis documents
- Contains: ARCHITECTURE.md, STRUCTURE.md, CONVENTIONS.md, TESTING.md, STACK.md, INTEGRATIONS.md, CONCERNS.md

## Key File Locations

**Entry Points:**

| File | Purpose | Invoked By |
|------|---------|-----------|
| `src/main.tsx` | React app entry point | Vite / index.html |
| `src-tauri/src/main.rs` | Tauri desktop app entry | `tauri dev` or binary executable |
| `viewer.html` + `src/` | Separate viewer build | Vite viewer build target |
| `public/index.html` | Static HTML template | Vite injects React app here |

**Configuration:**

| File | Purpose |
|------|---------|
| `package.json` | Frontend dependencies, build scripts |
| `tsconfig.json` | TypeScript compiler options |
| `vite.config.ts` | Build tool config (plugins, ports, outputs) |
| `core/Cargo.toml` | Core library dependencies (crypto, DB, HTTP) |
| `src-tauri/Cargo.toml` | Tauri-specific dependencies (Tauri, plugins, SSH agent) |
| `~/.pwmgr/config.json` | Runtime user config (theme, auto-lock, HIBP opt-in) [generated at runtime] |
| `~/.pwmgr/registry.db` | SQLite DB of all users (usernames, salts, PINs, vault paths) [generated at first login] |

**Core Logic:**

| File | Scope |
|------|-------|
| `core/src/commands.rs` | ~2000 LOC; all business logic for vault operations, auth, import/export, health checks |
| `core/src/vault.rs` | ~400 LOC; vault DB schema, encryption/decryption, CRUD |
| `core/src/auth.rs` | ~200 LOC; master password, PIN, biometric setup |
| `core/src/session.rs` | ~100 LOC; auth state machine |
| `core/src/crypto.rs` | ~300 LOC; AES-GCM, Argon2, key derivation |
| `core/src/autolock.rs` | ~100 LOC; timeout detection |
| `src-tauri/src/commands.rs` | ~500 LOC; thin wrappers, all handlers listed |
| `src-tauri/src/lib.rs` | ~115 LOC; app init, background task spawn |

**Testing:**

| File | Type |
|------|------|
| `src/lib/*.test.ts` | Frontend unit tests (co-located with implementation) |
| `core/src/*.rs` | Backend tests (inline `#[cfg(test)]` modules and separate `tests/` dir) |

## Naming Conventions

**Files:**

- Components: `PascalCase.tsx` (e.g., `Vault.tsx`, `SecretField.tsx`)
- Utilities: `camelCase.ts` (e.g., `api.ts`, `clipboard.ts`, `passwordColor.ts`)
- Rust modules: `snake_case.rs` (e.g., `commands.rs`, `vault.rs`, `biometric_provider.rs`)
- Tests: `*test.ts` or `*_test.rs` suffix (e.g., `clipboard.test.ts`, `vault_test.rs`)

**Directories:**

- UI components: `src/screens/` (full screens), `src/components/` (reusable pieces)
- Frontend utilities: `src/lib/` (non-component code)
- React hooks: `src/hooks/`
- Static assets: `public/`, `src/assets/`
- Rust backend: `src-tauri/src/`, `core/src/`

**Functions:**

- React components: `PascalCase` (e.g., `function Vault() {}`)
- Utility functions: `camelCase` (e.g., `function analyzeStrength() {}`)
- Rust commands (IPC): `snake_case` exposed to Tauri, `camelCase` in frontend (e.g., `#[tauri::command] pub fn register_user()` called via `api.registerUser()`)
- Core business logic: Prefixed with `do_` (e.g., `do_register()`, `do_login()`, `do_add_credential()`) to distinguish IPC handlers from core logic

**Variables:**

- React state: `camelCase` (e.g., `const [selectedSiteId, setSelectedSiteId]`)
- Rust state: `snake_case` (e.g., `let vault_key = derive_key()`)
- DTOs: `PascalCase` with `Dto` suffix (e.g., `CredentialDto`, `ConfigDto`)
- Constants: `SCREAMING_SNAKE_CASE` in Rust; normal camelCase in TypeScript

**Types:**

- TypeScript: `PascalCase` (e.g., `interface AuthSuccess`, `type Screen = "login" | "register"`)
- Rust enums: `PascalCase` variants (e.g., `enum VaultError { UnlockFailed, NotFound }`)

## Where to Add New Code

**New Feature: Vault Operation (e.g., "Add field to credential")**

1. **Backend business logic:**
   - Add `do_add_field()` function in `core/src/commands.rs`
   - Define any new error cases in `core/src/error.rs`
   - Update vault schema in `core/src/vault.rs` if needed (add table column, update `init_schema()`)

2. **Backend IPC handler:**
   - Add `#[tauri::command]` function in `src-tauri/src/commands.rs`
   - Call the core `do_*` function
   - Register in `src-tauri/src/lib.rs` `invoke_handler!` macro

3. **Frontend API abstraction:**
   - Add method to `api` object in `src/lib/api.ts`
   - Define DTO types if needed
   - Use `call()` dispatch (auto-routes to Tauri IPC or HTTP)

4. **Frontend UI:**
   - Add button/UI in relevant screen (e.g., `src/screens/SiteDetail.tsx`)
   - Call `api.addField()` with form data
   - Handle errors, show toast on success, refetch data

5. **Tests:**
   - Add test in `core/src/commands.rs` (#[test] function)
   - Test error cases (vault not open, invalid input, etc.)

**Example pattern:** See `do_add_credential()` → `commands::add_credential()` → `CredentialEditor.tsx`

---

**New UI Component:**

1. **Reusable component:**
   - Create `src/components/MyComponent.tsx`
   - Export as `function MyComponent() {}`
   - No API calls; accept callbacks as props

2. **Screen/page:**
   - Create `src/screens/MyScreen.tsx`
   - Can call `api.*` methods
   - Accept `onBack()` or navigation callbacks from parent (AppShell)
   - Use standard error handling: `try/catch`, extract `isCommandError()`, set state

3. **Integration in AppShell:**
   - Add new entry in `destinations` array in `src/screens/AppShell.tsx`
   - Add conditional render: `{screen === "myscreen" && <MyScreen ... />}`
   - Update `type ShellScreen` to include `"myscreen"`

**Example pattern:** See `Generator.tsx` screen and how it's integrated in `AppShell.tsx`

---

**New Backend Module (e.g., sync functionality):**

1. **Create module file:**
   - Add `core/src/sync.rs` with your functions
   - Export in `core/src/lib.rs`: `pub mod sync;`

2. **Add command wrapper:**
   - Add `do_sync()` function calling your module
   - Create IPC handler in `src-tauri/src/commands.rs`
   - Register in `src-tauri/src/lib.rs`

3. **Frontend integration:**
   - Add `api.sync()` in `src/lib/api.ts`
   - Call from appropriate screen

**Example pattern:** See `ssh_keys.rs` module (used by Settings screen)

---

**Utilities:**

- Password strength: Add function to `src/lib/strength.ts` or `passwordColor.ts`
- Toast notifications: Already exists; use `showToast()` from `src/lib/toast.ts` in any component
- Clipboard: Already abstracted; use `api.copyToClipboard(text)` in any component

## Special Directories

**`target/` and `dist/`:**
- Purpose: Build artifacts (generated, not committed)
- Generated by: `npm run build` (React), `cargo build` (Rust)
- Committed: No (.gitignore excludes these)

**`.git/`, `.worktrees/`:**
- Purpose: Git repository (worktrees used for parallel branch work)
- Committed: Yes (.git), but .worktrees is ignored

**`node_modules/`:**
- Purpose: NPM package dependencies
- Generated by: `npm install`
- Committed: No (.gitignore)

**`.planning/codebase/`:**
- Purpose: GSD codebase analysis output
- Generated by: `/gsd-map-codebase` skill
- Committed: Yes (source of truth for architecture documentation)

**`.vscode/`:**
- Purpose: VS Code workspace settings (launch configs, extensions, etc.)
- Committed: Selectively (settings.json yes, extensions.json yes, but not local settings)

**`.claude/`:**
- Purpose: Claude Code project configuration (settings.json, skills, etc.)
- Committed: Yes (project-specific conventions and configurations)

---

*Structure analysis: 2026-09-13*
