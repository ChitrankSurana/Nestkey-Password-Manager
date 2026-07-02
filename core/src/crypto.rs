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
    let params = Params::new(65536, 3, 4, Some(32)).map_err(|_| CryptoError::Kdf)?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut out = [0u8; 32];
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut out)
        .map_err(|_| CryptoError::Kdf)?;
    Ok(VaultKey(out))
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
