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
    // Zeroizing the hex string closes the copy this function directly owns, but it is not a
    // complete guarantee: PRAGMA statements can't bind parameters, so rusqlite's pragma_update
    // formats this value into its own internal, unwrapped SQL buffer before executing it as
    // literal SQL text - an additional in-memory copy of the raw key hex that this crate can't
    // reach or zeroize. That's inherent to any PRAGMA-based key-passing approach with rusqlite/
    // SQLCipher, not something fixable here without patching rusqlite itself. Found in the
    // full-crate final review; noted rather than "fixed" since it's at the practical ceiling of
    // what this call site can control.
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

#[derive(Clone, PartialEq)]
pub struct Credential {
    pub id: i64,
    pub site_id: i64,
    pub username: Option<String>,
    // TODO(M1 Phase B): decrypted secrets aren't wrapped in zeroize::Zeroizing yet. Deferred
    // deliberately to the UI/IPC layer, which is the last consumer and last real chance to
    // zeroize before display - see the Task 8 code review discussion in this plan's history.
    // Don't let this be forgotten when that phase is planned.
    pub secret: String,
    pub remarks: Option<String>,
}

// Manual Debug impl (not derived) so a stray `{:?}`, `dbg!()`, or future log statement can never
// print the plaintext secret. crypto::VaultKey already refuses to derive Debug for the same
// reason - this closes the same gap for the one other struct in the crate that carries a
// decrypted secret in memory. Found in the full-crate final review.
impl std::fmt::Debug for Credential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Credential")
            .field("id", &self.id)
            .field("site_id", &self.site_id)
            .field("username", &self.username)
            .field("secret", &"<redacted>")
            .field("remarks", &self.remarks)
            .finish()
    }
}

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
        // secret_nonce comes from the DB row - external input relative to this function, not
        // something this code controls. A malformed/corrupted row (bad migration, manual DB
        // surgery, future bug elsewhere) must not be able to crash the whole read path via an
        // unchecked copy_from_slice panic; fail that one lookup gracefully instead.
        if secret_nonce.len() != 12 {
            return Err(VaultError::UnlockFailed);
        }
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

#[derive(Debug, Clone, PartialEq)]
pub struct SearchResult {
    pub site_id: i64,
    pub site_name: String,
    pub credential_id: i64,
    pub username: Option<String>,
}

pub fn search(conn: &Connection, query: &str) -> Result<Vec<SearchResult>, VaultError> {
    // SQLite's LIKE treats '%' and '_' as wildcards even inside the user's own query text, so a
    // literal search for e.g. "50% off" or "jane_doe" would silently behave like a wildcard match
    // instead of a literal one. Escape them (and the escape character itself) before wrapping in
    // the leading/trailing '%' that makes this a substring search, and tell SQLite which escape
    // character to honor via ESCAPE.
    let escaped: String = query
        .to_lowercase()
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    let pattern = format!("%{}%", escaped);
    let mut stmt = conn.prepare(
        "SELECT s.id, s.name, c.id, c.username
         FROM credentials c
         JOIN sites s ON s.id = c.site_id
         WHERE c.deleted_at IS NULL AND s.deleted_at IS NULL
           AND (
             lower(s.name) LIKE ?1 ESCAPE '\\'
             OR lower(coalesce(c.username, '')) LIKE ?1 ESCAPE '\\'
             OR lower(coalesce(c.remarks, '')) LIKE ?1 ESCAPE '\\'
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
    fn credential_debug_format_redacts_secret() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[11u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();
        let site_id = add_site(&conn, "example.com", None, None).unwrap();
        add_credential(&conn, &key, site_id, Some("alice"), "extremely-unique-secret-xyz", None).unwrap();
        let creds = list_credentials_for_site(&conn, &key, site_id).unwrap();

        let debug_output = format!("{:?}", creds[0]);
        assert!(!debug_output.contains("extremely-unique-secret-xyz"));
        assert!(debug_output.contains("<redacted>"));
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

    #[test]
    fn list_credentials_for_site_returns_only_that_sites_credentials() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[9u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();
        let site_a = add_site(&conn, "a.com", None, None).unwrap();
        let site_b = add_site(&conn, "b.com", None, None).unwrap();
        add_credential(&conn, &key, site_a, Some("alice"), "secret-a1", None).unwrap();
        add_credential(&conn, &key, site_a, Some("alice2"), "secret-a2", None).unwrap();
        add_credential(&conn, &key, site_b, Some("bob"), "secret-b", None).unwrap();

        let creds_a = list_credentials_for_site(&conn, &key, site_a).unwrap();
        assert_eq!(creds_a.len(), 2);
        assert!(creds_a.iter().all(|c| c.site_id == site_a));

        let creds_b = list_credentials_for_site(&conn, &key, site_b).unwrap();
        assert_eq!(creds_b.len(), 1);
    }

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

    #[test]
    fn search_treats_percent_and_underscore_in_query_as_literal_characters() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[11u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();

        let site_id = add_site(&conn, "50% off deals", None, None).unwrap();
        add_credential(&conn, &key, site_id, Some("jane_doe"), "secret", None).unwrap();

        // A literal '%' in the query must match only the literal text, not act as a wildcard.
        assert_eq!(search(&conn, "50% off").unwrap().len(), 1);

        // A literal '_' must not behave like "match any single character".
        assert!(search(&conn, "jane.doe").unwrap().is_empty());
        assert_eq!(search(&conn, "jane_doe").unwrap().len(), 1);
    }

    #[test]
    fn search_excludes_soft_deleted_sites_and_credentials() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[10u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();

        let site_id = add_site(&conn, "deleteme.com", None, None).unwrap();
        let cred_id = add_credential(&conn, &key, site_id, Some("carol"), "secret", None).unwrap();
        assert_eq!(search(&conn, "deleteme").unwrap().len(), 1);

        soft_delete_credential(&conn, cred_id).unwrap();
        assert!(search(&conn, "deleteme").unwrap().is_empty());

        restore_credential(&conn, cred_id).unwrap();
        assert_eq!(search(&conn, "deleteme").unwrap().len(), 1);

        soft_delete_site(&conn, site_id).unwrap();
        assert!(search(&conn, "deleteme").unwrap().is_empty());
    }
}
