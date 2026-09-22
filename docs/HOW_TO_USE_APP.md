# How to Use — LocalVault (Current Build)

This is the full, current-state guide to the Main App: everything it can do as of this build,
across all three interfaces (Classic, New, Make). It supersedes
[`HOW_TO_USE_MAIN_APP.md`](HOW_TO_USE_MAIN_APP.md) as the complete reference — that file still
accurately describes the Classic interface specifically, but predates New, Make, and several
features (folders, favorites, item types, password history, PIN/biometric/SSH unlock, master
password change, reprompt) that this guide covers.

For the read-only emergency-access app or browser access, see
[`HOW_TO_USE_VIEWER_APP.md`](HOW_TO_USE_VIEWER_APP.md) and
[`HOW_TO_USE_WEB_PORTAL.md`](HOW_TO_USE_WEB_PORTAL.md) — everything below applies to both the
desktop app and the web portal identically (same backend, same features), except where noted.

## Installing and running

**From source (no installer yet):**
```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;C:\Strawberry\perl\bin;C:\Strawberry\c\bin;" + $env:PATH
npm install
npm run tauri:dev
```
This opens the app window. Everything you store lives under `%USERPROFILE%\.pwmgr\` on this machine
— nothing leaves it unless you explicitly export or back it up yourself, or opt in to breach
checking (see Health Dashboard below).

## Choosing your interface

Settings → **Interface** lets you pick which of three visual styles the app opens in. All three
talk to the same backend and the same encrypted vault — switching is purely cosmetic, with one
functional difference:

| Interface | What it looks like | Folders / favorites / item types / custom fields UI / password history |
|---|---|---|
| **Classic** | The original, simplest layout | Not exposed (data stays intact if set via another interface, just not editable here) |
| **New** | A refreshed two/three-pane layout | Favorites only |
| **Make** | The most current design, ported from the Figma prototype | Full support — this is the only interface with folders, item types, custom-field editing, and password history in its UI |

Changing this setting takes effect on next launch (desktop) or by opening a different `.html` entry
point in your browser (web mode) — the app tells you which after you change it.

## Registering and logging in

On first launch you land on **Register**.

1. Choose a **username** — identifies your vault on this machine; not a secret, never sent anywhere.
2. Choose a **master password** — the only key to everything you store. **There is no recovery if
   you forget it.** The app never stores it, or anything derived from it in reversible form.
3. Register. You land directly on your empty vault.

Multiple people can share a machine — each registers their own username; vaults are fully
independent, one encrypted file per user.

**Logging in:** username + master password. A wrong entry always shows the same generic
"Incorrect master password" — the app never reveals whether the username or password was the
problem.

**Quick unlock (PIN):** once set up (Settings → Security → PIN unlock), the login screen offers
"Unlock with PIN instead" — a short PIN in place of typing the full master password every time.

**Biometric unlock (Windows Hello, desktop only):** once enabled and set up (Settings → Security),
the login screen offers a fingerprint/face-unlock button as an alternative to typing anything.

Both PIN and biometric are *quick-unlock* shortcuts, not a weaker vault — under the hood they still
unwrap the same master key; losing your master password still means losing the vault regardless of
whether PIN/biometric are set up.

## The vault

A list of every **site** you've added, a search box, and navigation to every other feature (Health,
Generator, Trash, Data, Settings, Lock).

- **Add site**: name (required), URL and notes (optional).
- Click a site to see its credentials and **Add credential** (username optional, password required,
  remarks optional).
- **Search**: matches site names, usernames, remarks, and decrypted custom-field content — results
  filter live as you type.

### Item types, folders, and favorites (Make interface)

The Make interface adds four item types per site, not just logins:

| Type | What it holds |
|---|---|
| **Login** | Username + password + custom fields (the original, universal type) |
| **Secure Note** | A single free-text note, sealed like a password |
| **Payment Card** | Card number, expiration date, CVV — all sealed; a non-secret brand/last-4 hint shows in the list |
| **Identity** | Full name and other identity fields, sealed |

Note/Card/Identity items store their fields as sealed custom fields under reserved labels (Card
Number, Expiration Date, Security Code (CVV), Full Name, Note) rather than dedicated database
columns — functionally identical protection to a Login's password field.

- **Favorites**: star any item from its detail view; a Favorites section in the sidebar collects
  them.
- **Folders**: create/rename/delete from the sidebar. Deleting a folder *unfiles* its items — it
  never deletes them.
- Item type doesn't restrict CSV export/import: CSV only ever carries the universal
  username/password/remarks fields, so Secure Note/Card/Identity items and folder/favorite/type
  metadata are **not** included in CSV in or out — use Backup for a complete, lossless copy (see
  below).

## Editing a credential

- **Username / Password** — password field has the generator built in (see below).
- **Custom fields** — add any number of `label: value` pairs, each typed as **Text**, **Hidden**
  (masked like a password, revealed via the same reprompt gate), or **Boolean** (Yes/No).
- **Authenticator (TOTP) secret** — paste a Base32 secret from a site's "set up 2FA" screen; the app
  then shows a live 6-digit code with a countdown, right next to the credential.
- **Require master password to view** ("reprompt") — per-credential switch: when on, revealing or
  copying *this* credential's secret always asks for the master password again, even mid-session.
  Global behavior for reprompt is configurable in Settings (see below).
- **Password history** (Make interface only) — every password change is kept, most recent 5, each
  individually revealable through the same reprompt gate. Older entries roll off automatically.

## Generating passwords

Open **Generator**, or use the generator panel built into any credential editor. Four methods:

| Method | What it does | Options |
|---|---|---|
| **Random** | Fully random characters | Length; toggle uppercase / lowercase / digits / symbols independently |
| **Memorable** | Real dictionary words joined by a separator | Word count, separator character |
| **Pattern** | Fills a template you write | `C`/`c` = letter (upper/lower), `V`/`v` = vowel (upper/lower), `#` = digit, `?` = symbol, `\` = literal character |
| **Pronounceable** | Syllable-based, easier to read/type manually | Length |

A live strength meter (bar + label) updates as you generate or type. **Copy** puts it on the
clipboard, auto-clearing after 30 seconds.

The meter itself has two algorithms, switchable in Settings: **Simple** (fast entropy estimate) or
**Advanced** (zxcvbn — also recognizes common patterns/dictionary words). This only affects the live
meter — it does **not** change the Health Dashboard's "Weak" classification, which always uses the
advanced algorithm regardless of this setting.

## Health Dashboard

A whole-vault check, covering every login:

- **Weak** — passwords the advanced (zxcvbn) scorer flags, regardless of your strength-algorithm
  setting. Click to expand; click an entry to jump to that site.
- **Reused** — the same password used across more than one credential, grouped together.
- **Old** — passwords older than your configured threshold (Settings → "Consider a password 'old'
  after": 90/180/365 days).
- **Unsecured** — sites stored with an `http://` (not `https://`) URL.
- **Breached** — opt-in only (Settings → "Enable breach check (HIBP)"), then click **Check now**.
  The *only* feature that talks to the network: it checks against
  [Have I Been Pwned](https://haveibeenpwned.com/) using k-anonymity — only the first 5 characters
  of a SHA-1 hash of each password ever leave your machine, never the password itself. Leave it off
  for zero network calls, ever.

## Trash

Deleting a credential moves it to **Trash**, not gone-forever.

- **Restore** any item back to its site.
- **Empty Trash** — permanent, irreversible, asks you to confirm first. The only fully destructive
  action in the app.
- Deleting a *site* that still has credentials cascades: its live credentials move to Trash too
  (confirmed first), restoring one of them also revives the parent site if it was cascade-deleted.
- Per-item permanent delete isn't offered anywhere — only bulk Empty Trash.

(Make interface note: Trash is a filtered view of the same list/detail chrome, matching the current
design — functionally identical to Classic's dedicated Trash page.)

## Data: backup, export, import

### Backup and restore

- **Create Backup** writes a fully encrypted snapshot to `~/.pwmgr/backups/` (or triggers a browser
  download in web mode). Copy it anywhere for safekeeping — it's as encrypted as your live vault.
- **Restore Backup** picks a `.bak` file and replaces your current vault with it. Log in afterward
  with *that backup's* username/master password. Restoring locks the current session.

### Export to CSV

Requires your master password every time, regardless of mode (exporting hands out every secret).
Three modes:

1. **Encrypted (this device)** — recommended default; only re-openable in this app on this device.
2. **Encrypted (with a password)** — a separate *export* password (not your master password);
   portable to another device/install.
3. **Plaintext (unencrypted)** — every password in the clear. Gated behind typing the exact phrase
   `EXPORT PLAINTEXT` to confirm.

CSV only ever carries site name, URL, notes, username, password, and remarks — favorites, folders,
item type, custom fields, TOTP, and password history are never included, in either direction. For a
complete, lossless copy, use Backup instead.

### Import from CSV

Three source presets (Chrome/Chromium, Firefox, LastPass) or **Other**, where you map each column
yourself to a target field. **Preview** parses without saving anything (edit any cell first);
**Commit Import** inserts the records. Duplicates are detected and skipped automatically.

## Settings

| Setting | What it controls |
|---|---|
| Auto-lock after idle | Minutes of inactivity before auto-lock (1/5/15) — takes effect immediately |
| Consider a password "old" after | 90/180/365 days — feeds the Health Dashboard's Old card |
| Breach check (HIBP) | Off by default; enables the Health Dashboard's Check-now button |
| PIN unlock | Set up or remove a quick-unlock PIN (removing requires re-entering your master password) |
| Biometric unlock (Windows Hello, desktop only) | Master on/off switch, then a separate "set up" step that prompts Windows Hello |
| Change master password | Re-seals every stored secret under a new password; takes a backup automatically first; removes PIN and biometric unlock (set them up again after) |
| Enable SSH agent (desktop only) | Serves stored SSH private keys to SSH clients over a local named pipe; manage keys underneath |
| Theme | Dark or Light |
| Accent color | Indigo, Sky, Teal, Purple, or Pink |
| Mobile navigation style | How the sidebar is replaced below 640px width: icon rail, bottom tabs, or hamburger menu |
| Interface | Classic / New / Make — see "Choosing your interface" above |
| Password strength algorithm | Simple vs. Advanced (zxcvbn) — the live generator meter only, not Health Dashboard |
| Reprompt mode | **Per-credential**: only items with "Require master password to view" ask, every time. **Global**: every item's reveal/copy asks, with a grace window after one correct entry |
| Reprompt grace window | Global mode only — 15/30/60/300 seconds before the next reveal/copy asks again |

Every change saves immediately (the control briefly disables while saving) — no separate "Save"
step, except Change Master Password which has its own explicit submit.

Removing PIN, removing biometric unlock, and deleting an SSH key all require re-entering your
master password first — an unattended unlocked session can't quietly change how the vault can be
unlocked.

## SSH Agent (desktop only)

An experimental integration: stores OpenSSH private keys (unencrypted key material, no passphrase)
and serves them to SSH clients over a local Windows named pipe, same idea as `ssh-agent` /
`1Password`'s SSH agent. Off by default — turn on in Settings, then add keys with a label and the
raw private key text.

## Locking

**Lock** returns to the Login screen immediately, no confirmation — your key is wiped from memory
instantly. The app also **auto-locks** after your configured idle period regardless.

## Where your data lives

```
%USERPROFILE%\.pwmgr\
  registry.db            — usernames + KDF salts only, no secrets, unencrypted (by design — see docs/ARCHITECTURE.md)
  vaults\<user>.vault     — your actual encrypted data, SQLCipher + per-field AES-256-GCM
  backups\*.bak           — encrypted backups you've created
  config.json             — theme, auto-lock minutes, interface choice, and other preferences (global, not per-account)
```

## Related guides

- Classic interface specifically, in more granular detail → [`HOW_TO_USE_MAIN_APP.md`](HOW_TO_USE_MAIN_APP.md)
- Read-only emergency access → [`HOW_TO_USE_VIEWER_APP.md`](HOW_TO_USE_VIEWER_APP.md)
- Browser access on this machine → [`HOW_TO_USE_WEB_PORTAL.md`](HOW_TO_USE_WEB_PORTAL.md)
- What's protected and what isn't → [`THREAT_MODEL.md`](THREAT_MODEL.md)
- Known gaps and deferred items → [`KNOWN_LIMITATIONS.md`](KNOWN_LIMITATIONS.md)
