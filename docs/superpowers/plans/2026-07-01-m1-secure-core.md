# M1 Phase A — Secure Core (pwmgr-core) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build and fully test, in isolation from any UI, the Rust crypto/registry/vault/auth/session
core that every later milestone depends on — verifiable end-to-end with `cargo test`, no Tauri or
frontend required yet.

**Architecture:** A standalone Rust library crate `pwmgr-core` (at `core/`), later consumed as a path
dependency by the Tauri binary (`src-tauri/`) in **M1 Phase B** (separate plan, covers Tauri scaffold,
IPC commands, and the React login/vault UI). Splitting core-logic from UI wiring keeps each plan
independently verifiable, per the project's architecture doc (`docs/ARCHITECTURE.md`): `crypto` has no
DB knowledge, `vault` has no password knowledge, `auth`/`session` are the only modules that ever see
the master password.

**Tech Stack:** Rust, `argon2` (Argon2id KDF), `aes-gcm` (AES-256-GCM), `rusqlite` with
`bundled-sqlcipher-vendored-openssl` (encrypted vault DB + plain registry DB), `zeroize`, `subtle`
(constant-time compare), `thiserror`, `tempfile` (dev-dependency for tests).

**Prerequisite:** Rust toolchain (`cargo`) installed. The vendored-OpenSSL SQLCipher build compiles
OpenSSL from source and needs a C compiler (MSVC Build Tools on Windows, already required by the
standard Rust Windows toolchain) and **Perl** in `PATH` (e.g. [Strawberry Perl](https://strawberryperl.com/)
on Windows). If `cargo build` fails with an OpenSSL/Perl-related error, install Strawberry Perl and
retry — this will be codified into the TRD's dependency checker in a later milestone.

---

## Task 1: Scaffold the `pwmgr-core` crate

**Files:**
- Create: `core/Cargo.toml`
- Create: `core/src/lib.rs`
- Modify: `.gitignore` (repo root)

- [ ] **Step 1: Create the library crate**

Run:
```bash
cargo new --lib core --name pwmgr-core
```
Expected: creates `core/Cargo.toml` and `core/src/lib.rs` with a default `add` function/test.

- [ ] **Step 2: Set dependencies**

Replace `core/Cargo.toml` with:

```toml
[package]
name = "pwmgr-core"
version = "0.1.0"
edition = "2021"

[dependencies]
argon2 = "0.5"
aes-gcm = "0.10"
rand = "0.8"
zeroize = { version = "1.7", features = ["derive"] }
thiserror = "1.0"
subtle = "2.5"
rusqlite = { version = "0.31", features = ["bundled-sqlcipher-vendored-openssl"] }

[dev-dependencies]
tempfile = "3.10"
```

- [ ] **Step 3: Clear the default scaffold contents**

Replace `core/src/lib.rs` with:

```rust
pub mod crypto;
```

**Important — corrected after Task 1 first shipped:** Rust requires every `pub mod` declared in
`lib.rs` to have a backing file before the crate compiles at all (a `mod x;` with no `x.rs` is a hard
compile error, `E0583`), and since the whole crate compiles as one unit, that error blocks `cargo test`
for every module, not just the missing one. Declaring all 5 modules upfront (as an earlier draft of
this task did) makes it impossible to TDD them one at a time — the first task to actually try `cargo
test` after Task 1 would find 4 unrelated compile errors for modules that don't exist yet.

The fix: **only declare `pub mod crypto;` here.** Each later task that creates a new module file adds
its own `pub mod <name>;` line to `lib.rs` as part of that task (Task 5 adds `registry`, Task 6 adds
`vault`, Task 10 adds `auth`, Task 12 adds `session` — each of those tasks' steps below now say so
explicitly). This keeps the crate compiling (and every module's own tests runnable) after every single
task, which is the actual point of TDD granularity.

- [ ] **Step 4: Add build artifacts to .gitignore**

Add to `.gitignore` at the repo root (create the file if it doesn't exist):

```
/core/target/
/src-tauri/target/
node_modules/
dist/
```

- [ ] **Step 5: Commit**

```bash
git add core/Cargo.toml core/src/lib.rs .gitignore
git commit -m "chore: scaffold pwmgr-core crate"
```

---

## Task 2: `crypto` — key derivation (Argon2id)

**Files:**
- Create: `core/src/crypto.rs`

- [ ] **Step 1: Write the failing tests**

Create `core/src/crypto.rs` with:

```rust
use argon2::{Argon2, Algorithm, Params, Version};
use rand::RngCore;
use zeroize::ZeroizeOnDrop;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("key derivation failed")]
    Kdf,
    #[error("encryption failed")]
    Encrypt,
    #[error("decryption failed: data may be tampered or the key is wrong")]
    Decrypt,
}

/// A derived 256-bit key. Zeroized automatically when dropped.
///
/// The inner bytes are private and only reachable via `as_bytes()`, which returns a borrow.
/// `[u8; 32]` is `Copy`, so a public field would let any caller do `let raw = key.0;` and hold an
/// independent copy that `ZeroizeOnDrop` never touches — quietly defeating the zeroization
/// guarantee the moment `vault.rs`/`auth.rs` need the raw bytes. A borrow-only accessor closes
/// that gap: callers can read the bytes but can't walk away with an un-zeroized copy without
/// deliberately choosing to (e.g. `*key.as_bytes()`), which is a visible, greppable smell instead
/// of an invisible default.
#[derive(ZeroizeOnDrop)]
pub struct VaultKey([u8; 32]);

impl VaultKey {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

pub fn random_salt() -> [u8; 16] {
    let mut salt = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut salt);
    salt
}

pub fn derive_key(password: &str, salt: &[u8]) -> Result<VaultKey, CryptoError> {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_key_is_deterministic_for_same_input() {
        let salt = [7u8; 16];
        let k1 = derive_key("correct horse battery staple", &salt).unwrap();
        let k2 = derive_key("correct horse battery staple", &salt).unwrap();
        assert_eq!(k1.0, k2.0);
    }

    #[test]
    fn derive_key_differs_for_different_passwords() {
        let salt = [7u8; 16];
        let k1 = derive_key("password-one", &salt).unwrap();
        let k2 = derive_key("password-two", &salt).unwrap();
        assert_ne!(k1.0, k2.0);
    }

    #[test]
    fn derive_key_differs_for_different_salts() {
        let k1 = derive_key("same password", &[1u8; 16]).unwrap();
        let k2 = derive_key("same password", &[2u8; 16]).unwrap();
        assert_ne!(k1.0, k2.0);
    }

    #[test]
    fn random_salt_is_not_all_zero_and_varies() {
        let a = random_salt();
        let b = random_salt();
        assert_ne!(a, [0u8; 16]);
        assert_ne!(a, b);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p pwmgr-core crypto:: --manifest-path core/Cargo.toml`
Expected: FAIL — compile error, `unimplemented!()` panics (any of `derive_key_*` tests panic with
"not implemented").

- [ ] **Step 3: Implement `derive_key`**

Replace the `unimplemented!()` body:

```rust
pub fn derive_key(password: &str, salt: &[u8]) -> Result<VaultKey, CryptoError> {
    let params = Params::new(65536, 3, 4, Some(32)).map_err(|_| CryptoError::Kdf)?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut out = [0u8; 32];
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut out)
        .map_err(|_| CryptoError::Kdf)?;
    Ok(VaultKey(out))
}
```

(Params: 64 MiB memory, 3 iterations, 4 lanes — the OWASP-baseline starting point from `docs/TRD.md`
§3.5, to be benchmarked and tuned once the app runs end-to-end.)

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p pwmgr-core crypto:: --manifest-path core/Cargo.toml`
Expected: PASS — all 4 tests green (may take a few seconds; Argon2id is intentionally slow).

- [ ] **Step 5: Commit**

```bash
git add core/src/crypto.rs
git commit -m "feat(crypto): add Argon2id key derivation"
```

---

## Task 3: `crypto` — AES-256-GCM seal/open with tamper detection

**Files:**
- Modify: `core/src/crypto.rs`

- [ ] **Step 1: Write the failing tests**

Append to `core/src/crypto.rs`, inside the existing `mod tests { ... }` block (add these functions
alongside the existing ones, before the closing `}`):

```rust
    #[test]
    fn seal_then_open_roundtrips() {
        let key = derive_key("master password", &[9u8; 16]).unwrap();
        let plaintext = b"hunter2-super-secret";
        let (ciphertext, nonce) = seal(&key, plaintext).unwrap();
        let opened = open(&key, &ciphertext, &nonce).unwrap();
        assert_eq!(opened, plaintext);
    }

    #[test]
    fn open_fails_with_wrong_key() {
        let key1 = derive_key("password-one", &[1u8; 16]).unwrap();
        let key2 = derive_key("password-two", &[1u8; 16]).unwrap();
        let (ciphertext, nonce) = seal(&key1, b"secret data").unwrap();
        assert!(open(&key2, &ciphertext, &nonce).is_err());
    }

    #[test]
    fn open_fails_when_ciphertext_is_tampered() {
        let key = derive_key("master password", &[3u8; 16]).unwrap();
        let (mut ciphertext, nonce) = seal(&key, b"secret data").unwrap();
        ciphertext[0] ^= 0xFF; // flip a bit
        assert!(open(&key, &ciphertext, &nonce).is_err());
    }

    #[test]
    fn seal_uses_a_fresh_nonce_each_call() {
        let key = derive_key("master password", &[4u8; 16]).unwrap();
        let (_, nonce1) = seal(&key, b"same plaintext").unwrap();
        let (_, nonce2) = seal(&key, b"same plaintext").unwrap();
        assert_ne!(nonce1, nonce2);
    }
```

Also add the (unimplemented) function signatures above `#[cfg(test)]`:

```rust
pub fn seal(key: &VaultKey, plaintext: &[u8]) -> Result<(Vec<u8>, [u8; 12]), CryptoError> {
    unimplemented!()
}

pub fn open(key: &VaultKey, ciphertext: &[u8], nonce: &[u8; 12]) -> Result<Vec<u8>, CryptoError> {
    unimplemented!()
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p pwmgr-core crypto:: --manifest-path core/Cargo.toml`
Expected: FAIL — the 4 new tests panic on `unimplemented!()`.

- [ ] **Step 3: Implement `seal` and `open`**

```rust
pub fn seal(key: &VaultKey, plaintext: &[u8]) -> Result<(Vec<u8>, [u8; 12]), CryptoError> {
    use aes_gcm::{Aes256Gcm, Key, Nonce};
    use aes_gcm::aead::{Aead, KeyInit};

    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key.0));
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|_| CryptoError::Encrypt)?;
    Ok((ciphertext, nonce_bytes))
}

pub fn open(key: &VaultKey, ciphertext: &[u8], nonce: &[u8; 12]) -> Result<Vec<u8>, CryptoError> {
    use aes_gcm::{Aes256Gcm, Key, Nonce};
    use aes_gcm::aead::{Aead, KeyInit};

    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key.0));
    let nonce = Nonce::from_slice(nonce);
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| CryptoError::Decrypt)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p pwmgr-core crypto:: --manifest-path core/Cargo.toml`
Expected: PASS — all 8 crypto tests green.

- [ ] **Step 5: Commit**

```bash
git add core/src/crypto.rs
git commit -m "feat(crypto): add AES-256-GCM seal/open with tamper detection"
```

---

## Task 4: `crypto` — constant-time verifier comparison helper

**Files:**
- Modify: `core/src/crypto.rs`

- [ ] **Step 1: Write the failing test**

Add inside `mod tests`:

```rust
    #[test]
    fn verifier_matches_detects_equal_and_unequal() {
        let key = derive_key("master password", &[5u8; 16]).unwrap();
        let same = derive_key("master password", &[5u8; 16]).unwrap();
        let different = derive_key("different password", &[5u8; 16]).unwrap();
        assert!(verifier_matches(&key, &same.0));
        assert!(!verifier_matches(&key, &different.0));
    }
```

Add the (unimplemented) signature:

```rust
pub fn verifier_matches(computed: &VaultKey, stored: &[u8]) -> bool {
    unimplemented!()
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p pwmgr-core crypto:: --manifest-path core/Cargo.toml`
Expected: FAIL — panics on `unimplemented!()`.

- [ ] **Step 3: Implement using constant-time compare**

```rust
pub fn verifier_matches(computed: &VaultKey, stored: &[u8]) -> bool {
    use subtle::ConstantTimeEq;
    computed.0.as_slice().ct_eq(stored).unwrap_u8() == 1
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p pwmgr-core crypto:: --manifest-path core/Cargo.toml`
Expected: PASS — all 9 crypto tests green.

- [ ] **Step 5: Commit**

```bash
git add core/src/crypto.rs
git commit -m "feat(crypto): add constant-time verifier comparison"
```

---

## Task 5: `registry` — unencrypted user registry (usernames + KDF salts only)

**Files:**
- Create: `core/src/registry.rs`
- Modify: `core/src/lib.rs` (add `pub mod registry;` — see Step 1b)

- [ ] **Step 1: Write the failing tests**

Create `core/src/registry.rs`:

```rust
use rusqlite::{params, Connection};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RegistryError {
    #[error("username already exists")]
    UsernameTaken,
    #[error("user not found")]
    NotFound,
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
}

#[derive(Debug, Clone)]
pub struct UserRecord {
    pub username: String,
    pub kdf_salt: Vec<u8>,
    pub verifier_salt: Vec<u8>,
    pub verifier_hash: Vec<u8>,
    pub sentinel_ciphertext: Vec<u8>,
    pub sentinel_nonce: Vec<u8>,
}

pub fn open_registry(path: &Path) -> Result<Connection, RegistryError> {
    unimplemented!()
}

pub fn create_user(conn: &Connection, rec: &UserRecord) -> Result<(), RegistryError> {
    unimplemented!()
}

pub fn find_user(conn: &Connection, username: &str) -> Result<UserRecord, RegistryError> {
    unimplemented!()
}

pub fn list_usernames(conn: &Connection) -> Result<Vec<String>, RegistryError> {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn sample_record(username: &str) -> UserRecord {
        UserRecord {
            username: username.to_string(),
            kdf_salt: vec![1, 2, 3, 4],
            verifier_salt: vec![5, 6, 7, 8],
            verifier_hash: vec![9; 32],
            sentinel_ciphertext: vec![10, 11, 12],
            sentinel_nonce: vec![13; 12],
        }
    }

    #[test]
    fn create_and_find_user_roundtrips() {
        let dir = tempdir().unwrap();
        let conn = open_registry(&dir.path().join("registry.db")).unwrap();
        create_user(&conn, &sample_record("alice")).unwrap();

        let found = find_user(&conn, "alice").unwrap();
        assert_eq!(found.username, "alice");
        assert_eq!(found.kdf_salt, vec![1, 2, 3, 4]);
        assert_eq!(found.verifier_hash, vec![9; 32]);
    }

    #[test]
    fn create_user_rejects_duplicate_username() {
        let dir = tempdir().unwrap();
        let conn = open_registry(&dir.path().join("registry.db")).unwrap();
        create_user(&conn, &sample_record("bob")).unwrap();

        let result = create_user(&conn, &sample_record("bob"));
        assert!(matches!(result, Err(RegistryError::UsernameTaken)));
    }

    #[test]
    fn find_user_returns_not_found_for_unknown_username() {
        let dir = tempdir().unwrap();
        let conn = open_registry(&dir.path().join("registry.db")).unwrap();

        let result = find_user(&conn, "nobody");
        assert!(matches!(result, Err(RegistryError::NotFound)));
    }

    #[test]
    fn list_usernames_returns_all_registered_users_sorted() {
        let dir = tempdir().unwrap();
        let conn = open_registry(&dir.path().join("registry.db")).unwrap();
        create_user(&conn, &sample_record("zoe")).unwrap();
        create_user(&conn, &sample_record("amy")).unwrap();

        let names = list_usernames(&conn).unwrap();
        assert_eq!(names, vec!["amy".to_string(), "zoe".to_string()]);
    }
}
```

- [ ] **Step 1b: Declare the module in `lib.rs`**

Add `pub mod registry;` to `core/src/lib.rs` (it currently only has `pub mod crypto;` from Task 1).
Result:
```rust
pub mod crypto;
pub mod registry;
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p pwmgr-core registry:: --manifest-path core/Cargo.toml`
Expected: FAIL — panics on `unimplemented!()`.

- [ ] **Step 3: Implement the registry functions**

Replace the four `unimplemented!()` bodies:

```rust
pub fn open_registry(path: &Path) -> Result<Connection, RegistryError> {
    let conn = Connection::open(path)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS users (
            username TEXT PRIMARY KEY,
            kdf_salt BLOB NOT NULL,
            verifier_salt BLOB NOT NULL,
            verifier_hash BLOB NOT NULL,
            sentinel_ciphertext BLOB NOT NULL,
            sentinel_nonce BLOB NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )?;
    Ok(conn)
}

pub fn create_user(conn: &Connection, rec: &UserRecord) -> Result<(), RegistryError> {
    let exists: Option<i64> = conn
        .query_row(
            "SELECT 1 FROM users WHERE username = ?1",
            params![rec.username],
            |row| row.get(0),
        )
        .ok();
    if exists.is_some() {
        return Err(RegistryError::UsernameTaken);
    }
    conn.execute(
        "INSERT INTO users (username, kdf_salt, verifier_salt, verifier_hash, sentinel_ciphertext, sentinel_nonce)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            rec.username,
            rec.kdf_salt,
            rec.verifier_salt,
            rec.verifier_hash,
            rec.sentinel_ciphertext,
            rec.sentinel_nonce
        ],
    )?;
    Ok(())
}

pub fn find_user(conn: &Connection, username: &str) -> Result<UserRecord, RegistryError> {
    conn.query_row(
        "SELECT username, kdf_salt, verifier_salt, verifier_hash, sentinel_ciphertext, sentinel_nonce
         FROM users WHERE username = ?1",
        params![username],
        |row| {
            Ok(UserRecord {
                username: row.get(0)?,
                kdf_salt: row.get(1)?,
                verifier_salt: row.get(2)?,
                verifier_hash: row.get(3)?,
                sentinel_ciphertext: row.get(4)?,
                sentinel_nonce: row.get(5)?,
            })
        },
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => RegistryError::NotFound,
        other => RegistryError::Db(other),
    })
}

pub fn list_usernames(conn: &Connection) -> Result<Vec<String>, RegistryError> {
    let mut stmt = conn.prepare("SELECT username FROM users ORDER BY username")?;
    let rows = stmt.query_map([], |row| row.get(0))?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p pwmgr-core registry:: --manifest-path core/Cargo.toml`
Expected: PASS — all 4 registry tests green.

- [ ] **Step 5: Commit**

```bash
git add core/src/registry.rs core/src/lib.rs
git commit -m "feat(registry): add unencrypted user registry (usernames + salts)"
```

---

## Task 6: `vault` — SQLCipher open with wrong-key rejection

**Files:**
- Create: `core/src/vault.rs`
- Modify: `core/src/lib.rs` (add `pub mod vault;` — see Step 1b)

- [ ] **Step 1: Write the failing tests**

Create `core/src/vault.rs`:

```rust
use crate::crypto::VaultKey;
use rusqlite::Connection;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum VaultError {
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("wrong key or corrupted vault")]
    UnlockFailed,
    #[error("record not found")]
    NotFound,
    #[error("crypto error: {0}")]
    Crypto(#[from] crate::crypto::CryptoError),
}

pub fn open_vault(path: &Path, key: &VaultKey) -> Result<Connection, VaultError> {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::derive_key;
    use tempfile::tempdir;

    #[test]
    fn open_vault_with_correct_key_succeeds_and_creates_schema() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.vault");
        let key = derive_key("master password", &[1u8; 16]).unwrap();

        let conn = open_vault(&path, &key).unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='sites'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn reopen_vault_with_wrong_key_fails() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.vault");
        let right_key = derive_key("correct password", &[2u8; 16]).unwrap();
        let wrong_key = derive_key("wrong password", &[2u8; 16]).unwrap();

        // Create it with the right key first.
        open_vault(&path, &right_key).unwrap();
        drop(right_key);

        let result = open_vault(&path, &wrong_key);
        assert!(matches!(result, Err(VaultError::UnlockFailed)));
    }

    #[test]
    fn reopen_vault_with_correct_key_succeeds() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.vault");
        let key = derive_key("correct password", &[3u8; 16]).unwrap();

        open_vault(&path, &key).unwrap();
        let reopened = open_vault(&path, &key);
        assert!(reopened.is_ok());
    }
}
```

- [ ] **Step 1b: Declare the module in `lib.rs`**

Add `pub mod vault;` to `core/src/lib.rs`. Result:
```rust
pub mod crypto;
pub mod registry;
pub mod vault;
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p pwmgr-core vault:: --manifest-path core/Cargo.toml`
Expected: FAIL — panics on `unimplemented!()`.

- [ ] **Step 3: Implement `open_vault` with schema init**

```rust
pub fn open_vault(path: &Path, key: &VaultKey) -> Result<Connection, VaultError> {
    let conn = Connection::open(path)?;
    let key_hex: String = key.as_bytes().iter().map(|b| format!("{:02x}", b)).collect();
    conn.pragma_update(None, "key", format!("x'{}'", key_hex))?;

    // SQLCipher only actually verifies the key once you touch the schema.
    conn.query_row("SELECT count(*) FROM sqlite_master", [], |row| row.get::<_, i64>(0))
        .map_err(|_| VaultError::UnlockFailed)?;

    init_schema(&conn)?;
    Ok(conn)
}

fn init_schema(conn: &Connection) -> Result<(), VaultError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS sites (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            url TEXT,
            notes TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            deleted_at TEXT
        );
        CREATE TABLE IF NOT EXISTS credentials (
            id INTEGER PRIMARY KEY,
            site_id INTEGER NOT NULL REFERENCES sites(id),
            username TEXT,
            secret_enc BLOB NOT NULL,
            secret_nonce BLOB NOT NULL,
            remarks TEXT,
            password_updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            deleted_at TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_sites_name ON sites(name);
        CREATE INDEX IF NOT EXISTS idx_credentials_site ON credentials(site_id);",
    )?;
    Ok(())
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p pwmgr-core vault:: --manifest-path core/Cargo.toml`
Expected: PASS — all 3 vault tests green.

- [ ] **Step 5: Commit**

```bash
git add core/src/vault.rs core/src/lib.rs
git commit -m "feat(vault): add SQLCipher open with wrong-key rejection + schema"
```

---

## Task 7: `vault` — site CRUD + soft delete

**Files:**
- Modify: `core/src/vault.rs`

- [ ] **Step 1: Write the failing tests**

Add above `#[cfg(test)]` in `core/src/vault.rs`:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct Site {
    pub id: i64,
    pub name: String,
    pub url: Option<String>,
    pub notes: Option<String>,
}

pub fn add_site(conn: &Connection, name: &str, url: Option<&str>, notes: Option<&str>) -> Result<i64, VaultError> {
    unimplemented!()
}

pub fn list_sites(conn: &Connection) -> Result<Vec<Site>, VaultError> {
    unimplemented!()
}

pub fn soft_delete_site(conn: &Connection, site_id: i64) -> Result<(), VaultError> {
    unimplemented!()
}
```

Add inside `mod tests`:

```rust
    #[test]
    fn add_site_then_list_sites_returns_it() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[4u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();

        let id = add_site(&conn, "example.com", Some("https://example.com"), Some("note")).unwrap();
        let sites = list_sites(&conn).unwrap();

        assert_eq!(sites.len(), 1);
        assert_eq!(sites[0].id, id);
        assert_eq!(sites[0].name, "example.com");
    }

    #[test]
    fn soft_deleted_site_is_excluded_from_list_sites() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[5u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();

        let id = add_site(&conn, "example.com", None, None).unwrap();
        soft_delete_site(&conn, id).unwrap();

        let sites = list_sites(&conn).unwrap();
        assert!(sites.is_empty());
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p pwmgr-core vault:: --manifest-path core/Cargo.toml`
Expected: FAIL — panics on `unimplemented!()`.

- [ ] **Step 3: Implement site CRUD**

```rust
pub fn add_site(conn: &Connection, name: &str, url: Option<&str>, notes: Option<&str>) -> Result<i64, VaultError> {
    conn.execute(
        "INSERT INTO sites (name, url, notes) VALUES (?1, ?2, ?3)",
        rusqlite::params![name, url, notes],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn list_sites(conn: &Connection) -> Result<Vec<Site>, VaultError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, url, notes FROM sites WHERE deleted_at IS NULL ORDER BY name",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Site {
            id: row.get(0)?,
            name: row.get(1)?,
            url: row.get(2)?,
            notes: row.get(3)?,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub fn soft_delete_site(conn: &Connection, site_id: i64) -> Result<(), VaultError> {
    let updated = conn.execute(
        "UPDATE sites SET deleted_at = datetime('now') WHERE id = ?1 AND deleted_at IS NULL",
        rusqlite::params![site_id],
    )?;
    if updated == 0 {
        return Err(VaultError::NotFound);
    }
    Ok(())
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p pwmgr-core vault:: --manifest-path core/Cargo.toml`
Expected: PASS — all 5 vault tests green.

- [ ] **Step 5: Commit**

```bash
git add core/src/vault.rs
git commit -m "feat(vault): add site CRUD with soft delete"
```

---

## Task 8: `vault` — credential CRUD (encrypted secret) + soft delete/restore

**Files:**
- Modify: `core/src/vault.rs`

- [ ] **Step 1: Write the failing tests**

Add above `#[cfg(test)]`:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct Credential {
    pub id: i64,
    pub site_id: i64,
    pub username: Option<String>,
    pub secret: String,
    pub remarks: Option<String>,
}

pub fn add_credential(
    conn: &Connection,
    key: &VaultKey,
    site_id: i64,
    username: Option<&str>,
    secret: &str,
    remarks: Option<&str>,
) -> Result<i64, VaultError> {
    unimplemented!()
}

pub fn list_credentials_for_site(
    conn: &Connection,
    key: &VaultKey,
    site_id: i64,
) -> Result<Vec<Credential>, VaultError> {
    unimplemented!()
}

pub fn soft_delete_credential(conn: &Connection, credential_id: i64) -> Result<(), VaultError> {
    unimplemented!()
}

pub fn restore_credential(conn: &Connection, credential_id: i64) -> Result<(), VaultError> {
    unimplemented!()
}
```

Add inside `mod tests`:

```rust
    #[test]
    fn add_credential_then_list_decrypts_the_secret() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[6u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();
        let site_id = add_site(&conn, "example.com", None, None).unwrap();

        let cred_id = add_credential(&conn, &key, site_id, Some("alice"), "s3cret!", Some("work login")).unwrap();
        let creds = list_credentials_for_site(&conn, &key, site_id).unwrap();

        assert_eq!(creds.len(), 1);
        assert_eq!(creds[0].id, cred_id);
        assert_eq!(creds[0].username.as_deref(), Some("alice"));
        assert_eq!(creds[0].secret, "s3cret!");
        assert_eq!(creds[0].remarks.as_deref(), Some("work login"));
    }

    #[test]
    fn soft_delete_then_restore_credential_round_trips() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[7u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();
        let site_id = add_site(&conn, "example.com", None, None).unwrap();
        let cred_id = add_credential(&conn, &key, site_id, Some("alice"), "s3cret!", None).unwrap();

        soft_delete_credential(&conn, cred_id).unwrap();
        assert!(list_credentials_for_site(&conn, &key, site_id).unwrap().is_empty());

        restore_credential(&conn, cred_id).unwrap();
        assert_eq!(list_credentials_for_site(&conn, &key, site_id).unwrap().len(), 1);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p pwmgr-core vault:: --manifest-path core/Cargo.toml`
Expected: FAIL — panics on `unimplemented!()`.

- [ ] **Step 3: Implement credential CRUD**

```rust
pub fn add_credential(
    conn: &Connection,
    key: &VaultKey,
    site_id: i64,
    username: Option<&str>,
    secret: &str,
    remarks: Option<&str>,
) -> Result<i64, VaultError> {
    let (secret_enc, nonce) = crate::crypto::seal(key, secret.as_bytes())?;
    conn.execute(
        "INSERT INTO credentials (site_id, username, secret_enc, secret_nonce, remarks)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![site_id, username, secret_enc, nonce.to_vec(), remarks],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn list_credentials_for_site(
    conn: &Connection,
    key: &VaultKey,
    site_id: i64,
) -> Result<Vec<Credential>, VaultError> {
    let mut stmt = conn.prepare(
        "SELECT id, site_id, username, secret_enc, secret_nonce, remarks
         FROM credentials WHERE site_id = ?1 AND deleted_at IS NULL ORDER BY id",
    )?;
    let rows = stmt.query_map(rusqlite::params![site_id], |row| {
        let id: i64 = row.get(0)?;
        let site_id: i64 = row.get(1)?;
        let username: Option<String> = row.get(2)?;
        let secret_enc: Vec<u8> = row.get(3)?;
        let secret_nonce: Vec<u8> = row.get(4)?;
        let remarks: Option<String> = row.get(5)?;
        Ok((id, site_id, username, secret_enc, secret_nonce, remarks))
    })?;

    let mut out = Vec::new();
    for r in rows {
        let (id, site_id, username, secret_enc, secret_nonce, remarks) = r?;
        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(&secret_nonce);
        let plaintext = crate::crypto::open(key, &secret_enc, &nonce)?;
        let secret = String::from_utf8(plaintext).map_err(|_| VaultError::UnlockFailed)?;
        out.push(Credential { id, site_id, username, secret, remarks });
    }
    Ok(out)
}

pub fn soft_delete_credential(conn: &Connection, credential_id: i64) -> Result<(), VaultError> {
    let updated = conn.execute(
        "UPDATE credentials SET deleted_at = datetime('now') WHERE id = ?1 AND deleted_at IS NULL",
        rusqlite::params![credential_id],
    )?;
    if updated == 0 {
        return Err(VaultError::NotFound);
    }
    Ok(())
}

pub fn restore_credential(conn: &Connection, credential_id: i64) -> Result<(), VaultError> {
    let updated = conn.execute(
        "UPDATE credentials SET deleted_at = NULL WHERE id = ?1 AND deleted_at IS NOT NULL",
        rusqlite::params![credential_id],
    )?;
    if updated == 0 {
        return Err(VaultError::NotFound);
    }
    Ok(())
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p pwmgr-core vault:: --manifest-path core/Cargo.toml`
Expected: PASS — all 7 vault tests green.

- [ ] **Step 5: Commit**

```bash
git add core/src/vault.rs
git commit -m "feat(vault): add encrypted credential CRUD with soft delete/restore"
```

---

## Task 9: `vault` — real-time search across sites and credentials

**Files:**
- Modify: `core/src/vault.rs`

- [ ] **Step 1: Write the failing test**

Add above `#[cfg(test)]`:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct SearchResult {
    pub site_id: i64,
    pub site_name: String,
    pub credential_id: i64,
    pub username: Option<String>,
}

pub fn search(conn: &Connection, query: &str) -> Result<Vec<SearchResult>, VaultError> {
    unimplemented!()
}
```

Add inside `mod tests`:

```rust
    #[test]
    fn search_matches_site_name_username_and_remarks_case_insensitively() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[8u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();

        let site_a = add_site(&conn, "GitHub", None, None).unwrap();
        add_credential(&conn, &key, site_a, Some("alice"), "secret1", Some("work account")).unwrap();

        let site_b = add_site(&conn, "example.com", None, None).unwrap();
        add_credential(&conn, &key, site_b, Some("bob"), "secret2", None).unwrap();

        let by_site_name = search(&conn, "github").unwrap();
        assert_eq!(by_site_name.len(), 1);
        assert_eq!(by_site_name[0].site_name, "GitHub");

        let by_username = search(&conn, "bob").unwrap();
        assert_eq!(by_username.len(), 1);
        assert_eq!(by_username[0].username.as_deref(), Some("bob"));

        let by_remarks = search(&conn, "work").unwrap();
        assert_eq!(by_remarks.len(), 1);
        assert_eq!(by_remarks[0].site_name, "GitHub");

        let no_match = search(&conn, "nonexistent").unwrap();
        assert!(no_match.is_empty());
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p pwmgr-core vault:: --manifest-path core/Cargo.toml`
Expected: FAIL — panics on `unimplemented!()`.

- [ ] **Step 3: Implement `search`**

```rust
pub fn search(conn: &Connection, query: &str) -> Result<Vec<SearchResult>, VaultError> {
    let pattern = format!("%{}%", query.to_lowercase());
    let mut stmt = conn.prepare(
        "SELECT s.id, s.name, c.id, c.username
         FROM credentials c
         JOIN sites s ON s.id = c.site_id
         WHERE c.deleted_at IS NULL AND s.deleted_at IS NULL
           AND (
             lower(s.name) LIKE ?1
             OR lower(coalesce(c.username, '')) LIKE ?1
             OR lower(coalesce(c.remarks, '')) LIKE ?1
           )
         ORDER BY s.name",
    )?;
    let rows = stmt.query_map(rusqlite::params![pattern], |row| {
        Ok(SearchResult {
            site_id: row.get(0)?,
            site_name: row.get(1)?,
            credential_id: row.get(2)?,
            username: row.get(3)?,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p pwmgr-core vault:: --manifest-path core/Cargo.toml`
Expected: PASS — all 8 vault tests green.

- [ ] **Step 5: Commit**

```bash
git add core/src/vault.rs
git commit -m "feat(vault): add case-insensitive search across sites and credentials"
```

---

## Task 10: `auth` — register_user

**Files:**
- Create: `core/src/auth.rs`
- Modify: `core/src/lib.rs` (add `pub mod auth;` — see Step 1b)

- [ ] **Step 1: Write the failing test**

Create `core/src/auth.rs`:

```rust
use crate::{crypto, registry, vault};
use rusqlite::Connection;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("username already exists")]
    UsernameTaken,
    #[error("incorrect username or master password")]
    InvalidCredentials,
    #[error("registry error: {0}")]
    Registry(#[from] registry::RegistryError),
    #[error("vault error: {0}")]
    Vault(#[from] vault::VaultError),
    #[error("crypto error: {0}")]
    Crypto(#[from] crypto::CryptoError),
}

const SENTINEL_PLAINTEXT: &[u8] = b"pwmgr-sentinel-v1";

pub fn register_user(
    registry_conn: &Connection,
    vault_path: &Path,
    username: &str,
    master_password: &str,
) -> Result<crypto::VaultKey, AuthError> {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn register_user_creates_registry_entry_and_openable_vault() {
        let dir = tempdir().unwrap();
        let registry_conn = registry::open_registry(&dir.path().join("registry.db")).unwrap();
        let vault_path = dir.path().join("alice.vault");

        let key = register_user(&registry_conn, &vault_path, "alice", "correct horse battery staple").unwrap();

        assert!(registry::find_user(&registry_conn, "alice").is_ok());
        assert!(vault::open_vault(&vault_path, &key).is_ok());
    }

    #[test]
    fn register_user_rejects_duplicate_username() {
        let dir = tempdir().unwrap();
        let registry_conn = registry::open_registry(&dir.path().join("registry.db")).unwrap();

        register_user(&registry_conn, &dir.path().join("alice1.vault"), "alice", "password one").unwrap();
        let result = register_user(&registry_conn, &dir.path().join("alice2.vault"), "alice", "password two");

        assert!(matches!(result, Err(AuthError::UsernameTaken)));
    }
}
```

- [ ] **Step 1b: Declare the module in `lib.rs`**

Add `pub mod auth;` to `core/src/lib.rs`. Result:
```rust
pub mod crypto;
pub mod registry;
pub mod vault;
pub mod auth;
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p pwmgr-core auth:: --manifest-path core/Cargo.toml`
Expected: FAIL — panics on `unimplemented!()`.

- [ ] **Step 3: Implement `register_user`**

```rust
pub fn register_user(
    registry_conn: &Connection,
    vault_path: &Path,
    username: &str,
    master_password: &str,
) -> Result<crypto::VaultKey, AuthError> {
    let kdf_salt = crypto::random_salt();
    let verifier_salt = crypto::random_salt();

    let vault_key = crypto::derive_key(master_password, &kdf_salt)?;
    let verifier_key = crypto::derive_key(master_password, &verifier_salt)?;
    let (sentinel_ciphertext, sentinel_nonce) = crypto::seal(&vault_key, SENTINEL_PLAINTEXT)?;

    let rec = registry::UserRecord {
        username: username.to_string(),
        kdf_salt: kdf_salt.to_vec(),
        verifier_salt: verifier_salt.to_vec(),
        verifier_hash: verifier_key.as_bytes().to_vec(),
        sentinel_ciphertext,
        sentinel_nonce: sentinel_nonce.to_vec(),
    };

    registry::create_user(registry_conn, &rec).map_err(|e| match e {
        registry::RegistryError::UsernameTaken => AuthError::UsernameTaken,
        other => AuthError::Registry(other),
    })?;

    vault::open_vault(vault_path, &vault_key)?;
    Ok(vault_key)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p pwmgr-core auth:: --manifest-path core/Cargo.toml`
Expected: PASS — both auth tests green.

- [ ] **Step 5: Commit**

```bash
git add core/src/auth.rs core/src/lib.rs
git commit -m "feat(auth): add register_user tying crypto, registry, and vault together"
```

---

## Task 11: `auth` — login with wrong-password rejection

**Files:**
- Modify: `core/src/auth.rs`

- [ ] **Step 1: Write the failing tests**

Add above `#[cfg(test)]`:

```rust
pub fn login(
    registry_conn: &Connection,
    vault_path: &Path,
    username: &str,
    master_password: &str,
) -> Result<crypto::VaultKey, AuthError> {
    unimplemented!()
}
```

Add inside `mod tests`:

```rust
    #[test]
    fn login_with_correct_password_succeeds() {
        let dir = tempdir().unwrap();
        let registry_conn = registry::open_registry(&dir.path().join("registry.db")).unwrap();
        let vault_path = dir.path().join("alice.vault");
        register_user(&registry_conn, &vault_path, "alice", "correct horse battery staple").unwrap();

        let result = login(&registry_conn, &vault_path, "alice", "correct horse battery staple");
        assert!(result.is_ok());
    }

    #[test]
    fn login_with_wrong_password_is_rejected() {
        let dir = tempdir().unwrap();
        let registry_conn = registry::open_registry(&dir.path().join("registry.db")).unwrap();
        let vault_path = dir.path().join("alice.vault");
        register_user(&registry_conn, &vault_path, "alice", "correct horse battery staple").unwrap();

        let result = login(&registry_conn, &vault_path, "alice", "wrong password");
        assert!(matches!(result, Err(AuthError::InvalidCredentials)));
    }

    #[test]
    fn login_with_unknown_username_is_rejected_with_same_generic_error() {
        let dir = tempdir().unwrap();
        let registry_conn = registry::open_registry(&dir.path().join("registry.db")).unwrap();
        let vault_path = dir.path().join("nobody.vault");

        let result = login(&registry_conn, &vault_path, "nobody", "any password");
        assert!(matches!(result, Err(AuthError::InvalidCredentials)));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p pwmgr-core auth:: --manifest-path core/Cargo.toml`
Expected: FAIL — panics on `unimplemented!()`.

- [ ] **Step 3: Implement `login`**

```rust
pub fn login(
    registry_conn: &Connection,
    vault_path: &Path,
    username: &str,
    master_password: &str,
) -> Result<crypto::VaultKey, AuthError> {
    let rec = registry::find_user(registry_conn, username).map_err(|_| AuthError::InvalidCredentials)?;

    let verifier_key =
        crypto::derive_key(master_password, &rec.verifier_salt).map_err(|_| AuthError::InvalidCredentials)?;
    if !crypto::verifier_matches(&verifier_key, &rec.verifier_hash) {
        return Err(AuthError::InvalidCredentials);
    }

    let vault_key =
        crypto::derive_key(master_password, &rec.kdf_salt).map_err(|_| AuthError::InvalidCredentials)?;

    let mut nonce = [0u8; 12];
    if rec.sentinel_nonce.len() != 12 {
        return Err(AuthError::InvalidCredentials);
    }
    nonce.copy_from_slice(&rec.sentinel_nonce);
    crypto::open(&vault_key, &rec.sentinel_ciphertext, &nonce).map_err(|_| AuthError::InvalidCredentials)?;

    vault::open_vault(vault_path, &vault_key).map_err(|_| AuthError::InvalidCredentials)?;

    Ok(vault_key)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p pwmgr-core auth:: --manifest-path core/Cargo.toml`
Expected: PASS — all 5 auth tests green.

- [ ] **Step 5: Commit**

```bash
git add core/src/auth.rs
git commit -m "feat(auth): add login with verifier check and generic-error rejection"
```

---

## Task 12: `session` — in-memory key holder with idle auto-lock

**Files:**
- Create: `core/src/session.rs`
- Modify: `core/src/lib.rs` (add `pub mod session;` — see Step 1b)

- [ ] **Step 1: Write the failing tests**

Create `core/src/session.rs`:

```rust
use crate::crypto::VaultKey;
use std::cell::RefCell;
use std::time::{Duration, Instant};

pub trait Clock {
    fn now(&self) -> Instant;
}

pub struct SystemClock;
impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

pub struct Session {
    key: Option<VaultKey>,
    last_activity: Option<Instant>,
    timeout: Duration,
}

impl Session {
    pub fn new(timeout: Duration) -> Self {
        unimplemented!()
    }

    pub fn unlock(&mut self, key: VaultKey, clock: &dyn Clock) {
        unimplemented!()
    }

    pub fn touch(&mut self, clock: &dyn Clock) {
        unimplemented!()
    }

    pub fn lock(&mut self) {
        unimplemented!()
    }

    pub fn is_locked(&self) -> bool {
        unimplemented!()
    }

    pub fn check_expiry(&mut self, clock: &dyn Clock) {
        unimplemented!()
    }

    pub fn key(&self) -> Option<&VaultKey> {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::derive_key;

    struct FakeClock(RefCell<Instant>);
    impl FakeClock {
        fn new() -> Self {
            FakeClock(RefCell::new(Instant::now()))
        }
        fn advance(&self, d: Duration) {
            *self.0.borrow_mut() += d;
        }
    }
    impl Clock for FakeClock {
        fn now(&self) -> Instant {
            *self.0.borrow()
        }
    }

    #[test]
    fn new_session_starts_locked() {
        let session = Session::new(Duration::from_secs(300));
        assert!(session.is_locked());
        assert!(session.key().is_none());
    }

    #[test]
    fn unlock_makes_session_unlocked_and_exposes_key() {
        let clock = FakeClock::new();
        let mut session = Session::new(Duration::from_secs(300));
        let key = derive_key("pw", &[1u8; 16]).unwrap();

        session.unlock(key, &clock);

        assert!(!session.is_locked());
        assert!(session.key().is_some());
    }

    #[test]
    fn lock_clears_the_key() {
        let clock = FakeClock::new();
        let mut session = Session::new(Duration::from_secs(300));
        session.unlock(derive_key("pw", &[1u8; 16]).unwrap(), &clock);

        session.lock();

        assert!(session.is_locked());
        assert!(session.key().is_none());
    }

    #[test]
    fn check_expiry_locks_after_timeout_elapses_with_no_activity() {
        let clock = FakeClock::new();
        let mut session = Session::new(Duration::from_secs(300));
        session.unlock(derive_key("pw", &[1u8; 16]).unwrap(), &clock);

        clock.advance(Duration::from_secs(301));
        session.check_expiry(&clock);

        assert!(session.is_locked());
    }

    #[test]
    fn touch_resets_the_idle_timer_preventing_expiry() {
        let clock = FakeClock::new();
        let mut session = Session::new(Duration::from_secs(300));
        session.unlock(derive_key("pw", &[1u8; 16]).unwrap(), &clock);

        clock.advance(Duration::from_secs(200));
        session.touch(&clock);
        clock.advance(Duration::from_secs(200)); // 400s since unlock, but only 200s since touch
        session.check_expiry(&clock);

        assert!(!session.is_locked());
    }

    #[test]
    fn check_expiry_on_already_locked_session_is_a_no_op() {
        let clock = FakeClock::new();
        let mut session = Session::new(Duration::from_secs(300));
        session.check_expiry(&clock); // locked session, nothing to expire
        assert!(session.is_locked());
    }
}
```

- [ ] **Step 1b: Declare the module in `lib.rs`**

Add `pub mod session;` to `core/src/lib.rs`. Result (all 5 modules now declared):
```rust
pub mod crypto;
pub mod registry;
pub mod vault;
pub mod auth;
pub mod session;
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p pwmgr-core session:: --manifest-path core/Cargo.toml`
Expected: FAIL — panics on `unimplemented!()`.

- [ ] **Step 3: Implement `Session`**

```rust
impl Session {
    pub fn new(timeout: Duration) -> Self {
        Session { key: None, last_activity: None, timeout }
    }

    pub fn unlock(&mut self, key: VaultKey, clock: &dyn Clock) {
        self.key = Some(key);
        self.last_activity = Some(clock.now());
    }

    pub fn touch(&mut self, clock: &dyn Clock) {
        if self.key.is_some() {
            self.last_activity = Some(clock.now());
        }
    }

    pub fn lock(&mut self) {
        self.key = None; // VaultKey is ZeroizeOnDrop — dropping it here zeroizes the bytes.
        self.last_activity = None;
    }

    pub fn is_locked(&self) -> bool {
        self.key.is_none()
    }

    pub fn check_expiry(&mut self, clock: &dyn Clock) {
        if let Some(last) = self.last_activity {
            if clock.now().duration_since(last) >= self.timeout {
                self.lock();
            }
        }
    }

    pub fn key(&self) -> Option<&VaultKey> {
        self.key.as_ref()
    }
}
```

(Replace the five `unimplemented!()` method bodies with this `impl` block.)

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p pwmgr-core session:: --manifest-path core/Cargo.toml`
Expected: PASS — all 6 session tests green.

- [ ] **Step 5: Commit**

```bash
git add core/src/session.rs
git add core/src/session.rs core/src/lib.rs
git commit -m "feat(session): add idle-timeout auto-lock with injectable clock for testing"
```

---

## Task 13: End-to-end integration test — the M1 "Verify" scenario

**Files:**
- Create: `core/tests/end_to_end.rs`

- [ ] **Step 1: Write the integration test**

Create `core/tests/end_to_end.rs`:

```rust
use pwmgr_core::{auth, registry, session, vault};
use std::time::Duration;
use tempfile::tempdir;

/// Mirrors the M1 milestone's "Verify" line from docs/superpowers/plans — this is the
/// single test that proves the secure core actually delivers a usable, safe vault:
/// create user -> add/edit/delete/search entries -> lock -> reopen -> data intact & encrypted.
#[test]
fn full_lifecycle_register_use_lock_reopen() {
    let dir = tempdir().unwrap();
    let registry_path = dir.path().join("registry.db");
    let vault_path = dir.path().join("alice.vault");

    let registry_conn = registry::open_registry(&registry_path).unwrap();

    // Register.
    let key = auth::register_user(&registry_conn, &vault_path, "alice", "correct horse battery staple").unwrap();
    let vault_conn = vault::open_vault(&vault_path, &key).unwrap();

    // Add.
    let site_id = vault::add_site(&vault_conn, "github.com", Some("https://github.com"), None).unwrap();
    let cred_id = vault::add_credential(&vault_conn, &key, site_id, Some("alice"), "s3cret!", Some("work")).unwrap();

    // Search finds it.
    let results = vault::search(&vault_conn, "github").unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].credential_id, cred_id);

    // Soft-delete then restore (edit/delete lifecycle).
    vault::soft_delete_credential(&vault_conn, cred_id).unwrap();
    assert!(vault::list_credentials_for_site(&vault_conn, &key, site_id).unwrap().is_empty());
    vault::restore_credential(&vault_conn, cred_id).unwrap();

    // Lock (simulated via Session, not just dropping the connection).
    let mut sess = session::Session::new(Duration::from_secs(300));
    sess.unlock(key, &session::SystemClock);
    drop(vault_conn);
    sess.lock();
    assert!(sess.is_locked());

    // Reopen: wrong password must fail, correct password must succeed with data intact.
    let wrong = auth::login(&registry_conn, &vault_path, "alice", "not the password");
    assert!(wrong.is_err());

    let reopened_key = auth::login(&registry_conn, &vault_path, "alice", "correct horse battery staple").unwrap();
    let reopened_conn = vault::open_vault(&vault_path, &reopened_key).unwrap();
    let creds = vault::list_credentials_for_site(&reopened_conn, &reopened_key, site_id).unwrap();

    assert_eq!(creds.len(), 1);
    assert_eq!(creds[0].secret, "s3cret!");
    assert_eq!(creds[0].username.as_deref(), Some("alice"));
}

/// Proves the on-disk file is not readable without the key: raw bytes must not contain
/// the plaintext secret or site name.
#[test]
fn vault_file_on_disk_contains_no_plaintext_secrets() {
    let dir = tempdir().unwrap();
    let registry_path = dir.path().join("registry.db");
    let vault_path = dir.path().join("bob.vault");
    let registry_conn = registry::open_registry(&registry_path).unwrap();

    let key = auth::register_user(&registry_conn, &vault_path, "bob", "another master password").unwrap();
    let vault_conn = vault::open_vault(&vault_path, &key).unwrap();
    let site_id = vault::add_site(&vault_conn, "very-unique-site-name.example", None, None).unwrap();
    vault::add_credential(&vault_conn, &key, site_id, Some("bob"), "extremely-unique-secret-xyz", None).unwrap();
    drop(vault_conn);

    let raw = std::fs::read(&vault_path).unwrap();
    let raw_str = String::from_utf8_lossy(&raw);
    assert!(!raw_str.contains("very-unique-site-name.example"));
    assert!(!raw_str.contains("extremely-unique-secret-xyz"));
}
```

- [ ] **Step 2: Run the integration tests**

Run: `cargo test -p pwmgr-core --test end_to_end --manifest-path core/Cargo.toml`
Expected: PASS — both tests green. (No fail-first step here: this test only exercises functions
already implemented and tested in Tasks 2–11; it is a composition check, not new units.)

- [ ] **Step 3: Run the full test suite as a final gate**

Run: `cargo test --manifest-path core/Cargo.toml`
Expected: PASS — every test across `crypto`, `registry`, `vault`, `auth`, `session`, and
`end_to_end` is green (30+ tests total).

- [ ] **Step 4: Commit**

```bash
git add core/tests/end_to_end.rs
git commit -m "test: add end-to-end integration test proving the M1 verify scenario"
```

---

## Self-Review Notes (for whoever executes this plan)

- **Spec coverage:** every M1 bullet from `docs/PRD.md` §7 that belongs to the core (register/login,
  encrypted CRUD, soft-delete trash, search, auto-lock) has a task and passing tests above. UI, theme,
  and tooltips are explicitly **out of scope for this plan** — they belong to the M1 Phase B plan
  (Tauri scaffold + IPC commands + React screens), to be written next via the writing-plans skill once
  this crate is green.
- **Threat model traceability:** wrong-key rejection (Task 6), tamper detection (Task 3), constant-time
  verifier compare (Task 4), generic login error (Task 11), and key zeroization on lock (Task 12) each
  map directly to a mitigation row in `docs/THREAT_MODEL.md` §4.
- **Known follow-up, not a blocker:** Argon2id parameters (Task 2, Step 3) are the OWASP-baseline
  starting point; benchmark and tune once the full app is running, per `docs/TRD.md` §3.5.
