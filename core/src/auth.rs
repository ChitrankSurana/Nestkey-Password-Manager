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

// Fixed, non-secret salt used ONLY to burn an Argon2id derivation of equivalent cost when the
// username doesn't exist, so "unknown username" and "known username, wrong password" take the same
// wall-clock time. Found in Task 11's code-quality review pass: without this, find_user's fast
// indexed-lookup failure returned immediately (skipping both derive_key calls entirely) while a
// wrong password for a real user always paid the full ~100ms-1s Argon2id cost first - an attacker
// could enumerate valid usernames by timing alone, even though both paths return the identical
// AuthError::InvalidCredentials value.
const DECOY_SALT: [u8; 16] = [0u8; 16];

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

    // Create the vault file BEFORE committing the registry row. If vault creation fails (disk
    // full, permission error), nothing is left committed and a retry is clean. The reverse
    // ordering can leave an orphaned registry row that permanently blocks the username with no
    // vault to log into, since a retry would always hit UsernameTaken - see the Task 10 code
    // review discussion in this plan's history. The residual failure mode here (an unclaimed
    // vault file left on disk if the registry insert fails afterward) is self-healing: a retry's
    // open_vault call just reopens the same correctly-keyed file rather than creating a new one.
    vault::open_vault(vault_path, &vault_key)?;

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

    Ok(vault_key)
}

/// Cheap re-auth check for an already-unlocked session (the master-password re-prompt gate on a
/// sensitive field): only the verifier_salt/verifier_hash comparison `login` also does, without
/// re-deriving the vault key or reopening the vault file - the vault is already open, this just
/// needs a yes/no "did they type the right master password."
pub fn verify_master_password(
    registry_conn: &Connection,
    username: &str,
    master_password: &str,
) -> Result<bool, AuthError> {
    let rec = match registry::find_user(registry_conn, username) {
        Ok(rec) => rec,
        Err(_) => {
            let _ = crypto::derive_key(master_password, &DECOY_SALT);
            return Ok(false);
        }
    };
    let verifier_key =
        crypto::derive_key(master_password, &rec.verifier_salt).map_err(|_| AuthError::InvalidCredentials)?;
    Ok(crypto::verifier_matches(&verifier_key, &rec.verifier_hash))
}

pub fn login(
    registry_conn: &Connection,
    vault_path: &Path,
    username: &str,
    master_password: &str,
) -> Result<crypto::VaultKey, AuthError> {
    let rec = match registry::find_user(registry_conn, username) {
        Ok(rec) => rec,
        Err(_) => {
            let _ = crypto::derive_key(master_password, &DECOY_SALT);
            return Err(AuthError::InvalidCredentials);
        }
    };

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

    // rusqlite::Connection::open (inside open_vault) silently creates a new empty file when
    // the path doesn't exist (SQLite's default SQLITE_OPEN_CREATE behavior) - it never errors
    // for "file not found." Without this check, a correct password against a deleted/moved
    // vault file would silently succeed into a brand-new empty vault instead of failing,
    // masking data loss. The registry-based verifier/sentinel checks above only prove the
    // password is right; they say nothing about whether the vault file itself still exists.
    if !vault_path.exists() {
        return Err(AuthError::InvalidCredentials);
    }
    vault::open_vault(vault_path, &vault_key).map_err(|_| AuthError::InvalidCredentials)?;

    Ok(vault_key)
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

        let rec = registry::find_user(&registry_conn, "alice").unwrap();
        // The whole point of deriving vault_key and verifier_key from separate salts is that
        // neither can be reconstructed from the other - make that property test-visible, not
        // just code-inspection-visible.
        assert_ne!(rec.kdf_salt, rec.verifier_salt);
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

    #[test]
    fn verify_master_password_accepts_the_correct_password() {
        let dir = tempdir().unwrap();
        let registry_conn = registry::open_registry(&dir.path().join("registry.db")).unwrap();
        let vault_path = dir.path().join("alice.vault");
        register_user(&registry_conn, &vault_path, "alice", "correct horse battery staple").unwrap();

        let ok = verify_master_password(&registry_conn, "alice", "correct horse battery staple").unwrap();

        assert!(ok);
    }

    #[test]
    fn verify_master_password_rejects_a_wrong_password() {
        let dir = tempdir().unwrap();
        let registry_conn = registry::open_registry(&dir.path().join("registry.db")).unwrap();
        let vault_path = dir.path().join("alice.vault");
        register_user(&registry_conn, &vault_path, "alice", "correct horse battery staple").unwrap();

        let ok = verify_master_password(&registry_conn, "alice", "wrong password").unwrap();

        assert!(!ok);
    }

    #[test]
    fn verify_master_password_rejects_an_unknown_username() {
        let dir = tempdir().unwrap();
        let registry_conn = registry::open_registry(&dir.path().join("registry.db")).unwrap();

        let ok = verify_master_password(&registry_conn, "nobody", "anything").unwrap();

        assert!(!ok);
    }

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

    #[test]
    fn login_with_unknown_username_pays_equivalent_kdf_cost_to_avoid_timing_oracle() {
        // Not a precise timing assertion (flaky by nature) - just a floor check that the
        // unknown-username path isn't a near-instant DB-lookup-only return. Argon2id at this
        // crate's params (64 MiB / 3 iterations / 4 lanes) costs tens of milliseconds minimum on
        // any real hardware; a fast-path bypass (the bug this test guards against) would return in
        // well under 1ms since it's just an indexed SQLite miss with no KDF call at all.
        let dir = tempdir().unwrap();
        let registry_conn = registry::open_registry(&dir.path().join("registry.db")).unwrap();
        let vault_path = dir.path().join("nobody.vault");

        let start = std::time::Instant::now();
        let _ = login(&registry_conn, &vault_path, "nobody", "any password");
        let elapsed = start.elapsed();

        assert!(
            elapsed.as_millis() >= 5,
            "unknown-username login returned in {:?} - looks like the decoy KDF call was skipped",
            elapsed
        );
    }

    #[test]
    fn login_with_correct_password_but_missing_vault_file_is_rejected() {
        let dir = tempdir().unwrap();
        let registry_conn = registry::open_registry(&dir.path().join("registry.db")).unwrap();
        let vault_path = dir.path().join("alice.vault");
        register_user(&registry_conn, &vault_path, "alice", "correct horse battery staple").unwrap();

        // Simulate the vault file being deleted/moved out from under the registry entry.
        std::fs::remove_file(&vault_path).unwrap();

        let result = login(&registry_conn, &vault_path, "alice", "correct horse battery staple");
        assert!(matches!(result, Err(AuthError::InvalidCredentials)));
    }
}
