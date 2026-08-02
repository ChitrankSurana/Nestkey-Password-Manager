use rusqlite::{params, Connection, OptionalExtension};
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
        );
        -- Deliberately its own table, not columns on `users`: PIN unlock is a local-device
        -- convenience, not part of the account itself, and backup.rs's bundle format encodes
        -- `UserRecord`'s fields individually rather than dumping tables - a separate table is
        -- excluded from backup/restore for free, matching Bitwarden's own PIN-doesn't-sync model.
        CREATE TABLE IF NOT EXISTS pin_unlock (
            username TEXT PRIMARY KEY REFERENCES users(username),
            pin_salt BLOB NOT NULL,
            wrapped_key_enc BLOB NOT NULL,
            wrapped_key_nonce BLOB NOT NULL
        );",
    )?;
    Ok(conn)
}

pub fn create_user(conn: &Connection, rec: &UserRecord) -> Result<(), RegistryError> {
    // Rely on the username PRIMARY KEY constraint to detect duplicates atomically, rather than a
    // separate SELECT-then-INSERT (found in Task 11's code-quality review pass): a check-then-insert
    // has a TOCTOU window where two callers on separate Connections to the same registry file could
    // both see "not found" and then both attempt the INSERT, with the loser surfacing a raw
    // rusqlite constraint-violation error instead of the clean UsernameTaken every other duplicate
    // path returns.
    let result = conn.execute(
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
    );
    match result {
        Ok(_) => Ok(()),
        Err(rusqlite::Error::SqliteFailure(e, _))
            if e.code == rusqlite::ErrorCode::ConstraintViolation =>
        {
            Err(RegistryError::UsernameTaken)
        }
        Err(other) => Err(RegistryError::Db(other)),
    }
}

pub fn upsert_user(conn: &Connection, rec: &UserRecord) -> Result<(), RegistryError> {
    conn.execute(
        "INSERT INTO users (username, kdf_salt, verifier_salt, verifier_hash, sentinel_ciphertext, sentinel_nonce)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(username) DO UPDATE SET
           kdf_salt = excluded.kdf_salt,
           verifier_salt = excluded.verifier_salt,
           verifier_hash = excluded.verifier_hash,
           sentinel_ciphertext = excluded.sentinel_ciphertext,
           sentinel_nonce = excluded.sentinel_nonce",
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

#[derive(Debug, Clone)]
pub struct PinRecord {
    pub username: String,
    pub pin_salt: Vec<u8>,
    pub wrapped_key_enc: Vec<u8>,
    pub wrapped_key_nonce: Vec<u8>,
}

pub fn set_pin(conn: &Connection, rec: &PinRecord) -> Result<(), RegistryError> {
    conn.execute(
        "INSERT INTO pin_unlock (username, pin_salt, wrapped_key_enc, wrapped_key_nonce)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(username) DO UPDATE SET
           pin_salt = excluded.pin_salt,
           wrapped_key_enc = excluded.wrapped_key_enc,
           wrapped_key_nonce = excluded.wrapped_key_nonce",
        params![rec.username, rec.pin_salt, rec.wrapped_key_enc, rec.wrapped_key_nonce],
    )?;
    Ok(())
}

pub fn find_pin(conn: &Connection, username: &str) -> Result<Option<PinRecord>, RegistryError> {
    conn.query_row(
        "SELECT username, pin_salt, wrapped_key_enc, wrapped_key_nonce FROM pin_unlock WHERE username = ?1",
        params![username],
        |row| {
            Ok(PinRecord {
                username: row.get(0)?,
                pin_salt: row.get(1)?,
                wrapped_key_enc: row.get(2)?,
                wrapped_key_nonce: row.get(3)?,
            })
        },
    )
    .optional()
    .map_err(RegistryError::from)
}

pub fn clear_pin(conn: &Connection, username: &str) -> Result<(), RegistryError> {
    conn.execute("DELETE FROM pin_unlock WHERE username = ?1", params![username])?;
    Ok(())
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

    #[test]
    fn upsert_user_inserts_when_username_is_new() {
        let dir = tempdir().unwrap();
        let conn = open_registry(&dir.path().join("registry.db")).unwrap();

        upsert_user(&conn, &sample_record("alice")).unwrap();

        let found = find_user(&conn, "alice").unwrap();
        assert_eq!(found.username, "alice");
    }

    #[test]
    fn upsert_user_replaces_existing_fields_without_erroring() {
        let dir = tempdir().unwrap();
        let conn = open_registry(&dir.path().join("registry.db")).unwrap();
        create_user(&conn, &sample_record("alice")).unwrap();

        let mut replacement = sample_record("alice");
        replacement.verifier_hash = vec![99; 32];
        upsert_user(&conn, &replacement).unwrap();

        let found = find_user(&conn, "alice").unwrap();
        assert_eq!(found.verifier_hash, vec![99; 32]);
        // Still exactly one row for "alice" - an upsert must not leave a duplicate behind.
        assert_eq!(list_usernames(&conn).unwrap(), vec!["alice".to_string()]);
    }

    fn sample_pin(username: &str) -> PinRecord {
        PinRecord {
            username: username.to_string(),
            pin_salt: vec![1, 2, 3, 4],
            wrapped_key_enc: vec![5, 6, 7, 8],
            wrapped_key_nonce: vec![9; 12],
        }
    }

    #[test]
    fn find_pin_returns_none_when_no_pin_is_set() {
        let dir = tempdir().unwrap();
        let conn = open_registry(&dir.path().join("registry.db")).unwrap();
        create_user(&conn, &sample_record("alice")).unwrap();

        assert!(find_pin(&conn, "alice").unwrap().is_none());
    }

    #[test]
    fn set_pin_then_find_pin_roundtrips() {
        let dir = tempdir().unwrap();
        let conn = open_registry(&dir.path().join("registry.db")).unwrap();
        create_user(&conn, &sample_record("alice")).unwrap();

        set_pin(&conn, &sample_pin("alice")).unwrap();

        let found = find_pin(&conn, "alice").unwrap().unwrap();
        assert_eq!(found.pin_salt, vec![1, 2, 3, 4]);
        assert_eq!(found.wrapped_key_enc, vec![5, 6, 7, 8]);
    }

    #[test]
    fn set_pin_twice_replaces_rather_than_duplicates() {
        let dir = tempdir().unwrap();
        let conn = open_registry(&dir.path().join("registry.db")).unwrap();
        create_user(&conn, &sample_record("alice")).unwrap();
        set_pin(&conn, &sample_pin("alice")).unwrap();

        let mut replacement = sample_pin("alice");
        replacement.wrapped_key_enc = vec![99; 8];
        set_pin(&conn, &replacement).unwrap();

        let found = find_pin(&conn, "alice").unwrap().unwrap();
        assert_eq!(found.wrapped_key_enc, vec![99; 8]);
    }

    #[test]
    fn clear_pin_removes_it() {
        let dir = tempdir().unwrap();
        let conn = open_registry(&dir.path().join("registry.db")).unwrap();
        create_user(&conn, &sample_record("alice")).unwrap();
        set_pin(&conn, &sample_pin("alice")).unwrap();

        clear_pin(&conn, "alice").unwrap();

        assert!(find_pin(&conn, "alice").unwrap().is_none());
    }
}
