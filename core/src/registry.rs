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
