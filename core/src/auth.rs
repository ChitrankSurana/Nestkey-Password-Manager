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
    // Windows filenames ignore case, so "Alice" would open alice's existing vault file.
    let lowered = username.to_lowercase();
    if registry::list_usernames(registry_conn)?.iter().any(|u| u.to_lowercase() == lowered) {
        return Err(AuthError::UsernameTaken);
    }

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

/// What `change_master_password` derives before anything is written: the still-valid old vault
/// key (needed to decrypt every sealed blob so it can be re-sealed), the new vault key, and the
/// new registry record. Kept as one struct so `commands.rs` can compute all of this from a single
/// `registry` lock, drop it, then acquire `vault` separately - the same "never hold both locks at
/// once" discipline every other function in `commands.rs` already follows (see its own comment on
/// `do_set_pin`, the one deliberate exception, which this is not).
pub struct PasswordChangeMaterial {
    pub old_vault_key: crypto::VaultKey,
    pub new_vault_key: crypto::VaultKey,
    pub new_user_record: registry::UserRecord,
}

/// B1-6, step 1 of 3 (registry-only, no vault access yet): verifies `current_password`, then
/// derives everything a password change needs under a *new* kdf/verifier salt pair - exactly the
/// same salt/verifier/sentinel shape `register_user` creates for a brand-new account, since a
/// password change is, from the registry's point of view, indistinguishable from re-registering
/// with new salts. Does not touch the vault or write anything - purely preparatory.
pub fn prepare_password_change(
    registry_conn: &Connection,
    username: &str,
    current_password: &str,
    new_password: &str,
) -> Result<PasswordChangeMaterial, AuthError> {
    let rec = registry::find_user(registry_conn, username)?;

    let verifier_key = crypto::derive_key(current_password, &rec.verifier_salt)
        .map_err(|_| AuthError::InvalidCredentials)?;
    if !crypto::verifier_matches(&verifier_key, &rec.verifier_hash) {
        return Err(AuthError::InvalidCredentials);
    }
    let old_vault_key =
        crypto::derive_key(current_password, &rec.kdf_salt).map_err(|_| AuthError::InvalidCredentials)?;

    let new_kdf_salt = crypto::random_salt();
    let new_verifier_salt = crypto::random_salt();
    let new_vault_key = crypto::derive_key(new_password, &new_kdf_salt)?;
    let new_verifier_key = crypto::derive_key(new_password, &new_verifier_salt)?;
    let (sentinel_ciphertext, sentinel_nonce) = crypto::seal(&new_vault_key, SENTINEL_PLAINTEXT)?;

    Ok(PasswordChangeMaterial {
        old_vault_key,
        new_vault_key,
        new_user_record: registry::UserRecord {
            username: username.to_string(),
            kdf_salt: new_kdf_salt.to_vec(),
            verifier_salt: new_verifier_salt.to_vec(),
            verifier_hash: new_verifier_key.as_bytes().to_vec(),
            sentinel_ciphertext,
            sentinel_nonce: sentinel_nonce.to_vec(),
        },
    })
}

/// B1-6, step 3 of 3 (registry-only, after the vault has already been re-sealed and re-keyed
/// under `material.new_vault_key` - see `vault::reseal_all_blobs`/`vault::rekey_vault`): commits
/// the new salts/verifier/sentinel and clears PIN + biometric, both of which wrap the *old* vault
/// key and are now unusable. Ordering this last means an interruption anywhere before this call
/// leaves the registry untouched - the old password still logs in, and restoring the pre-flight
/// vault backup (the caller's responsibility, taken before any of this starts) makes the vault
/// file consistent with it again.
pub fn commit_password_change(
    registry_conn: &Connection,
    username: &str,
    material: &PasswordChangeMaterial,
) -> Result<(), AuthError> {
    registry::upsert_user(registry_conn, &material.new_user_record)?;
    registry::clear_pin(registry_conn, username)?;
    registry::clear_biometric(registry_conn, username)?;
    Ok(())
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

/// Wraps the already-unlocked vault key under a PIN-derived key, so a later `unlock_with_pin`
/// can recover it directly - a faster local-only unlock alternative to the master password,
/// never the master password itself.
pub fn set_pin(
    registry_conn: &Connection,
    username: &str,
    pin: &str,
    vault_key: &crypto::VaultKey,
) -> Result<(), AuthError> {
    let pin_salt = crypto::random_salt();
    let pin_kek = crypto::derive_key(pin, &pin_salt)?;
    let (wrapped_key_enc, wrapped_key_nonce) = crypto::seal(&pin_kek, vault_key.as_bytes())?;
    registry::set_pin(
        registry_conn,
        &registry::PinRecord {
            username: username.to_string(),
            pin_salt: pin_salt.to_vec(),
            wrapped_key_enc,
            wrapped_key_nonce: wrapped_key_nonce.to_vec(),
        },
    )?;
    Ok(())
}

pub fn clear_pin(registry_conn: &Connection, username: &str) -> Result<(), AuthError> {
    registry::clear_pin(registry_conn, username)?;
    Ok(())
}

pub fn has_pin(registry_conn: &Connection, username: &str) -> Result<bool, AuthError> {
    Ok(registry::find_pin(registry_conn, username)?.is_some())
}

/// Unwraps the vault key using a PIN instead of the master password - same decoy-derivation
/// timing defense as `login` (unknown username / wrong PIN take the same wall-clock time).
pub fn unlock_with_pin(
    registry_conn: &Connection,
    username: &str,
    pin: &str,
) -> Result<crypto::VaultKey, AuthError> {
    let rec = match registry::find_pin(registry_conn, username)? {
        Some(rec) => rec,
        None => {
            let _ = crypto::derive_key(pin, &DECOY_SALT);
            return Err(AuthError::InvalidCredentials);
        }
    };
    let pin_kek = crypto::derive_key(pin, &rec.pin_salt).map_err(|_| AuthError::InvalidCredentials)?;
    if rec.wrapped_key_nonce.len() != 12 {
        return Err(AuthError::InvalidCredentials);
    }
    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&rec.wrapped_key_nonce);
    let raw = zeroize::Zeroizing::new(
        crypto::open(&pin_kek, &rec.wrapped_key_enc, &nonce).map_err(|_| AuthError::InvalidCredentials)?,
    );
    if raw.len() != 32 {
        return Err(AuthError::InvalidCredentials);
    }
    let mut key_bytes = [0u8; 32];
    key_bytes.copy_from_slice(&raw);
    Ok(crypto::VaultKey::from_bytes(key_bytes))
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
    fn register_user_rejects_username_differing_only_by_case() {
        let dir = tempdir().unwrap();
        let registry_conn = registry::open_registry(&dir.path().join("registry.db")).unwrap();

        register_user(&registry_conn, &dir.path().join("alice.vault"), "alice", "password one").unwrap();
        let result = register_user(&registry_conn, &dir.path().join("Alice2.vault"), "Alice", "password two");

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

    #[test]
    fn has_pin_is_false_until_set_pin_is_called() {
        let dir = tempdir().unwrap();
        let registry_conn = registry::open_registry(&dir.path().join("registry.db")).unwrap();
        let vault_path = dir.path().join("alice.vault");
        let vault_key =
            register_user(&registry_conn, &vault_path, "alice", "correct horse battery staple").unwrap();

        assert!(!has_pin(&registry_conn, "alice").unwrap());

        set_pin(&registry_conn, "alice", "1234", &vault_key).unwrap();

        assert!(has_pin(&registry_conn, "alice").unwrap());
    }

    #[test]
    fn unlock_with_pin_recovers_the_exact_same_vault_key() {
        let dir = tempdir().unwrap();
        let registry_conn = registry::open_registry(&dir.path().join("registry.db")).unwrap();
        let vault_path = dir.path().join("alice.vault");
        let vault_key =
            register_user(&registry_conn, &vault_path, "alice", "correct horse battery staple").unwrap();
        set_pin(&registry_conn, "alice", "1234", &vault_key).unwrap();

        let recovered = unlock_with_pin(&registry_conn, "alice", "1234").unwrap();

        assert_eq!(recovered.as_bytes(), vault_key.as_bytes());
    }

    #[test]
    fn unlock_with_pin_rejects_a_wrong_pin() {
        let dir = tempdir().unwrap();
        let registry_conn = registry::open_registry(&dir.path().join("registry.db")).unwrap();
        let vault_path = dir.path().join("alice.vault");
        let vault_key =
            register_user(&registry_conn, &vault_path, "alice", "correct horse battery staple").unwrap();
        set_pin(&registry_conn, "alice", "1234", &vault_key).unwrap();

        let result = unlock_with_pin(&registry_conn, "alice", "0000");

        assert!(matches!(result, Err(AuthError::InvalidCredentials)));
    }

    #[test]
    fn unlock_with_pin_rejects_when_no_pin_is_configured() {
        let dir = tempdir().unwrap();
        let registry_conn = registry::open_registry(&dir.path().join("registry.db")).unwrap();
        let vault_path = dir.path().join("alice.vault");
        register_user(&registry_conn, &vault_path, "alice", "correct horse battery staple").unwrap();

        let result = unlock_with_pin(&registry_conn, "alice", "1234");

        assert!(matches!(result, Err(AuthError::InvalidCredentials)));
    }

    #[test]
    fn clear_pin_makes_unlock_with_pin_fail_again() {
        let dir = tempdir().unwrap();
        let registry_conn = registry::open_registry(&dir.path().join("registry.db")).unwrap();
        let vault_path = dir.path().join("alice.vault");
        let vault_key =
            register_user(&registry_conn, &vault_path, "alice", "correct horse battery staple").unwrap();
        set_pin(&registry_conn, "alice", "1234", &vault_key).unwrap();

        clear_pin(&registry_conn, "alice").unwrap();

        assert!(!has_pin(&registry_conn, "alice").unwrap());
        assert!(matches!(
            unlock_with_pin(&registry_conn, "alice", "1234"),
            Err(AuthError::InvalidCredentials)
        ));
    }

    #[test]
    fn prepare_password_change_rejects_the_wrong_current_password() {
        let dir = tempdir().unwrap();
        let registry_conn = registry::open_registry(&dir.path().join("registry.db")).unwrap();
        let vault_path = dir.path().join("alice.vault");
        register_user(&registry_conn, &vault_path, "alice", "correct horse battery staple").unwrap();

        let result = prepare_password_change(&registry_conn, "alice", "wrong password", "new password");

        assert!(matches!(result, Err(AuthError::InvalidCredentials)));
    }

    #[test]
    fn change_master_password_end_to_end_makes_the_new_password_the_only_way_in() {
        let dir = tempdir().unwrap();
        let registry_conn = registry::open_registry(&dir.path().join("registry.db")).unwrap();
        let vault_path = dir.path().join("alice.vault");
        let vault_key =
            register_user(&registry_conn, &vault_path, "alice", "correct horse battery staple").unwrap();
        let vault_conn = vault::open_vault(&vault_path, &vault_key).unwrap();
        let site = vault::add_site(&vault_conn, "example.com", None, None).unwrap();
        vault::add_credential(&vault_conn, &vault_key, site, Some("alice"), "s3cret", None, &[], false).unwrap();
        set_pin(&registry_conn, "alice", "1234", &vault_key).unwrap();
        assert!(has_pin(&registry_conn, "alice").unwrap());

        // Mirrors do_change_master_password's exact three-step orchestration (registry, then
        // vault, then registry again - see PasswordChangeMaterial's own doc comment).
        let material =
            prepare_password_change(&registry_conn, "alice", "correct horse battery staple", "new correct horse").unwrap();
        vault::reseal_all_blobs(&vault_conn, &material.old_vault_key, &material.new_vault_key).unwrap();
        vault::rekey_vault(&vault_conn, &material.new_vault_key).unwrap();
        commit_password_change(&registry_conn, "alice", &material).unwrap();
        drop(vault_conn);

        // New password logs in and reveals the same data.
        let new_key = login(&registry_conn, &vault_path, "alice", "new correct horse").unwrap();
        let reopened = vault::open_vault(&vault_path, &new_key).unwrap();
        let creds = vault::list_credentials_for_site(&reopened, &new_key, site).unwrap();
        assert_eq!(creds[0].secret, "s3cret");

        // Old password no longer works anywhere - not the registry verifier, not the vault file.
        assert!(matches!(
            login(&registry_conn, &vault_path, "alice", "correct horse battery staple"),
            Err(AuthError::InvalidCredentials)
        ));

        // PIN wrapped the old key - it must be gone, not silently broken.
        assert!(!has_pin(&registry_conn, "alice").unwrap());
        assert!(matches!(
            unlock_with_pin(&registry_conn, "alice", "1234"),
            Err(AuthError::InvalidCredentials)
        ));
    }
}
