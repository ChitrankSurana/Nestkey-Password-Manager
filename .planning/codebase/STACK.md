# Technology Stack

**Analysis Date:** 2026-09-13

## Languages

**Primary:**
- TypeScript 5.8.3 - Frontend application (`src/`)
- Rust 2021 edition - Backend/desktop application (`src-tauri/`, `core/`)

**Secondary:**
- HTML5 - UI markup (`index.html`, `viewer.html`)
- CSS3 - Styling with Tailwind CSS (`src/`)

## Runtime

**Environment:**
- Node.js (version not pinned, assumed LTS 18+)
- Tauri v2 - Desktop/native runtime for both frontend and backend

**Package Manager:**
- npm - JavaScript dependencies
- Cargo - Rust dependencies

**Lockfiles:**
- `package-lock.json` - npm lockfile present
- `Cargo.lock` - Rust lockfile present

## Frameworks

**Core:**
- Tauri v2 - Cross-platform desktop framework with Rust backend, `src-tauri/`
- React 19.1.0 - UI library, `src/`
- Vite 7.0.4 - Build tool and dev server, configured via `vite.config.ts`

**Styling:**
- Tailwind CSS 4.3.2 - Utility-first CSS framework, integrated via `@tailwindcss/vite`
- Bootstrap Icons 1.13.1 - Icon library

**Testing:**
- Vitest 4.1.10 - Unit test framework for TypeScript/JavaScript

## Key Dependencies

**Frontend (TypeScript/React):**
- `@tauri-apps/api@^2` - Tauri API bindings for IPC with Rust backend
- `@tauri-apps/plugin-dialog@^2.7.1` - Native file/folder dialogs
- `@tauri-apps/plugin-opener@^2` - Launch URLs and files with native applications
- `react-dom@^19.1.0` - React DOM rendering
- `@vitejs/plugin-react@^4.6.0` - Vite plugin for React

**Frontend (Development):**
- `@types/react@^19.1.8` - TypeScript types for React
- `@types/react-dom@^19.1.6` - TypeScript types for React DOM
- `@tauri-apps/cli@^2` - Tauri CLI for building/running

**Backend (Rust - Tauri):**
- `tauri@^2` - Core Tauri framework
- `tauri-plugin-clipboard-manager@^2` - Clipboard access plugin
- `tauri-plugin-dialog@^2` - Native dialogs plugin
- `tauri-plugin-opener@^2` - File/URL opener plugin
- `ssh-agent-lib@^0.6.0` - SSH agent protocol implementation
- `ssh-key@^0.6` - SSH key parsing and management (supports Ed25519, RSA with encryption)
- `dirs@^5` - Platform-specific home directory resolution
- `tokio@^1` - Async runtime for background tasks
- `zeroize@^1.7` - Secure memory clearing for sensitive data
- `serde@^1` - Serialization/deserialization (with derive macros)
- `serde_json@^1` - JSON handling

**Backend (Rust - Core Library `pwmgr-core`):**
- `argon2@^0.5` - Argon2id password hashing algorithm
- `aes-gcm@^0.10` - AES-256-GCM authenticated encryption
- `rand@^0.8` - Cryptographically secure random number generation
- `rusqlite@^0.31` - SQLite bindings with bundled SQLCipher for encryption
- `csv@^1` - CSV import/export parsing
- `reqwest@^0.12` - HTTP client (blocking mode, TLS with rustls)
- `sha1@^0.10` - SHA-1 hashing (for HaveIBeenPwned API k-anonymity)
- `hmac@^0.12` - HMAC implementation
- `data-encoding@^2` - Hex encoding
- `zxcvbn@^3` - Password strength analysis
- `thiserror@^1.0` - Error handling ergonomics
- `subtle@^2.5` - Constant-time comparison primitives

**Platform-Specific (Windows):**
- `windows@^0.58` - Windows API bindings for:
  - Windows Hello biometric authentication (`Security_Credentials_UI`)
  - Win32 Security/Cryptography APIs (`Win32_Security_Cryptography`)
  - Foundation classes (`Win32_Foundation`)

## Configuration

**TypeScript:**
- Strict mode enabled (`strict: true`)
- Target: ES2020
- Module: ESNext with bundler resolution
- JSX: React 17+ new transform (`react-jsx`)
- Linting: `noUnusedLocals`, `noUnusedParameters`, `noFallthroughCasesInSwitch`
- Config file: `tsconfig.json`

**Vite:**
- Dev server port: 1420
- Multiple entry points: `index.html` (main), `viewer.html` (viewer window)
- React plugin with Tailwind CSS plugin integration
- Tauri-aware: ignores `src-tauri/` and `viewer-src-tauri/` during watch
- Config file: `vite.config.ts`

**Tauri:**
- Desktop app configuration: `src-tauri/tauri.conf.json`
  - Identifier: `com.pwmgr.app`
  - Single primary window (800x600)
  - CSP: null (disabled for Tauri)
- Build artifacts: `dist/` (frontend) bundled into Tauri app
- Dev URL: `http://localhost:1420` (Vite dev server)

**Rust:**
- Edition: 2021
- Crate types: staticlib, cdylib, rlib (for Tauri compatibility)
- No feature flags enabled on core dependencies by default

**Testing:**
- Vitest with Node.js environment
- Test files: `src/**/*.test.ts`
- Config file: `vitest.config.ts`

## Platform Requirements

**Development:**
- Windows 11 Pro (observed in environment)
- Node.js LTS 18+ (required for npm and Vite)
- Rust toolchain 2021+ (required for Cargo build)
- PowerShell 7+ or Git Bash (for shell commands)

**Production:**
- Windows 10+, macOS 11+, or Linux (Tauri v2 supports all major platforms)
- Desktop application bundle (single .exe on Windows)

## Build & Deployment

**Development Build:**
```bash
npm run dev                    # Runs Vite dev server
tauri dev                      # Launches Tauri in dev mode
```

**Production Build:**
```bash
npm run build                  # Compiles TypeScript + Vite build
tauri build                    # Creates platform-specific installer
```

**Pre-build Steps:**
- TypeScript compilation (`tsc`)
- Tauri CLI build process includes:
  - Cargo build of Rust backend
  - Bundling frontend dist into app
  - Code signing (configurable per platform)

---

*Stack analysis: 2026-09-13*
