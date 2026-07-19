# How to Use — Web Portal

The Web Portal lets you use the full password manager from a **browser tab on this machine**,
without opening the desktop app window — the same vault, the same full feature set, served over
`http://127.0.0.1:7373`. It never listens on your network; it is reachable only from this machine,
never from another device, even one on the same Wi-Fi.

## When to use this instead of the Main App

- You'd rather work in a browser tab than a separate desktop window.
- You want to use the password manager alongside other browser-based workflows (e.g. copy-pasting
  into a web form you have open in another tab).
- You want every family member/user on a shared machine to log in through their own browser profile
  without each needing the desktop app focused.

Functionally it's the same app as the Main App — same screens, same features, same data. The only
practical difference is *how* you reach it.

## Starting the portal

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;C:\Strawberry\perl\bin;C:\Strawberry\c\bin;" + $env:PATH
npm run build          # only needed once, or after an update — builds the web assets
cargo run -p pwmgr-web
```
Run this from the **repo root** (it serves files from `./dist`, relative to where you launch it).
You'll see:
```
pwmgr-web listening on http://127.0.0.1:7373 (serving ./dist)
```
Leave that terminal window running for as long as you want the portal available, then open
**http://127.0.0.1:7373** in your browser. To stop it, press Ctrl+C in that terminal.

## Logging in / registering

Same Login and Register screens as the Main App, same shared vault files — a user you registered in
the Main App logs in here with the identical username/password, and vice versa.

## Using it

Every feature documented in [`HOW_TO_USE_MAIN_APP.md`](HOW_TO_USE_MAIN_APP.md) works identically
here — vault CRUD, search, the four generator methods, Health Dashboard (including the opt-in HIBP
breach check), Settings, Trash, Backup/Restore, and CSV export/import. One small difference:

- **Copying to clipboard** uses your browser's own clipboard permission instead of a native OS call
  — your browser may prompt you to allow clipboard access the first time.

## A few things worth understanding about how this mode works

- **It's genuinely localhost-only.** The server binds to `127.0.0.1` exclusively — it is not
  configurable to bind anywhere else, and it will refuse connections from any other device on your
  network. Nothing about "web mode" means "exposed to your Wi-Fi."
- **Each browser session is independent.** If you log in from two different browsers (or two
  separate profiles/incognito windows), each is its own fully separate session with its own vault
  connection — locking one doesn't affect the other, and one user's session never sees another's
  data.
- **Sessions live in memory only.** If the `pwmgr-web` process is stopped or crashes, everyone gets
  logged out — nothing about who was logged in survives a restart. Your actual vault data is
  untouched (it's durable on disk); you just need to log back in.
- **Idle auto-lock still applies**, using the same Settings value as the Main App — but because
  there's no desktop "vault-locked" event in a browser, an idle-locked session shows up as a failed
  request on your next click rather than an automatic redirect. Reloading the page will take you back
  to Login.

## Where your data lives

Identical to the Main App and Viewer — this mode doesn't create a separate copy of anything:

```
%USERPROFILE%\.pwmgr\
  registry.db          — usernames + KDF salts only, unencrypted, no secrets
  vaults\<user>.vault   — your encrypted data
  backups\*.bak         — encrypted backups
  config.json           — shared theme/settings
```

## Related guides

- Full read/write daily use (desktop window) → [`HOW_TO_USE_MAIN_APP.md`](HOW_TO_USE_MAIN_APP.md)
- Read-only emergency access → [`HOW_TO_USE_VIEWER_APP.md`](HOW_TO_USE_VIEWER_APP.md)
- What's protected and what isn't, including the web-mode-specific CSRF/cookie mitigations →
  [`THREAT_MODEL.md`](THREAT_MODEL.md)
- Step-by-step verification of this mode → [`testing/HOW_TO_TEST_WEB_PORTAL.md`](testing/HOW_TO_TEST_WEB_PORTAL.md)
