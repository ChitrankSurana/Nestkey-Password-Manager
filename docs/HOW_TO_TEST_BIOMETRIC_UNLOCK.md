# How to Test — Biometric Unlock (Windows Hello)

Manual QA checklist for biometric unlock, added on `feature/bitwarden-port-phase1`. This is native
Windows Hello (`UserConsentVerifier`) + DPAPI (`CryptProtectData`/`CryptUnprotectData`) integration
that could only be compiled and unit-tested with a fake provider (`core/src/biometric.rs`) — it was
never exercised against a real Windows Hello prompt or a real DPAPI round-trip. `cargo test` /
`npm test` / `npm run build` all pass, but none of that proves the actual OS integration works.

Defaults **off** (`biometric_enabled: false` in config). Nothing here runs unless you explicitly
turn it on in Settings.

## Prerequisites

- Windows 10/11 with Windows Hello configured (face, fingerprint, or PIN-backed Hello) for the
  Windows account you're testing under.
- The desktop app (`src-tauri`), not the web or viewer build — biometric unlock has no web-mode
  route (Windows Hello can't run server-side for a remote browser).

```powershell
npm run tauri dev
```

## Checklist

- [ ] **Feature is off by default.** Fresh vault, open Settings → Security. "Enable biometric
      unlock (Windows Hello)" checkbox is unchecked. No "Set up biometric unlock" button visible.
- [ ] **Enabling reveals setup.** Check the box. A "Set up biometric unlock" button appears.
      `config.json` in `~/.pwmgr` should now show `"biometricEnabled": true`.
- [ ] **Setup prompts Windows Hello.** Click "Set up biometric unlock". The real Windows Hello UI
      (face/fingerprint/PIN prompt) should appear. Complete it successfully.
  - [ ] On success: button changes to "Biometric unlock is set up." + a "Remove" button.
  - [ ] Cancel the prompt instead (click away / Esc): should show an error, not crash, and not
        leave a half-configured state — "Set up biometric unlock" button should still be there
        to retry.
- [ ] **Lock and unlock with biometric.** Lock the vault (or restart the app). On the login screen,
      type the username you set biometric up for, then tab/click away from the username field.
      An "Unlock with Windows Hello" button should appear below the master-password field.
  - [ ] Click it → Windows Hello prompts again → on success, vault unlocks without ever typing the
        master password.
  - [ ] Cancel the prompt → clear error message, vault stays locked, master-password field still
        usable as a fallback.
- [ ] **Removing it works.** In Settings, click "Remove" next to biometric unlock. Confirm the
      "Unlock with Windows Hello" button no longer appears on the login screen for that user.
- [ ] **Turning the feature off hides everything.** Uncheck "Enable biometric unlock" in Settings.
      The setup/remove UI disappears from Settings, and "Unlock with Windows Hello" no longer
      appears on the login screen even if a biometric record still exists underneath (the toggle
      is a server-side gate, not just a UI hide — this is worth actually confirming, not assuming).
- [ ] **Restart persistence.** After setting biometric up, fully quit and relaunch the app. It
      should still offer "Unlock with Windows Hello" on the login screen without needing to be
      re-configured.
- [ ] **Sanity check the DPAPI angle.** DPAPI ties the wrapped key to your Windows user profile.
      If you have a second Windows account handy, confirm biometric unlock set up under account A
      does *not* work (and doesn't crash) if somehow invoked while logged in as account B — not
      expected to be reachable via the UI, but worth knowing if it ever surfaces as a bug report.

If any step fails: the "Enable biometric unlock" checkbox is the escape hatch — turn it off and
the vault behaves exactly as before this feature existed (master password / PIN unlock only).
File what broke rather than silently working around it.
