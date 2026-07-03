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
