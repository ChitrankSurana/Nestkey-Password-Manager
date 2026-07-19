# How to Use — Main App

The Main App is the primary way to use this password manager day-to-day: a desktop window with full
read/write access to your vault — add, edit, search, generate, back up, import/export, and monitor
password health.

## Installing and running

**From source (no installer yet):**
```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;C:\Strawberry\perl\bin;C:\Strawberry\c\bin;" + $env:PATH
npm install
npm run tauri dev
```
This opens the app window. Everything you store lives under `%USERPROFILE%\.pwmgr\` on this machine —
nothing leaves it unless you explicitly export or back it up yourself.

## Registering your first user

On first launch you land on the **Register** screen.

1. Choose a **username** — this just identifies your vault on this machine; it isn't a secret and
   isn't sent anywhere.
2. Choose a **master password** — this is the only key to everything you'll ever store. **There is
   no recovery if you forget it.** This is by design: the app never stores your master password or
   anything derived from it in a reversible form, so there's nothing anyone (including you, without
   the password) could use to get back in. Pick something you'll remember, or store it somewhere
   safe outside this app (a physical note, a separate secret you trust).
3. Click **Register**. You land directly on your empty Vault.

If you want more than one person's vault on this machine, each person registers their own username —
vaults are fully independent, one encrypted file per user.

## Logging in

Enter your username and master password, click **Log in**. If either is wrong, you'll see one
generic **"Incorrect master password"** message — the app never reveals whether it was the username
or the password that didn't match, so an attacker watching failed attempts can't narrow down which
part is wrong.

## The Vault screen

This is home base: a list of every **site** you've added, a search box, and a nav rail (left side)
to reach every other feature — Health, Generator, Trash, Data, Settings, and Lock.

- **Add site** (top right): name (required), URL and notes (both optional).
- Click a site to open it, see its credentials, and **Add credential** (username optional, password
  required, remarks optional).
- **Search**: type anything — a site name, a username, or a word from your remarks — results filter
  live as you type, across hundreds of entries with no perceptible delay.
- The nav rail can be **collapsed** to icon-only (click the `«`/`»` toggle) — your preference is
  remembered.

## Generating passwords

Open **Generator** from the nav rail, or use the built-in generator panel whenever you're adding/
editing a credential. Four methods:

| Method | What it does | Options |
|---|---|---|
| **Random** | Fully random characters | Length; toggle uppercase / lowercase / digits / symbols independently |
| **Memorable** | Real dictionary words joined by a separator | Word count, separator character |
| **Pattern** | Fills a template you write | `C`/`c` = letter (upper/lower), `V`/`v` = vowel (upper/lower), `#` = digit, `?` = symbol, `\` = literal character |
| **Pronounceable** | Syllable-based, easier to read aloud/type manually | Length |

Every method shows a live strength meter (colored bar + label) as you generate or type. Click
**Copy** to put it on the clipboard — it **auto-clears after 30 seconds** so a stray paste later
doesn't leak it.

The strength meter itself has two modes, switchable in Settings: **Simple** (a fast entropy
estimate) or **Advanced** (zxcvbn, which also recognizes common patterns/dictionary words, not just
raw character-class math). This setting only affects the live meter you see while typing/generating
— it does **not** change how the Health Dashboard classifies "Weak" (see below).

## Health Dashboard

Open **Health** from the nav rail for a whole-vault check:

- **⚠️ Weak** — passwords the advanced (zxcvbn) scorer considers weak, regardless of your Settings
  strength-algorithm choice. Click to expand the list; click an entry to jump straight to that site.
- **♻️ Reused** — the same password used across more than one credential, grouped together.
- **🕰️ Old** — passwords older than your configured threshold (Settings → "Consider a password 'old'
  after", 90/180/365 days).
- **🛡️ Breached** — opt-in only. Turn on "Enable breach check (HIBP)" in Settings first, then click
  **Check now**. This is the *one* feature in this app that talks to the internet: it checks your
  passwords against [Have I Been Pwned](https://haveibeenpwned.com/)'s breach database using
  k-anonymity — only the first 5 characters of a SHA-1 hash of each password ever leave your machine,
  never the password itself, and HIBP cannot reconstruct your password from that. Leave it off if you
  want the app to make zero network calls, ever.

## Settings

| Setting | What it controls |
|---|---|
| Auto-lock after idle | Minutes of inactivity before the vault automatically locks (1/5/15) — takes effect immediately on the running session, not just next login |
| Password strength algorithm | Simple vs. Advanced (zxcvbn) — affects the live meter only, not the Health Dashboard's Weak classification |
| Theme | Dark or Light |
| Accent color | Indigo, Sky, Teal, Purple, or Pink — used for buttons/highlights throughout |
| Consider a password "old" after | 90/180/365 days — feeds the Health Dashboard's Old card |
| Breach check (HIBP) | Off by default; turn on to enable the Health Dashboard's Check-now button |

Every change here saves immediately (the control briefly disables while saving) — there's no
separate "Save" step.

## Locking

Click **Lock** in the nav rail any time to return to the Login screen immediately, no confirmation
needed — your key is wiped from memory the instant you do. The app also **auto-locks** after your
configured idle period, whether or not you manually lock first.

## Trash

Deleting a credential moves it to **Trash**, not gone-forever. From Trash you can **Restore** any
item back to its site. **Empty Trash** permanently and irreversibly deletes everything currently in
it — this is the only destructive action in the app that isn't otherwise reversible, and it asks you
to confirm first.

## Backup and restore

**Data → Backup & Restore:**
- **Create Backup** writes a fully encrypted snapshot of your vault to
  `~/.pwmgr/backups/`. Copy that file anywhere you like (another drive, a USB stick) for safekeeping
  — it's just as encrypted as your live vault.
- **Restore Backup** picks a `.bak` file (from this machine or copied in from another) and restores
  it. After restoring, log in with that backup's own username/master password.

## Exporting to CSV

**Data → Export CSV**, three modes:

1. **Encrypted (this device)** — recommended default. Only re-importable/openable in this app on
   this device.
2. **Encrypted (with a password)** — you set a separate *export* password (not your master
   password). Portable to another device/install of this app, opened with that export password. Not
   a standard ZIP or anything else openable outside this app.
3. **Plaintext (unencrypted)** — every password in your vault, fully readable by anyone who opens the
   file. Gated behind typing the exact phrase `EXPORT PLAINTEXT` to confirm — the button stays
   disabled until you do. Use this only when you genuinely need a plain CSV (e.g. importing into
   another tool), and treat the resulting file as sensitive from the moment it's created.

## Importing from CSV

**Data → Import CSV** — three source options:

- **Chrome/Chromium export** or **Firefox export** — pick the CSV those browsers exported, no
  further setup needed.
- **Other (map columns myself)** — pick any CSV, then map each of its columns to a target field
  (site name, URL, notes, username, secret, remarks) or leave it ignored.

Either way: **Preview** parses the file without saving anything (you can edit any cell in the
preview table before committing), then **Commit Import** actually inserts the records. Duplicates
are detected and skipped automatically, and the result tells you how many were imported vs. skipped.

## Where your data lives

```
%USERPROFILE%\.pwmgr\
  registry.db          — usernames + KDF salts only, no secrets, unencrypted (by design — see docs/ARCHITECTURE.md)
  vaults\<user>.vault   — your actual encrypted data, SQLCipher + per-field AES-256-GCM
  backups\*.bak         — encrypted backups you've created
  config.json           — theme, auto-lock minutes, and other preferences
```

## Related guides

- Read-only emergency access → [`HOW_TO_USE_VIEWER_APP.md`](HOW_TO_USE_VIEWER_APP.md)
- Browser access on this machine → [`HOW_TO_USE_WEB_PORTAL.md`](HOW_TO_USE_WEB_PORTAL.md)
- What's protected and what isn't → [`THREAT_MODEL.md`](THREAT_MODEL.md)
