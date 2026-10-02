# Crypto Core: Bitwarden vs. This Project

Sources: `bitwarden/sdk-internal` (current home of the Rust crypto crates; the older
`bitwarden/sdk` repo has been superseded by this one for crypto), specifically
`crates/bitwarden-crypto/src/{keys/kdf.rs, keys/master_key.rs, enc_string/symmetric.rs}` and
`crates/bitwarden-unlock/src/session_key.rs`, fetched via `gh api` on 2026-07-26.

## 1. Bitwarden's approach

**KDF.** `Kdf` enum supports `PBKDF2 { iterations }` (legacy, min 5,000, default 600,000 — OWASP's
current PBKDF2-HMAC-SHA256 floor) and `Argon2id { iterations, memory, parallelism }` (min 2
iter / 16 MiB / 1 lane, default **6 iterations / 32 MiB / 4 lanes**). Both minimums are enforced in
code (`derive_kdf_key` hard-errors below the floor) — this isn't just a doc convention, a caller
literally cannot construct a sub-minimum derivation.
(`crates/bitwarden-crypto/src/keys/kdf.rs`)

**Key hierarchy (the key design difference).** Bitwarden does NOT use the password-derived key to
encrypt vault data directly. Instead:

1. `MasterKey = KDF(password, email)` — used only to *wrap* things, never to decrypt vault content.
2. A `master_password_hash = PBKDF2(master_key_bytes, password_bytes, purpose)` is derived
   separately (`HashPurpose::ServerAuthorization` / `LocalAuthorization`) — this is their
   login/verifier check, analogous to our `verifier_key`.
3. A **random, independent `UserKey`** (the actual data-encryption key) is generated once at
   registration and never derived from the password. It's wrapped: `stretch_key(master_key)` →
   encrypt `UserKey` with that stretched key → store the resulting `EncString` blob.
   (`crates/bitwarden-crypto/src/keys/master_key.rs`, `encrypt_user_key`/`decrypt_user_key`)
4. To change the master password, they only need to re-wrap the same `UserKey` under a new
   `MasterKey` — vault content is never touched.

**Envelope/ciphertext format.** `EncString` is a tagged, versioned format:
`[type].[iv]|[data]|[mac]` for legacy, or `[type].[cose_encrypt0_bytes]` for the current preferred
variant. Three live variants:
- `Aes256Cbc_B64` — deprecated, explicitly documented "MUST NOT be used for encrypting" (unauthenticated).
- `Aes256Cbc_HmacSha256_B64` — AES-256-CBC + encrypt-then-MAC (HMAC-SHA256). Still supported for
  decryption, being phased out.
- `Cose_Encrypt0_B64` — current preferred: standard COSE Encrypt0 structure, algorithm-agile
  (their code references both `XAES_256_GCM` and `XCHACHA20_POLY1305` as COSE algorithm IDs), i.e.
  they're moving toward a self-describing AEAD envelope rather than a fixed CBC+HMAC pair.
  (`crates/bitwarden-crypto/src/enc_string/symmetric.rs`)

**Zeroization.** `KdfDerivedKeyMaterial` and friends use pinned heap allocations
(`Pin<Box<Array<u8, U32>>>`) plus explicit `zeroize()` calls, and there's a deliberate
`clear_stack()` trick after Argon2 hashing specifically because Argon2's internal C-adjacent
implementation leaves working memory on the stack that Rust's normal zeroize-on-drop can't reach —
they force a 4KB stack write to overwrite it. `EncString` itself derives `ZeroizeOnDrop`.

**Session/biometric unlock.** `bitwarden-unlock`'s `SessionKey` is a random `XAes256Gcm` key that
*seals the UserKey* (same envelope pattern as above, reusing `SymmetricKeyEnvelope`). The SDK's
job is only to mint/seal/unseal this key — the caller (client app) is responsible for storing it
in "a secure location outside the SDK (e.g. the OS keychain)". This is the same wrap-an-existing-key
pattern, just with the OS keychain as the second unlock factor instead of a password.

## 2. Our current approach

(`core/src/crypto.rs`, `core/src/auth.rs`, `core/src/vault.rs`)

- KDF: Argon2id only, fixed params (64 MiB / 3 iterations / 4 lanes / 32-byte output), no PBKDF2
  fallback, no enforced minimum (the params are hardcoded, so there's nothing to violate).
- Two independently-salted Argon2id derivations from the same password: `kdf_salt` → `vault_key`
  (`crypto.rs:40-48`) and `verifier_salt` → `verifier_key` (`auth.rs:38-41`) — conceptually
  equivalent to Bitwarden's master-key/master-password-hash split, but **`vault_key` is used
  directly** as both the SQLCipher DB key (`vault.rs:22-46`, hex-PRAGMA'd in) and the per-field
  AES-256-GCM key. There is no independent random data-encryption key — the password-derived key
  IS the data key.
- Ciphertext format: single fixed scheme, AES-256-GCM (already AEAD, no separate HMAC needed),
  random 12-byte nonce per `seal()` call, no version tag / algorithm-agility field.
- Verifier check: constant-time compare via `subtle::ConstantTimeEq` (`verifier_matches`) — a
  timing-safe equivalent of their `HashPurpose::LocalAuthorization` check, plus a decoy-salt Argon2id
  call on unknown usernames specifically to close a username-enumeration timing oracle
  (`auth.rs:22-29, 76-82`) — this is *more* defense-in-depth than what I found in Bitwarden's crypto
  crate itself (their username-enumeration protection, if any, would live in `server`, out of scope
  here).
- Zeroization: `VaultKey` is `ZeroizeOnDrop` over a private `[u8; 32]`, accessible only via a
  borrowing `as_bytes()` — explicitly designed (per the in-file comment) to prevent a caller from
  copying out the raw bytes and defeating zeroization. The SQLCipher PRAGMA path is documented as
  a known, unfixable ceiling: rusqlite formats the hex key into its own internal SQL buffer that
  this crate can't reach or zeroize (`vault.rs:24-31`).

## 3. Gaps / ideas worth adopting (ranked)

1. **Split the data-encryption key from the password-derived key (highest value).** Right now,
   changing the master password would require decrypting and re-encrypting the entire vault
   (every credential row + the SQLCipher DB itself) — I confirmed via grep that no
   `change_password`/rewrap function exists anywhere in `core/src`. Adopting Bitwarden's pattern —
   generate one random `VaultDataKey` at registration, wrap it with the Argon2id-derived key,
   store only the wrapped blob in the registry — means a password change becomes "re-wrap one
   32-byte key" instead of "re-encrypt the whole vault." This is the single biggest architectural
   difference and the most reusable idea here. Cost: touches `auth.rs` (registration/login), the
   registry schema (one new wrapped-key column), and `vault.rs` (open with unwrapped data key
   instead of the raw Argon2id output). Existing vaults would need a one-time migration.
2. **Reuse the same wrap/unwrap primitive for a future OS-keychain "remember me" unlock.** If a
   quick-unlock feature is ever wanted (skip re-typing the master password after a short-lived
   app restart), the `SessionKey` pattern — a random key that wraps the vault data key, itself
   stored in the OS keychain — is the same primitive as #1 and costs almost nothing extra once #1
   exists. Not urgent; only worth doing *because* #1 makes it nearly free.
3. **Ciphertext version/type tag.** Our `seal()`/`open()` have no format tag — if the encryption
   scheme ever needs to change, there's no way to distinguish old vs. new ciphertext. A one-byte
   prefix (or reusing SQLCipher's own versioning) would cost little now and avoid a hard migration
   wall later. Low priority since we've never needed to change the scheme yet — flag, don't build.
4. **Enforced KDF minimums** — not applicable to us the way it is to them: they support multiple
   KDF choices (PBKDF2 or Argon2id, user-configurable), so a floor guards against a
   misconfiguration client-side. We hardcode one fixed Argon2id param set with no user-facing KDF
   choice, so there's no configuration surface to misuse. Skip.
5. **The Argon2 `clear_stack()` trick** — worth knowing about, marginal value for us: our
   `derive_key` already zeroizes the returned key via `ZeroizeOnDrop`, and the residual stack
   scratch from the `argon2` crate's hashing call is the same class of risk in both codebases.
   Could add the same `black_box([0u8; 4096])` trick after our `hash_password_into` call for
   parity, but it's a genuine corner-case (attacker needs a stack-memory read primitive, which is
   already out of scope per our O1/O3 threat model exclusions). Low priority.

## 4. Things we already do better or equivalently

- **Username-enumeration timing defense** (`DECOY_SALT` + always-pay-Argon2id-cost on unknown
  username, with a dedicated test asserting ≥5ms) is more explicit and test-verified in our code
  than anything visible in Bitwarden's crypto crate itself (their equivalent protection, if any,
  is a server concern, not a crypto-crate concern — so this isn't quite apples-to-apples, but our
  local-auth path has no analogous gap to fix).
- **AEAD-only, single scheme.** We ship one authenticated scheme (AES-256-GCM) with no legacy
  unauthenticated variant ever having existed. Bitwarden still carries a deprecated
  `Aes256Cbc_B64` (unauthenticated) variant in their decrypt path for backwards compatibility, and
  their still-current `Aes256Cbc_HmacSha256_B64` variant needs manual encrypt-then-MAC composition
  where GCM gives us that for free. Having no legacy baggage is a genuine advantage of building
  fresh in 2026 vs. Bitwarden's decade of accumulated format compatibility.
- **Missing-vault-file check on login** (`auth.rs:100-108`, guarding against `rusqlite`'s silent
  `SQLITE_OPEN_CREATE` behavior masking data loss) is a sharp, specific correctness guard that
  wasn't obviously mirrored in what I read of Bitwarden's login path (their equivalent concern is
  handled server-side via the API layer, not applicable to our local-file model).
