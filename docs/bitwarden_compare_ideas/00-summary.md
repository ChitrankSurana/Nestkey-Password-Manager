# Bitwarden Comparison — Summary

Research date: 2026-07-26. Full detail in `01-crypto-core.md`, `02-vault-and-cli.md`,
`03-server-security-notes.md`. This project is local-first, single-user, zero-knowledge with
**no server, no accounts, no sync** — most of Bitwarden's org/sharing/multi-device machinery is
structurally not applicable. Where it isn't, each doc says so explicitly rather than forcing a fit.

## Top actionable ideas, ranked across all three reports

1. **Split the data-encryption key from the password-derived key.** *(01-crypto-core.md #1)*
   Bitwarden never encrypts vault data with a key derived straight from the password — they
   generate a random data key once, then wrap it under the password-derived key. We derive
   `vault_key` straight from Argon2id and use it directly as the SQLCipher key. Confirmed via grep:
   **no `change_password` function exists anywhere in `core/src`.** Changing the master password
   today would mean re-encrypting the whole vault; with a wrapped random key it's "re-wrap one
   32-byte key." Biggest single architectural gap found. Touches `auth.rs`, `vault.rs`, registry
   schema (new wrapped-key column); needs a one-time migration for existing vaults.

2. **Decide and document unlock-attempt throttling, explicitly.** *(03-server-security-notes.md #2)*
   Neither we nor Bitwarden document a concrete brute-force/lockout algorithm — genuine open gap on
   both sides, not just us. Given local-only (no network attacker to lock out), the honest choices
   are "Argon2id cost is the rate limit, nothing else needed" or "add a small backoff after N wrong
   unlocks to blunt a local automated-guessing script." Either is fine — what's missing is writing
   the decision down in `THREAT_MODEL.md` instead of leaving it implicit.

3. **Scripting-friendly flags on the Web Portal HTTP API.** *(02-vault-and-cli.md #1)*
   We already have an HTTP surface (`pwmgr-web`). A `?raw=true` / `Accept: text/plain` convention on
   read endpoints mirrors `bw --raw` for shell-scriptable access to one credential, at near-zero
   cost, without building a whole CLI binary.

4. **Generator: ambiguous-character avoidance + per-class minimum counts.** *(02-vault-and-cli.md #3)*
   Small, contained addition to `generate_random` — closes a real gap (some sites require ≥1 digit;
   some users hate `0`/`O`/`1`/`l` confusion).

5. **JSON export alongside CSV.** *(02-vault-and-cli.md #4)*
   Our CSV schema can't represent multiple credentials per site without flattening. A JSON export
   mirroring our own `SiteDto`/`CredentialDto` (nested credentials array) fixes that — for our own
   restore path, not for Bitwarden-format compatibility.

6. **Surface the "device compromise is out of scope" statement to end users.** *(03 #1)*
   Already in `THREAT_MODEL.md` as O1, but only contributors read that file. A one-liner in
   `USAGE_GUIDE.md` or the app's About/Settings screen costs nothing and sets expectations.

7. **Ciphertext version/type tag on `seal()`/`open()`.** *(01 #3)*
   Flag only, don't build — no scheme change has ever been needed. Cheap insurance if one ever is.

## Explicitly flagged, not recommended

- **Standalone CLI binary** *(02 #2)* — real gap, but new binary + new session-key IPC path + new
  attack surface for a secrets tool. A YAGNI decision for the user to make deliberately, not a
  default yes.
- **Card / Identity / SSH-key item types, folders/collections, org-sharing** — these exist in
  Bitwarden to power autofill and multi-user cloud sharing, both explicit PRD non-goals (§4/§5).
  Adding them here would be speculative scope with no user story.
- **PBKDF2 fallback / dual-KDF support, RSA org-key wrapping, per-item Cipher Key envelope** — all
  solve problems (legacy migration, multi-tenant sharing, per-item isolation) this project doesn't
  have.

## Where we already do as well or better

- Compile-time-enforced read-only Viewer binary — stronger than any runtime RBAC check.
- Explicit plaintext-CSV export confirmation gate (PRD §8) — Bitwarden's plain `.csv` export has none.
- AEAD-only ciphertext (AES-256-GCM), no legacy unauthenticated format ever existed — Bitwarden
  still carries a deprecated unauthenticated CBC variant for backward compat.
- Username-enumeration timing defense (decoy-salt Argon2id, test-asserted ≥5ms) — more explicit
  and test-covered than anything visible in Bitwarden's crypto crate.
- Concretely specified CSRF mitigation (double-submit, `SameSite=Strict` + custom header) vs. their
  whitepaper's generic "we use security headers" description.
- Structurally zero-knowledge (no server exists at all for Main/Viewer; Web Portal hard-bound to
  `127.0.0.1`) — sidesteps the entire malicious-server threat class Bitwarden needed a dedicated
  ETH Zurich audit to reason about.

## Suggested next step

None of this is built yet — this folder is research only, no code changed. If you want to move on
#1 (the split-key architecture) or #2 (throttling decision), say so and it goes through a proper
design/plan cycle before any implementation, per brainstorming → writing-plans.
