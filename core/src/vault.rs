use crate::crypto::VaultKey;
use rusqlite::Connection;
use std::path::Path;
use thiserror::Error;
use zeroize::Zeroizing;

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
    let conn = Connection::open(path)?;
    let key_hex: Zeroizing<String> = Zeroizing::new(
        key.as_bytes().iter().map(|b| format!("{:02x}", b)).collect(),
    );
    conn.pragma_update(None, "key", format!("x'{}'", key_hex.as_str()))?;

    // SQLCipher only actually verifies the key once you touch the schema.
    conn.query_row("SELECT count(*) FROM sqlite_master", [], |row| row.get::<_, i64>(0))
        .map_err(|_| VaultError::UnlockFailed)?;

    // Not persisted in the file - SQLite disables FK enforcement per-connection by default,
    // so this must be reissued on every open, not just at schema-creation time.
    conn.pragma_update(None, "foreign_keys", "ON")?;

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
            strength_score REAL,
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

#[derive(Debug, Clone, PartialEq)]
pub struct Site {
    pub id: i64,
    pub name: String,
    pub url: Option<String>,
    pub notes: Option<String>,
}

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

    #[test]
    fn failed_wrong_key_attempt_does_not_prevent_correct_key_reopen() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.vault");
        let right_key = derive_key("correct password", &[6u8; 16]).unwrap();
        let wrong_key = derive_key("wrong password", &[6u8; 16]).unwrap();

        open_vault(&path, &right_key).unwrap();
        let _ = open_vault(&path, &wrong_key); // expected to fail, side effects are what we're checking

        let reopened = open_vault(&path, &right_key);
        assert!(reopened.is_ok());
    }

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
}
