# Release notes — 2.9.0

**Date:** 2026-10-04. **Status:** release candidate for 3.0.0.

2.9.0 is the first tagged version. Earlier manifests all said `0.1.0` and were never bumped. This
release tags everything shipped so far. **3.0.0** will follow once the open live tests below are
closed.

## What is in this release

- **Secure core (M1):** register/login, SQLCipher-encrypted vault (AES-256-GCM, Argon2id), CRUD,
  trash, search, auto-lock, tooltips, dark theme.
- **Password generator (M2):** four methods, real-time strength and entropy.
- **Backup and CSV (M3):** encrypted backup/restore, CSV export/import.
- **Health, navigation, theming (M4):** health dashboard, navigation rail, full theming.
- **Read-only Viewer app (M5).**
- **Web portal and responsive layout (M6):** localhost-only Axum portal.
- **Windows Hello biometric unlock and SSH agent (Phase 7).**
- **New vault UI (Phase 8):** item types (login, secure note, card, identity), favorites, folders,
  master-password reprompt.
- **Web-mode security hardening (Phase 9):** path closure, throttling, CSP.
- **Make UI (third UI variant):** the modern interface, plus full-fidelity JSON export/import,
  per-credential change log, password history, master-password change, secret-content search, and
  the Viewer restyle.
- **Fixes:** single-instance guard, WebView2 black-screen mitigation, copy-button unification,
  devtools no longer opens automatically.

## Known limitations (accepted for this release)

These are open on purpose. They do not block 2.9.0 and are tracked for 3.0.0.

**Open live tests (blockers for 3.0.0, not for 2.9.0)**
- Wave 0 live retests R1–R5, including the **vault data-loss check**, the **blank-window launch**
  check, and N15. They need a running app and have not been re-run.
- Deferred web-mode UAT pass (B2, J7, N11, N14) and a full web re-run.
- Phase 9 human UAT: production-CSP browser sweep (`09-VERIFICATION.md`).

Until the data-loss retest is closed, **keep a backup** of `%USERPROFILE%\.pwmgr` and use the
encrypted backup feature regularly.

**Behavior**
- Health Dashboard "Weak" always uses zxcvbn, regardless of the Settings strength algorithm.
- A web portal crash or restart logs out every browser session.
- Web portal is tested at household scale only.
- CSV export/import does not carry secure notes, cards or identities. Use JSON export/import. The
  Viewer does not show these types yet.
- No automated UI or visual tests. UI is verified by the manual checklists in `docs/testing/`.
- Windows only. macOS and Linux builds are not available.

**Not in scope:** cloud sync, browser extension/autofill, and master-password recovery (impossible
by design). See [`KNOWN_LIMITATIONS.md`](KNOWN_LIMITATIONS.md) for the full list.

## Version bump

`version` changed from `0.1.0` to `2.9.0` in: `package.json`, `package-lock.json`,
`src-tauri/tauri.conf.json`, `viewer-src-tauri/tauri.conf.json`, and the `Cargo.toml` files of
`core`, `src-tauri`, `viewer-src-tauri`, `web-src-tauri` (plus `Cargo.lock`).

## Upgrading

No vault format, data folder or crypto change. Existing vaults open as before.
