# 🔐 Password Manager

A **local-first, zero-knowledge password manager**. Every credential lives only on your machine,
inside an encrypted vault file, unlockable only with a master password that the software itself
never stores, logs, or transmits. No account, no cloud sync, no telemetry — just one file you own.

Built with **Tauri (Rust) + React/TypeScript**, backed by SQLCipher-encrypted SQLite with an extra
layer of per-field AES-256-GCM, and Argon2id for master-password key derivation.

## Why this exists

Cloud password managers ask you to trust a third party's servers with the single most sensitive
file you own. This project takes the opposite bet: **one encrypted file, three ways to reach it**,
all running the exact same audited Rust core — nothing about your vault ever needs to touch a
network you don't control.

## The three apps

This repo builds **three separate binaries** from one shared Rust core (`pwmgr-core`), so "read-only"
and "no write commands exist in this binary" are actual compile-time facts, not just UI restrictions.

| | Main App | Viewer App | Web Portal |
|---|---|---|---|
| **What it's for** | Everyday full read/write use | Emergency read-only access if the main app is unavailable | Browser access on your own machine, no desktop window |
| **Binary / crate** | `pwmgr-app` (`src-tauri`) | `pwmgr-viewer` (`viewer-src-tauri`) | `pwmgr-web` (`web-src-tauri`) |
| **Interface** | Tauri desktop window | Tauri desktop window (`viewer.html`) | `http://127.0.0.1:7373` in your browser |
| **Can write?** | Yes — full CRUD, generator, backup/CSV, health dashboard | No — view, search, copy only. No add/edit/delete commands are even registered | Yes — full command set, over HTTP |
| **Network exposure** | None (IPC only) | None (IPC only) | Bound to `127.0.0.1` only, never `0.0.0.0` |

## Features

- **Encrypted vault** — SQLCipher full-database encryption + independent per-field AES-256-GCM,
  keyed by an Argon2id-derived master key that is never written to disk.
- **Fast, real-time search** across site names, usernames, and remarks.
- **Password generator** — four methods: random (character-class control), memorable (word list +
  separator), pattern (template-based), pronounceable — with a live strength meter (simple entropy
  or zxcvbn, your choice).
- **Health Dashboard** — flags weak, reused, and old passwords, plus an opt-in
  [Have I Been Pwned](https://haveibeenpwned.com/) breach check using k-anonymity (only 5 hex chars
  of a SHA-1 hash ever leave the app).
- **Soft-delete Trash** — deleted credentials are recoverable until you empty Trash.
- **Encrypted backup/restore**, and **CSV export/import** — export as this-device-only, password-
  protected, or explicitly-confirmed plaintext; import from Chrome/Edge, Firefox, or any CSV via a
  generic column mapper.
- **Theming** — light/dark, five accent colors.
- **Keyboard-accessible tooltips** on every control.
- **Auto-lock** on idle, with a manual Lock button, and full key zeroization on lock.

## Quick start

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;C:\Strawberry\perl\bin;C:\Strawberry\c\bin;" + $env:PATH
npm install
npm run tauri dev        # Main App
```

See [`docs/GETTING-STARTED`](docs/HOW_TO_USE_MAIN_APP.md) below for the other two apps, and
[`docs/testing/`](docs/testing/) for step-by-step manual test walkthroughs of all three.

## Docs

| Doc | What's in it |
|---|---|
| [`docs/PRD.md`](docs/PRD.md) | What we're building and why |
| [`docs/TRD.md`](docs/TRD.md) | How it's built: stack, crypto design, data model, module breakdown |
| [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) | System architecture and key data flows |
| [`docs/THREAT_MODEL.md`](docs/THREAT_MODEL.md) | Assets, adversaries, mitigations, accepted risks |
| [`docs/HOW_TO_USE_MAIN_APP.md`](docs/HOW_TO_USE_MAIN_APP.md) | End-user guide — Main App |
| [`docs/HOW_TO_USE_VIEWER_APP.md`](docs/HOW_TO_USE_VIEWER_APP.md) | End-user guide — Viewer App |
| [`docs/HOW_TO_USE_WEB_PORTAL.md`](docs/HOW_TO_USE_WEB_PORTAL.md) | End-user guide — Web Portal |
| [`docs/testing/`](docs/testing/) | Step-by-step manual test walkthroughs, one per app |
| [`docs/KNOWN_LIMITATIONS.md`](docs/KNOWN_LIMITATIONS.md) | Deferred items and accepted non-goals |

## Status

**M1–M5 and M6's web-mode half are complete and merged.** M6's responsive-layout half is
spec'd and planned but not yet implemented. See `RESUME_HERE.md` for the exact in-progress state.

| Milestone | Delivers | Status |
|---|---|---|
| M1 | Register/login, encrypted vault CRUD, trash, search, auto-lock, dark theme, tooltips | ✅ Done |
| M2 | Password generator (4 methods), real-time strength + entropy | ✅ Done |
| M3 | Encrypted backup/restore, CSV export/import | ✅ Done |
| M4 | Health dashboard, navigation rail, full theming | ✅ Done |
| M5 | Read-only Viewer app | ✅ Done |
| M6 (web mode) | Localhost-bound web portal | ✅ Done |
| M6 (responsive layout) | Mobile-width layout for Main App + Viewer | 🚧 Planned, not built |
| Later | Android companion + sync design, macOS/Linux builds | ⏳ Not started |

## Stack

Tauri 2 (Rust core) + React 19 / TypeScript / Tailwind 4. SQLCipher-encrypted SQLite, AES-256-GCM,
Argon2id, Axum (web portal), zxcvbn (optional strength scoring).

## Security

This is a real password manager holding real credentials — see
[`docs/THREAT_MODEL.md`](docs/THREAT_MODEL.md) for exactly what is and isn't defended against. If
you find a security issue, please open an issue describing it rather than a public exploit writeup.
