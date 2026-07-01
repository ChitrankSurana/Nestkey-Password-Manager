# Password Manager

A local-first, zero-knowledge password manager. All data is stored only on your machine, in an
encrypted vault file, unlocked by a master password that the software never stores.

## Status

Pre-implementation. Design and planning documents are complete; the M1 milestone (secure core +
desktop MVP) is being planned/built next. See `docs/` for the full design.

## Docs

- [`docs/PRD.md`](docs/PRD.md) — what we're building and why
- [`docs/TRD.md`](docs/TRD.md) — how it's built: stack, crypto design, data model, module breakdown
- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) — system architecture and key data flows
- [`docs/THREAT_MODEL.md`](docs/THREAT_MODEL.md) — assets, adversaries, mitigations, accepted risks
- [`docs/USAGE_GUIDE.md`](docs/USAGE_GUIDE.md) — end-user guide (filled in as features ship)

## Roadmap

MVP-first: secure core + desktop app (M1), then password generator + strength (M2), backup/restore
+ CSV import/export (M3), health dashboard + theming (M4), read-only viewer app (M5), localhost web
mode (M6). Android companion is design-only for now. Full detail in the TRD/architecture docs.

## Stack

Tauri (Rust core) + React/TypeScript frontend. SQLCipher-encrypted SQLite, AES-256-GCM, Argon2id.
