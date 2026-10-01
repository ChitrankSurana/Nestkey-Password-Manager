# Vault Data Model & CLI/UX — Bitwarden vs. This Project

Sources: `bitwarden.com/help/cli`, `/help/managing-items`, `/help/generator`, `/help/export-your-data` (fetched 2026-07-26). Bitwarden's `clients` monorepo source itself wasn't cloned — these are their published docs, which describe the same behavior their `clients`/CLI code implements.

## 1. Bitwarden's approach

**Vault item types** (`/help/managing-items`, `bw` CLI enum): five item types — `Login` (1), `Secure Note` (2), `Card` (3), `Identity` (4), `SSH key` (5). Login items can also hold passkeys and (Premium) TOTP seeds. Cards/Identities/SSH keys exist purely to power autofill on other kinds of web forms (checkout, address forms) and SSH auth.

**Organization**: folders (personal, one level) vs. collections (org-shared, permission-scoped) are distinct concepts — folders are per-user, collections are the sharing/ACL unit for org vaults. Favorites and custom fields (`Text` / `Hidden` / `Boolean` typed, per the CLI enum) layer on top of any item type.

**CLI (`bw`)**:
- `bw login`/`bw unlock` return a session key; you either `export BW_SESSION=...` or pass `--session <key>` per-command. `bw status` reports `unlocked`/`locked`/`unauthenticated` as JSON.
- `bw serve --port <p> --hostname <h>` starts a local Express server exposing the full CLI surface as a REST API (defaults to `localhost`-bound; blocks non-matching `Origin` headers unless you explicitly disable that check; `--hostname all` opts out of the bind restriction with an explicit warning).
- Global scripting flags: `--raw` (bare value, no descriptive wrapper), `--response` (force JSON envelope even for normally-prose output), `--pretty`, `--quiet` (suppress stdout, useful when piping a secret to a file/app), `--nointeraction`.
- `bw generate` — password mode (`-u -l -n -s --length`, min length 5, default 14-char u+l+n) and passphrase mode (`--words --separator --capitalize --includeNumber`, default 3 words, `-` separator).
- `bw export` — `.csv`, `.json`, `encrypted_json` (password- or account-key-encrypted), or `.zip` (json + attachments). Only `.json`/`encrypted_json` carry cards/identities/passkeys/SSH keys — `.csv` only covers logins+notes. Trash and Sends are never exported.
- `bw import <format> <path>` — dozens of source formats (`bw import --formats` lists them), including their own encrypted export.

**Autofill** (browser extension/mobile, high-level only): item type drives which web-form fields get matched — not architecturally relevant here since this project has no browser extension and it's an explicit PRD non-goal (`docs/design/PRD.md` §4).

## 2. Our current approach

**Data model** (`core/src/vault.rs`): two tables — `Site { id, name, url, notes }` and `Credential { id, site_id, username, secret, remarks }`, one-to-many. This *is* effectively Bitwarden's "Login" item type only — no Card/Identity/Secure-Note/SSH-key equivalents, no folders, no collections (no org/sharing model — by design, single-user local vault), no favorites, no custom fields. Multiple credentials per site (e.g. two logins for one service) is something Bitwarden's flat item list doesn't do as cleanly — worth noting as a place we're already ahead structurally.

**Generator** (`core/src/generator.rs`): four methods — random (class toggles, length 4–128, no per-class minimum-count option, no ambiguous-character-avoidance toggle), memorable (EFF short wordlist, 2–12 words, separator up to 8 chars, no capitalize/include-number options), pattern (template DSL), pronounceable. No standalone username generator.

**Export/import** (`core/src/portio.rs`, `core/src/backup.rs`): CSV export/import (generic 6-column schema: `site_name, url, notes, username, secret, remarks`; Chromium-format import parser). Encrypted backup is a custom length-prefixed binary bundle (`encode_bundle`/`decode_bundle`) wrapping the registry record + raw SQLCipher vault bytes — not a JSON schema, and not vault-item-granular (it's a whole-vault-file bundle, more like a full disk image than Bitwarden's per-item JSON export).

**CLI**: none exists. No `clap`/`StructOpt`, no `bin` entry in `package.json`. All three surfaces (Main App, Viewer, Web Portal) are GUI-only (Tauri window or browser), reached via `npm run tauri dev` / `cargo run -p pwmgr-web` — no scriptable session-key/headless-automation path today.

## 3. Gaps / ideas worth adopting (ranked)

1. **Global scripting flags on the Web Portal's HTTP API would cost little and add real value.** We already have an HTTP surface (`pwmgr-web`, §`ARCHITECTURE.md`) — a `?raw=true` or `Accept: text/plain` convention for scriptable read endpoints (e.g. "get one password" for a shell script) mirrors `bw --raw` cheaply, without needing a whole CLI binary. *Low effort, direct fit.*
2. **A minimal `bw`-style CLI is the biggest structural gap**, useful for scripting (`pwmgr get <site>`, piping into other tools) without opening a GUI — but it's genuinely new surface area (new binary/crate, new session-key IPC-equivalent, new attack surface for a secret-handling tool) — this is a "does it need to exist" YAGNI question for the user, not an obvious yes. Flag, don't build speculatively.
3. **Ambiguous-character avoidance + per-class minimum counts in the random generator** are small, contained additions to `generate_random` (a couple of bool/usize fields + one filter pass) that close a real UX gap (some sites reject passwords without a digit; some users hate `0`/`O`/`1`/`l` confusion) — worth adding on request, not much bigger than what's already there.
4. **JSON export (not just CSV) would remove a real limitation**: our CSV schema can't represent multiple credentials per site without flattening/duplicating rows. A `.json` export mirroring our own `SiteDto`/`CredentialDto` shape (nested credentials array) is a natural fit for backup portability between installs, distinct from the existing binary `backup.rs` bundle (which is disk-image-like, not human-inspectable/interoperable). *Note: this is a new format for our own restore path, not compatibility with Bitwarden's json — no reason to match their schema exactly.*
5. **Item-type variety (Card/Identity/SSH-key) is explicitly NOT recommended to copy** — it exists in Bitwarden to serve autofill, which is a non-goal here (PRD §4). Adding those types with nothing to autofill into is speculative scope with no user story behind it. Skip.
6. **Folders/collections**: folders (flat, personal) could be a real usability win once the vault grows past casual browsing (`docs/KNOWN_LIMITATIONS.md` may already track this — check before proposing). Collections/org-sharing is Bitwarden's multi-user cloud model — explicitly out of scope for a single-vault local-only design (PRD §4, §5).

## 4. Where we're already doing as well or better

- **Compile-time write-restriction (Viewer app has no add/edit/delete code paths at all)** is a stronger guarantee than any RBAC/permission check Bitwarden's collections model provides — theirs is enforced at runtime/server, ours is enforced by the binary literally not containing the code.
- **CSV plaintext-export confirmation gate** (`docs/design/PRD.md` §8: "cannot happen without an explicit, unambiguous confirmation step") is more deliberate friction than Bitwarden's plain `.csv` export, which has no such gate.
- **Per-field AES-256-GCM on top of SQLCipher** is defense-in-depth beyond what a single-layer full-disk-encrypted export needs, appropriate for our "trust nothing, not even a stolen disk image" threat model.
- **No server/account/org attack surface at all** — the entire class of Bitwarden CVEs and incidents tied to server-side sync, account takeover, or org-permission bugs simply doesn't exist in our design space.
