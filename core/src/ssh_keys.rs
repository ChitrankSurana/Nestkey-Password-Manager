use crate::crypto::VaultKey;
use crate::vault::VaultError;
use rusqlite::Connection;

pub struct SshKeyEntry {
    pub id: i64,
    pub label: String,
    pub private_key_openssh: String,
}

// Manual Debug impl (not derived) so a stray `{:?}`/`dbg!()` can never print the plaintext private
// key - same rationale and pattern as vault::Credential.
impl std::fmt::Debug for SshKeyEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SshKeyEntry")
            .field("id", &self.id)
            .field("label", &self.label)
            .field("private_key_openssh", &"<redacted>")
            .finish()
    }
}

/// No trash/soft-delete for SSH keys (unlike credentials) - a smaller surface than the plan
/// called for, since nothing else here depends on recovering a deleted key, and a hard delete
/// keeps this module tiny; add it if that turns out to matter.
pub fn add_ssh_key(
    conn: &Connection,
    key: &VaultKey,
    label: &str,
    private_key_openssh: &str,
) -> Result<i64, VaultError> {
    let (enc, nonce) = crate::crypto::seal(key, private_key_openssh.as_bytes())?;
    conn.execute(
        "INSERT INTO ssh_keys (label, private_key_enc, private_key_nonce) VALUES (?1, ?2, ?3)",
        rusqlite::params![label, enc, nonce.to_vec()],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn list_ssh_keys(conn: &Connection, key: &VaultKey) -> Result<Vec<SshKeyEntry>, VaultError> {
    let mut stmt = conn.prepare("SELECT id, label, private_key_enc, private_key_nonce FROM ssh_keys ORDER BY id")?;
    let rows = stmt.query_map([], |row| {
        let id: i64 = row.get(0)?;
        let label: String = row.get(1)?;
        let enc: Vec<u8> = row.get(2)?;
        let nonce: Vec<u8> = row.get(3)?;
        Ok((id, label, enc, nonce))
    })?;

    let mut out = Vec::new();
    for r in rows {
        let (id, label, enc, nonce_bytes) = r?;
        if nonce_bytes.len() != 12 {
            return Err(VaultError::UnlockFailed);
        }
        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(&nonce_bytes);
        let plaintext = crate::crypto::open(key, &enc, &nonce)?;
        let private_key_openssh = String::from_utf8(plaintext).map_err(|_| VaultError::UnlockFailed)?;
        out.push(SshKeyEntry { id, label, private_key_openssh });
    }
    Ok(out)
}

pub fn delete_ssh_key(conn: &Connection, id: i64) -> Result<(), VaultError> {
    let updated = conn.execute("DELETE FROM ssh_keys WHERE id = ?1", rusqlite::params![id])?;
    if updated == 0 {
        return Err(VaultError::NotFound);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::derive_key;
    use crate::vault::open_vault;
    use ssh_key::{Algorithm, PrivateKey};
    use tempfile::tempdir;

    fn sample_openssh_key() -> String {
        let key = PrivateKey::random(&mut rand::rngs::OsRng, Algorithm::Ed25519).unwrap();
        key.to_openssh(ssh_key::LineEnding::LF).unwrap().to_string()
    }

    #[test]
    fn add_then_list_decrypts_the_private_key() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[1u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();
        let openssh = sample_openssh_key();

        add_ssh_key(&conn, &key, "my laptop key", &openssh).unwrap();

        let keys = list_ssh_keys(&conn, &key).unwrap();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].label, "my laptop key");
        assert_eq!(keys[0].private_key_openssh, openssh);
    }

    #[test]
    fn list_ssh_keys_returns_every_stored_key_in_insertion_order() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[2u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();
        add_ssh_key(&conn, &key, "first", &sample_openssh_key()).unwrap();
        add_ssh_key(&conn, &key, "second", &sample_openssh_key()).unwrap();

        let keys = list_ssh_keys(&conn, &key).unwrap();

        assert_eq!(keys.len(), 2);
        assert_eq!(keys[0].label, "first");
        assert_eq!(keys[1].label, "second");
    }

    #[test]
    fn delete_ssh_key_removes_it() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[3u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();
        let id = add_ssh_key(&conn, &key, "temp", &sample_openssh_key()).unwrap();

        delete_ssh_key(&conn, id).unwrap();

        assert!(list_ssh_keys(&conn, &key).unwrap().is_empty());
    }

    #[test]
    fn delete_ssh_key_on_unknown_id_returns_not_found() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[4u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();

        let result = delete_ssh_key(&conn, 9999);

        assert!(matches!(result, Err(VaultError::NotFound)));
    }
}
