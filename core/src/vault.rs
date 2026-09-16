use crate::crypto::VaultKey;
use crate::portio::ImportRecord;
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
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
    #[error("import record is missing a required field: {0}")]
    InvalidImportRecord(String),
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
        CREATE TABLE IF NOT EXISTS ssh_keys (
            id INTEGER PRIMARY KEY,
            label TEXT NOT NULL,
            private_key_enc BLOB NOT NULL,
            private_key_nonce BLOB NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_sites_name ON sites(name);
        CREATE INDEX IF NOT EXISTS idx_credentials_site ON credentials(site_id);",
    )?;
    add_credential_columns_if_missing(conn)?;
    Ok(())
}

// `CREATE TABLE IF NOT EXISTS` above is a no-op against a vault file created before these columns
// existed, so a real existing vault (not just fresh installs) needs them added explicitly or every
// later INSERT/SELECT referencing them would fail with "no such column." No migrations table
// exists in this schema yet - table_info + ADD COLUMN is the whole mechanism, and is itself
// idempotent (checks before adding), so this runs unconditionally on every open.
fn add_credential_columns_if_missing(conn: &Connection) -> Result<(), VaultError> {
    let mut stmt = conn.prepare("PRAGMA table_info(credentials)")?;
    let existing: Vec<String> = stmt.query_map([], |row| row.get::<_, String>(1))?.collect::<Result<_, _>>()?;
    drop(stmt);
    if !existing.iter().any(|c| c == "reprompt") {
        conn.execute("ALTER TABLE credentials ADD COLUMN reprompt INTEGER NOT NULL DEFAULT 0", [])?;
    }
    if !existing.iter().any(|c| c == "custom_fields_enc") {
        conn.execute("ALTER TABLE credentials ADD COLUMN custom_fields_enc BLOB", [])?;
        conn.execute("ALTER TABLE credentials ADD COLUMN custom_fields_nonce BLOB", [])?;
    }
    if !existing.iter().any(|c| c == "totp_secret_enc") {
        conn.execute("ALTER TABLE credentials ADD COLUMN totp_secret_enc BLOB", [])?;
        conn.execute("ALTER TABLE credentials ADD COLUMN totp_secret_nonce BLOB", [])?;
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CustomFieldType {
    Text,
    Hidden,
    Boolean,
}

// Not `#[derive(Debug)]`'d as part of `Credential`'s manual redacting impl below - a `Hidden`
// field's value is exactly as sensitive as the main secret, so it gets the same treatment.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomField {
    pub label: String,
    pub value: String,
    // The alias below keeps legacy encrypted blobs (sealed by any build predating this rename)
    // readable: writes always emit `fieldType`, but reads accept either spelling.
    #[serde(alias = "field_type")]
    pub field_type: CustomFieldType,
}

/// Serializes and encrypts a credential's custom fields as one blob, the same way the main secret
/// is sealed - simplest way to keep `Hidden`-type values off the plaintext page, without the
/// complexity of per-field encryption for a personal-vault-scale field list.
fn seal_custom_fields(key: &VaultKey, fields: &[CustomField]) -> Result<(Vec<u8>, [u8; 12]), VaultError> {
    let json = serde_json::to_vec(fields).map_err(|_| VaultError::UnlockFailed)?;
    Ok(crate::crypto::seal(key, &json)?)
}

fn open_custom_fields(
    key: &VaultKey,
    enc: Option<Vec<u8>>,
    nonce: Option<Vec<u8>>,
) -> Result<Vec<CustomField>, VaultError> {
    let (enc, nonce_bytes) = match (enc, nonce) {
        (Some(e), Some(n)) => (e, n),
        _ => return Ok(Vec::new()), // rows saved before this feature existed have neither column set
    };
    if nonce_bytes.len() != 12 {
        return Err(VaultError::UnlockFailed);
    }
    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&nonce_bytes);
    let json = crate::crypto::open(key, &enc, &nonce)?;
    serde_json::from_slice(&json).map_err(|_| VaultError::UnlockFailed)
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

pub fn update_site(
    conn: &Connection,
    site_id: i64,
    name: &str,
    url: Option<&str>,
    notes: Option<&str>,
) -> Result<(), VaultError> {
    let updated = conn.execute(
        "UPDATE sites SET name = ?1, url = ?2, notes = ?3, updated_at = datetime('now')
         WHERE id = ?4 AND deleted_at IS NULL",
        rusqlite::params![name, url, notes, site_id],
    )?;
    if updated == 0 {
        return Err(VaultError::NotFound);
    }
    Ok(())
}

// Cheap existence check for the "refuse to delete a non-empty site" business rule at the command
// layer - deliberately not a decrypt (unlike list_credentials_for_site), since this only needs a
// count, not the secrets.
pub fn count_live_credentials_for_site(conn: &Connection, site_id: i64) -> Result<i64, VaultError> {
    Ok(conn.query_row(
        "SELECT count(*) FROM credentials WHERE site_id = ?1 AND deleted_at IS NULL",
        rusqlite::params![site_id],
        |row| row.get(0),
    )?)
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
    pub custom_fields: Vec<CustomField>,
    pub reprompt: bool,
    // Presence only - the TOTP seed itself is never eagerly decrypted for a listing, only on
    // demand by `get_totp_secret` (used solely to compute the current code).
    pub has_totp: bool,
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
            // A Hidden-type value here is exactly as sensitive as `secret` above - count only.
            .field("custom_fields", &format!("<{} field(s)>", self.custom_fields.len()))
            .field("reprompt", &self.reprompt)
            .field("has_totp", &self.has_totp)
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
    custom_fields: &[CustomField],
    reprompt: bool,
) -> Result<i64, VaultError> {
    let (secret_enc, nonce) = crate::crypto::seal(key, secret.as_bytes())?;
    let (fields_enc, fields_nonce) = seal_custom_fields(key, custom_fields)?;
    conn.execute(
        "INSERT INTO credentials
            (site_id, username, secret_enc, secret_nonce, remarks, custom_fields_enc, custom_fields_nonce, reprompt)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            site_id, username, secret_enc, nonce.to_vec(), remarks,
            fields_enc, fields_nonce.to_vec(), reprompt,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn list_credentials_for_site(
    conn: &Connection,
    key: &VaultKey,
    site_id: i64,
) -> Result<Vec<Credential>, VaultError> {
    let mut stmt = conn.prepare(
        "SELECT c.id, c.site_id, c.username, c.secret_enc, c.secret_nonce, c.remarks,
                c.custom_fields_enc, c.custom_fields_nonce, c.reprompt, (c.totp_secret_enc IS NOT NULL)
         FROM credentials c
         JOIN sites s ON s.id = c.site_id
         WHERE c.site_id = ?1 AND c.deleted_at IS NULL AND s.deleted_at IS NULL
         ORDER BY c.id",
    )?;
    let rows = stmt.query_map(rusqlite::params![site_id], |row| {
        let id: i64 = row.get(0)?;
        let site_id: i64 = row.get(1)?;
        let username: Option<String> = row.get(2)?;
        let secret_enc: Vec<u8> = row.get(3)?;
        let secret_nonce: Vec<u8> = row.get(4)?;
        let remarks: Option<String> = row.get(5)?;
        let fields_enc: Option<Vec<u8>> = row.get(6)?;
        let fields_nonce: Option<Vec<u8>> = row.get(7)?;
        let reprompt: bool = row.get(8)?;
        let has_totp: bool = row.get(9)?;
        Ok((id, site_id, username, secret_enc, secret_nonce, remarks, fields_enc, fields_nonce, reprompt, has_totp))
    })?;

    let mut out = Vec::new();
    for r in rows {
        let (id, site_id, username, secret_enc, secret_nonce, remarks, fields_enc, fields_nonce, reprompt, has_totp) = r?;
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
        let custom_fields = open_custom_fields(key, fields_enc, fields_nonce)?;
        out.push(Credential { id, site_id, username, secret, remarks, custom_fields, reprompt, has_totp });
    }
    Ok(out)
}

/// Sets (or replaces) a credential's TOTP seed - a dedicated command rather than a param on
/// `add_credential`/`update_credential` so those two never risk silently wiping an existing TOTP
/// seed on an unrelated field edit: their UPDATE statements simply never touch these two columns.
pub fn set_totp_secret(
    conn: &Connection,
    key: &VaultKey,
    credential_id: i64,
    secret: &str,
) -> Result<(), VaultError> {
    let (enc, nonce) = crate::crypto::seal(key, secret.as_bytes())?;
    let updated = conn.execute(
        "UPDATE credentials SET totp_secret_enc = ?1, totp_secret_nonce = ?2, updated_at = datetime('now')
         WHERE id = ?3 AND deleted_at IS NULL",
        rusqlite::params![enc, nonce.to_vec(), credential_id],
    )?;
    if updated == 0 {
        return Err(VaultError::NotFound);
    }
    Ok(())
}

pub fn clear_totp_secret(conn: &Connection, credential_id: i64) -> Result<(), VaultError> {
    let updated = conn.execute(
        "UPDATE credentials SET totp_secret_enc = NULL, totp_secret_nonce = NULL, updated_at = datetime('now')
         WHERE id = ?1 AND deleted_at IS NULL",
        rusqlite::params![credential_id],
    )?;
    if updated == 0 {
        return Err(VaultError::NotFound);
    }
    Ok(())
}

/// Decrypts a credential's TOTP seed on demand - the only reader is the current-code computation,
/// polled roughly once a second while a TOTP row is expanded, so this is a targeted single-row
/// fetch rather than something `list_credentials_for_site` decrypts eagerly for every row.
pub fn get_totp_secret(
    conn: &Connection,
    key: &VaultKey,
    credential_id: i64,
) -> Result<Option<String>, VaultError> {
    let row: Option<(Option<Vec<u8>>, Option<Vec<u8>>)> = conn
        .query_row(
            "SELECT totp_secret_enc, totp_secret_nonce FROM credentials WHERE id = ?1 AND deleted_at IS NULL",
            rusqlite::params![credential_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let (enc, nonce_bytes) = match row.ok_or(VaultError::NotFound)? {
        (Some(e), Some(n)) => (e, n),
        _ => return Ok(None),
    };
    if nonce_bytes.len() != 12 {
        return Err(VaultError::UnlockFailed);
    }
    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&nonce_bytes);
    let plaintext = crate::crypto::open(key, &enc, &nonce)?;
    Ok(Some(String::from_utf8(plaintext).map_err(|_| VaultError::UnlockFailed)?))
}

pub fn update_credential(
    conn: &Connection,
    key: &VaultKey,
    credential_id: i64,
    username: Option<&str>,
    secret: &str,
    remarks: Option<&str>,
    custom_fields: &[CustomField],
    reprompt: bool,
) -> Result<(), VaultError> {
    let existing: Option<(Vec<u8>, Vec<u8>)> = conn
        .query_row(
            "SELECT secret_enc, secret_nonce FROM credentials WHERE id = ?1 AND deleted_at IS NULL",
            rusqlite::params![credential_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let (old_enc, old_nonce_bytes) = existing.ok_or(VaultError::NotFound)?;
    if old_nonce_bytes.len() != 12 {
        return Err(VaultError::UnlockFailed);
    }
    let mut old_nonce = [0u8; 12];
    old_nonce.copy_from_slice(&old_nonce_bytes);
    let old_plaintext = crate::crypto::open(key, &old_enc, &old_nonce)?;
    let old_secret = String::from_utf8(old_plaintext).map_err(|_| VaultError::UnlockFailed)?;
    let secret_changed = old_secret != secret;

    let (secret_enc, nonce) = crate::crypto::seal(key, secret.as_bytes())?;
    let (fields_enc, fields_nonce) = seal_custom_fields(key, custom_fields)?;
    let updated = if secret_changed {
        conn.execute(
            "UPDATE credentials SET username = ?1, secret_enc = ?2, secret_nonce = ?3, remarks = ?4,
             custom_fields_enc = ?5, custom_fields_nonce = ?6, reprompt = ?7,
             updated_at = datetime('now'), password_updated_at = datetime('now')
             WHERE id = ?8 AND deleted_at IS NULL",
            rusqlite::params![
                username, secret_enc, nonce.to_vec(), remarks,
                fields_enc, fields_nonce.to_vec(), reprompt, credential_id,
            ],
        )?
    } else {
        conn.execute(
            "UPDATE credentials SET username = ?1, secret_enc = ?2, secret_nonce = ?3, remarks = ?4,
             custom_fields_enc = ?5, custom_fields_nonce = ?6, reprompt = ?7,
             updated_at = datetime('now') WHERE id = ?8 AND deleted_at IS NULL",
            rusqlite::params![
                username, secret_enc, nonce.to_vec(), remarks,
                fields_enc, fields_nonce.to_vec(), reprompt, credential_id,
            ],
        )?
    };
    if updated == 0 {
        return Err(VaultError::NotFound);
    }
    Ok(())
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
pub struct TrashedCredential {
    pub credential_id: i64,
    pub site_id: i64,
    pub site_name: String,
    pub username: Option<String>,
    pub deleted_at: String,
}

pub fn list_trash(conn: &Connection) -> Result<Vec<TrashedCredential>, VaultError> {
    let mut stmt = conn.prepare(
        "SELECT c.id, c.site_id, s.name, c.username, c.deleted_at
         FROM credentials c
         JOIN sites s ON s.id = c.site_id
         WHERE c.deleted_at IS NOT NULL
         ORDER BY c.deleted_at DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(TrashedCredential {
            credential_id: row.get(0)?,
            site_id: row.get(1)?,
            site_name: row.get(2)?,
            username: row.get(3)?,
            deleted_at: row.get(4)?,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub fn purge_trash(conn: &Connection) -> Result<usize, VaultError> {
    let count = conn.execute("DELETE FROM credentials WHERE deleted_at IS NOT NULL", [])?;
    Ok(count)
}

#[derive(Debug, Clone, PartialEq)]
pub struct OldCredential {
    pub credential_id: i64,
    pub site_id: i64,
    pub site_name: String,
    pub username: Option<String>,
    pub password_updated_at: String,
}

/// Credentials whose password hasn't changed in more than `threshold_days`. Uses SQLite's own
/// date arithmetic (no Rust-side date parsing, no new dependency) - `password_updated_at` is a
/// plaintext column, so this needs no decryption either.
pub fn list_old_credentials(conn: &Connection, threshold_days: u32) -> Result<Vec<OldCredential>, VaultError> {
    let mut stmt = conn.prepare(
        "SELECT c.id, c.site_id, s.name, c.username, c.password_updated_at
         FROM credentials c JOIN sites s ON s.id = c.site_id
         WHERE c.deleted_at IS NULL AND s.deleted_at IS NULL
           AND c.password_updated_at < datetime('now', '-' || ?1 || ' days')
         ORDER BY c.password_updated_at ASC",
    )?;
    let rows = stmt.query_map(rusqlite::params![threshold_days], |row| {
        Ok(OldCredential {
            credential_id: row.get(0)?,
            site_id: row.get(1)?,
            site_name: row.get(2)?,
            username: row.get(3)?,
            password_updated_at: row.get(4)?,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
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

#[derive(Debug, Clone, PartialEq)]
pub struct ImportSummary {
    pub imported: usize,
    pub duplicates: usize,
}

fn find_or_create_site(
    conn: &Connection,
    name: &str,
    url: Option<&str>,
    notes: Option<&str>,
) -> Result<i64, VaultError> {
    let existing: Option<i64> = conn
        .query_row(
            "SELECT id FROM sites WHERE name = ?1 AND deleted_at IS NULL",
            rusqlite::params![name],
            |row| row.get(0),
        )
        .optional()?;
    match existing {
        Some(id) => Ok(id),
        None => add_site(conn, name, url, notes),
    }
}

fn credential_exists(conn: &Connection, site_id: i64, username: Option<&str>) -> Result<bool, VaultError> {
    // `username IS ?2` (not `=`) so a NULL username compares NULL-safely, matching search()'s
    // established care around SQL NULL semantics elsewhere in this file.
    let count: i64 = conn.query_row(
        "SELECT count(*) FROM credentials WHERE site_id = ?1 AND username IS ?2 AND deleted_at IS NULL",
        rusqlite::params![site_id, username],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

/// Inserts an entire CSV import batch in one transaction. Any row missing a required field
/// (`site_name`) fails the whole batch - the `Transaction` rolls back on `Drop` since `?`'s early
/// return means `commit()` is never reached, needing no explicit rollback call. A row whose
/// site name + username already exists is skipped (counted, not an error) - this is the only
/// documented "duplicate" definition per the design; it does not fail the batch.
pub fn import_credentials(
    conn: &mut Connection,
    key: &VaultKey,
    records: &[ImportRecord],
) -> Result<ImportSummary, VaultError> {
    let tx = conn.transaction()?;
    let mut imported = 0usize;
    let mut duplicates = 0usize;

    for rec in records {
        if rec.site_name.trim().is_empty() {
            return Err(VaultError::InvalidImportRecord("site name is required".to_string()));
        }
        let site_id = find_or_create_site(&tx, &rec.site_name, rec.url.as_deref(), rec.notes.as_deref())?;
        if credential_exists(&tx, site_id, rec.username.as_deref())? {
            duplicates += 1;
            continue;
        }
        add_credential(&tx, key, site_id, rec.username.as_deref(), &rec.secret, rec.remarks.as_deref(), &[], false)?;
        imported += 1;
    }

    tx.commit()?;
    Ok(ImportSummary { imported, duplicates })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::derive_key;
    use tempfile::tempdir;

    fn get_password_updated_at(conn: &Connection, credential_id: i64) -> String {
        conn.query_row(
            "SELECT password_updated_at FROM credentials WHERE id = ?1",
            rusqlite::params![credential_id],
            |row| row.get(0),
        )
        .unwrap()
    }

    fn set_password_updated_at_sentinel(conn: &Connection, credential_id: i64) {
        // A clearly-stale fixed value (not "now") so the tests can assert "unchanged" vs "changed"
        // without racing datetime('now')'s 1-second resolution.
        conn.execute(
            "UPDATE credentials SET password_updated_at = '2000-01-01 00:00:00' WHERE id = ?1",
            rusqlite::params![credential_id],
        )
        .unwrap();
    }

    fn sample_import_record(site: &str, username: &str, secret: &str) -> ImportRecord {
        ImportRecord {
            site_name: site.to_string(),
            url: None,
            notes: None,
            username: Some(username.to_string()),
            secret: secret.to_string(),
            remarks: None,
        }
    }

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
    fn update_site_changes_name_url_and_notes() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[30u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();
        let id = add_site(&conn, "old.com", None, None).unwrap();

        update_site(&conn, id, "new.com", Some("https://new.com"), Some("note")).unwrap();

        let sites = list_sites(&conn).unwrap();
        assert_eq!(sites.len(), 1);
        assert_eq!(sites[0].name, "new.com");
        assert_eq!(sites[0].url.as_deref(), Some("https://new.com"));
        assert_eq!(sites[0].notes.as_deref(), Some("note"));
    }

    #[test]
    fn update_site_on_unknown_id_returns_not_found() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[31u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();

        let err = update_site(&conn, 999, "x", None, None).unwrap_err();
        assert!(matches!(err, VaultError::NotFound));
    }

    #[test]
    fn count_live_credentials_for_site_ignores_soft_deleted() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[32u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();
        let site_id = add_site(&conn, "example.com", None, None).unwrap();
        let cred_id = add_credential(&conn, &key, site_id, Some("alice"), "s3cret", None, &[], false).unwrap();

        assert_eq!(count_live_credentials_for_site(&conn, site_id).unwrap(), 1);

        soft_delete_credential(&conn, cred_id).unwrap();
        assert_eq!(count_live_credentials_for_site(&conn, site_id).unwrap(), 0);
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

        let cred_id = add_credential(&conn, &key, site_id, Some("alice"), "s3cret!", Some("work login"), &[], false).unwrap();
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
        add_credential(&conn, &key, site_id, Some("alice"), "extremely-unique-secret-xyz", None, &[], false).unwrap();
        let creds = list_credentials_for_site(&conn, &key, site_id).unwrap();

        let debug_output = format!("{:?}", creds[0]);
        assert!(!debug_output.contains("extremely-unique-secret-xyz"));
        assert!(debug_output.contains("<redacted>"));
    }

    #[test]
    fn update_credential_changes_username_secret_and_remarks() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[20u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();
        let site_id = add_site(&conn, "example.com", None, None).unwrap();
        let cred_id = add_credential(&conn, &key, site_id, Some("alice"), "old-secret", Some("old note"), &[], false).unwrap();

        update_credential(&conn, &key, cred_id, Some("alice2"), "new-secret", Some("new note"), &[], false).unwrap();

        let creds = list_credentials_for_site(&conn, &key, site_id).unwrap();
        assert_eq!(creds.len(), 1);
        assert_eq!(creds[0].username.as_deref(), Some("alice2"));
        assert_eq!(creds[0].secret, "new-secret");
        assert_eq!(creds[0].remarks.as_deref(), Some("new note"));
    }

    #[test]
    fn update_credential_on_nonexistent_id_returns_not_found() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[21u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();

        let result = update_credential(&conn, &key, 9999, Some("x"), "y", None, &[], false);

        assert!(matches!(result, Err(VaultError::NotFound)));
    }

    #[test]
    fn update_credential_on_soft_deleted_credential_returns_not_found() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[22u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();
        let site_id = add_site(&conn, "example.com", None, None).unwrap();
        let cred_id = add_credential(&conn, &key, site_id, Some("alice"), "secret", None, &[], false).unwrap();
        soft_delete_credential(&conn, cred_id).unwrap();

        let result = update_credential(&conn, &key, cred_id, Some("alice2"), "new-secret", None, &[], false);

        assert!(matches!(result, Err(VaultError::NotFound)));
    }

    #[test]
    fn new_credential_has_no_totp_by_default() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[23u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();
        let site_id = add_site(&conn, "example.com", None, None).unwrap();
        let cred_id = add_credential(&conn, &key, site_id, Some("alice"), "secret", None, &[], false).unwrap();

        let creds = list_credentials_for_site(&conn, &key, site_id).unwrap();

        assert!(!creds.iter().find(|c| c.id == cred_id).unwrap().has_totp);
        assert_eq!(get_totp_secret(&conn, &key, cred_id).unwrap(), None);
    }

    #[test]
    fn set_totp_secret_then_get_totp_secret_roundtrips_and_flags_has_totp() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[24u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();
        let site_id = add_site(&conn, "example.com", None, None).unwrap();
        let cred_id = add_credential(&conn, &key, site_id, Some("alice"), "secret", None, &[], false).unwrap();

        set_totp_secret(&conn, &key, cred_id, "JBSWY3DPEHPK3PXP").unwrap();

        assert_eq!(get_totp_secret(&conn, &key, cred_id).unwrap().as_deref(), Some("JBSWY3DPEHPK3PXP"));
        let creds = list_credentials_for_site(&conn, &key, site_id).unwrap();
        assert!(creds.iter().find(|c| c.id == cred_id).unwrap().has_totp);
    }

    #[test]
    fn clear_totp_secret_removes_it_without_touching_the_main_secret() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[25u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();
        let site_id = add_site(&conn, "example.com", None, None).unwrap();
        let cred_id = add_credential(&conn, &key, site_id, Some("alice"), "secret", None, &[], false).unwrap();
        set_totp_secret(&conn, &key, cred_id, "JBSWY3DPEHPK3PXP").unwrap();

        clear_totp_secret(&conn, cred_id).unwrap();

        assert_eq!(get_totp_secret(&conn, &key, cred_id).unwrap(), None);
        let creds = list_credentials_for_site(&conn, &key, site_id).unwrap();
        let cred = creds.iter().find(|c| c.id == cred_id).unwrap();
        assert!(!cred.has_totp);
        assert_eq!(cred.secret, "secret");
    }

    #[test]
    fn set_totp_secret_on_nonexistent_credential_returns_not_found() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[26u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();

        let result = set_totp_secret(&conn, &key, 9999, "JBSWY3DPEHPK3PXP");

        assert!(matches!(result, Err(VaultError::NotFound)));
    }

    #[test]
    fn update_credential_editing_only_username_does_not_bump_password_updated_at() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[40u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();
        let site_id = add_site(&conn, "example.com", None, None).unwrap();
        let cred_id = add_credential(&conn, &key, site_id, Some("alice"), "same-secret", None, &[], false).unwrap();
        set_password_updated_at_sentinel(&conn, cred_id);

        update_credential(&conn, &key, cred_id, Some("alice2"), "same-secret", Some("a note"), &[], false).unwrap();

        assert_eq!(get_password_updated_at(&conn, cred_id), "2000-01-01 00:00:00");
    }

    #[test]
    fn update_credential_changing_the_secret_bumps_password_updated_at() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[41u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();
        let site_id = add_site(&conn, "example.com", None, None).unwrap();
        let cred_id = add_credential(&conn, &key, site_id, Some("alice"), "old-secret", None, &[], false).unwrap();
        set_password_updated_at_sentinel(&conn, cred_id);

        update_credential(&conn, &key, cred_id, Some("alice"), "new-secret", None, &[], false).unwrap();

        assert_ne!(get_password_updated_at(&conn, cred_id), "2000-01-01 00:00:00");
    }

    #[test]
    fn soft_delete_then_restore_credential_round_trips() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[7u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();
        let site_id = add_site(&conn, "example.com", None, None).unwrap();
        let cred_id = add_credential(&conn, &key, site_id, Some("alice"), "s3cret!", None, &[], false).unwrap();

        soft_delete_credential(&conn, cred_id).unwrap();
        assert!(list_credentials_for_site(&conn, &key, site_id).unwrap().is_empty());

        restore_credential(&conn, cred_id).unwrap();
        assert_eq!(list_credentials_for_site(&conn, &key, site_id).unwrap().len(), 1);
    }

    #[test]
    fn list_trash_returns_soft_deleted_credentials_with_site_name() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[24u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();
        let site_id = add_site(&conn, "example.com", None, None).unwrap();
        let cred_id = add_credential(&conn, &key, site_id, Some("alice"), "s3cret", None, &[], false).unwrap();

        assert!(list_trash(&conn).unwrap().is_empty());

        soft_delete_credential(&conn, cred_id).unwrap();
        let trash = list_trash(&conn).unwrap();

        assert_eq!(trash.len(), 1);
        assert_eq!(trash[0].credential_id, cred_id);
        assert_eq!(trash[0].site_name, "example.com");
    }

    #[test]
    fn purge_trash_permanently_removes_soft_deleted_credentials_only() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[25u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();
        let site_id = add_site(&conn, "example.com", None, None).unwrap();
        let kept_id = add_credential(&conn, &key, site_id, Some("alice"), "keep-me", None, &[], false).unwrap();
        let trashed_id = add_credential(&conn, &key, site_id, Some("bob"), "purge-me", None, &[], false).unwrap();
        soft_delete_credential(&conn, trashed_id).unwrap();

        let purged_count = purge_trash(&conn).unwrap();

        assert_eq!(purged_count, 1);
        assert!(list_trash(&conn).unwrap().is_empty());
        assert_eq!(list_credentials_for_site(&conn, &key, site_id).unwrap().len(), 1);
        assert_eq!(list_credentials_for_site(&conn, &key, site_id).unwrap()[0].id, kept_id);

        // Purged rows must be gone, not just re-hidden - restore_credential on a purged id is a NotFound.
        assert!(matches!(restore_credential(&conn, trashed_id), Err(VaultError::NotFound)));
    }

    #[test]
    fn list_credentials_for_site_returns_only_that_sites_credentials() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[9u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();
        let site_a = add_site(&conn, "a.com", None, None).unwrap();
        let site_b = add_site(&conn, "b.com", None, None).unwrap();
        add_credential(&conn, &key, site_a, Some("alice"), "secret-a1", None, &[], false).unwrap();
        add_credential(&conn, &key, site_a, Some("alice2"), "secret-a2", None, &[], false).unwrap();
        add_credential(&conn, &key, site_b, Some("bob"), "secret-b", None, &[], false).unwrap();

        let creds_a = list_credentials_for_site(&conn, &key, site_a).unwrap();
        assert_eq!(creds_a.len(), 2);
        assert!(creds_a.iter().all(|c| c.site_id == site_a));

        let creds_b = list_credentials_for_site(&conn, &key, site_b).unwrap();
        assert_eq!(creds_b.len(), 1);
    }

    #[test]
    fn list_credentials_for_site_excludes_credentials_whose_site_is_soft_deleted() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[23u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();
        let site_id = add_site(&conn, "example.com", None, None).unwrap();
        add_credential(&conn, &key, site_id, Some("alice"), "s3cret", None, &[], false).unwrap();

        soft_delete_site(&conn, site_id).unwrap();

        // Before this fix, list_credentials_for_site only checked the credential's own deleted_at,
        // not its parent site's - so a credential under a soft-deleted site still showed up here even
        // though it's invisible to search()/list_sites().
        assert!(list_credentials_for_site(&conn, &key, site_id).unwrap().is_empty());
    }

    #[test]
    fn search_matches_site_name_username_and_remarks_case_insensitively() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[8u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();

        let site_a = add_site(&conn, "GitHub", None, None).unwrap();
        add_credential(&conn, &key, site_a, Some("alice"), "secret1", Some("work account"), &[], false).unwrap();

        let site_b = add_site(&conn, "example.com", None, None).unwrap();
        add_credential(&conn, &key, site_b, Some("bob"), "secret2", None, &[], false).unwrap();

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
        add_credential(&conn, &key, site_id, Some("jane_doe"), "secret", None, &[], false).unwrap();

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
        let cred_id = add_credential(&conn, &key, site_id, Some("carol"), "secret", None, &[], false).unwrap();
        assert_eq!(search(&conn, "deleteme").unwrap().len(), 1);

        soft_delete_credential(&conn, cred_id).unwrap();
        assert!(search(&conn, "deleteme").unwrap().is_empty());

        restore_credential(&conn, cred_id).unwrap();
        assert_eq!(search(&conn, "deleteme").unwrap().len(), 1);

        soft_delete_site(&conn, site_id).unwrap();
        assert!(search(&conn, "deleteme").unwrap().is_empty());
    }

    #[test]
    fn import_credentials_inserts_every_valid_row_and_reuses_a_shared_site() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[30u8; 16]).unwrap();
        let mut conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();
        let records = vec![
            sample_import_record("example.com", "alice", "secret-a"),
            sample_import_record("example.com", "bob", "secret-b"), // same site, different user
        ];

        let summary = import_credentials(&mut conn, &key, &records).unwrap();

        assert_eq!(summary.imported, 2);
        assert_eq!(summary.duplicates, 0);
        let sites = list_sites(&conn).unwrap();
        assert_eq!(sites.len(), 1, "both rows should share one site, not create two");
        assert_eq!(list_credentials_for_site(&conn, &key, sites[0].id).unwrap().len(), 2);
    }

    #[test]
    fn import_credentials_skips_a_duplicate_site_and_username_without_failing_the_batch() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[31u8; 16]).unwrap();
        let mut conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();
        let records = vec![
            sample_import_record("example.com", "alice", "secret-a"),
            sample_import_record("example.com", "alice", "secret-a-again"), // exact duplicate
        ];

        let summary = import_credentials(&mut conn, &key, &records).unwrap();

        assert_eq!(summary.imported, 1);
        assert_eq!(summary.duplicates, 1);
    }

    #[test]
    fn import_credentials_rolls_back_the_whole_batch_when_any_row_is_missing_a_required_field() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[32u8; 16]).unwrap();
        let mut conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();
        let records = vec![
            sample_import_record("valid.com", "alice", "secret-a"),
            ImportRecord {
                site_name: String::new(), // missing required field
                url: None,
                notes: None,
                username: Some("bob".to_string()),
                secret: "secret-b".to_string(),
                remarks: None,
            },
        ];

        let result = import_credentials(&mut conn, &key, &records);

        assert!(matches!(result, Err(VaultError::InvalidImportRecord(_))));
        // Nothing from the batch was persisted, including the row BEFORE the failing one.
        assert!(list_sites(&conn).unwrap().is_empty());
    }

    #[test]
    fn list_old_credentials_returns_only_credentials_older_than_the_threshold() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[42u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();
        let site_id = add_site(&conn, "example.com", None, None).unwrap();
        let old_id = add_credential(&conn, &key, site_id, Some("alice"), "secret-a", None, &[], false).unwrap();
        let fresh_id = add_credential(&conn, &key, site_id, Some("bob"), "secret-b", None, &[], false).unwrap();
        conn.execute(
            "UPDATE credentials SET password_updated_at = datetime('now', '-400 days') WHERE id = ?1",
            rusqlite::params![old_id],
        )
        .unwrap();
        // fresh_id keeps its just-created (now) password_updated_at.

        let old = list_old_credentials(&conn, 365).unwrap();

        assert_eq!(old.len(), 1);
        assert_eq!(old[0].credential_id, old_id);
        assert_eq!(old[0].site_name, "example.com");
        assert_eq!(old[0].username.as_deref(), Some("alice"));
        let _ = fresh_id; // not returned - kept only to make the "one is old, one isn't" setup explicit
    }

    #[test]
    fn list_old_credentials_excludes_soft_deleted_rows() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[43u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();
        let site_id = add_site(&conn, "example.com", None, None).unwrap();
        let cred_id = add_credential(&conn, &key, site_id, Some("alice"), "secret", None, &[], false).unwrap();
        conn.execute(
            "UPDATE credentials SET password_updated_at = datetime('now', '-400 days') WHERE id = ?1",
            rusqlite::params![cred_id],
        )
        .unwrap();
        soft_delete_credential(&conn, cred_id).unwrap();

        assert!(list_old_credentials(&conn, 365).unwrap().is_empty());
    }

    #[test]
    fn custom_field_deserializes_camel_case_field_type() {
        let field: CustomField =
            serde_json::from_str(r#"{"label":"PIN","value":"1234","fieldType":"hidden"}"#).unwrap();
        assert_eq!(field.field_type, CustomFieldType::Hidden);
    }

    #[test]
    fn custom_field_serializes_field_type_as_camel_case() {
        let field = CustomField {
            label: "PIN".into(),
            value: "1234".into(),
            field_type: CustomFieldType::Hidden,
        };
        let json = serde_json::to_string(&field).unwrap();
        assert!(json.contains("\"fieldType\""));
    }

    #[test]
    fn custom_field_still_deserializes_legacy_snake_case_key() {
        let fields: Vec<CustomField> =
            serde_json::from_slice(br#"[{"label":"PIN","value":"1234","field_type":"hidden"}]"#).unwrap();
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].field_type, CustomFieldType::Hidden);
    }

    #[test]
    fn credential_with_hidden_custom_field_round_trips_through_vault() {
        let dir = tempdir().unwrap();
        let key = derive_key("master password", &[45u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();
        let site_id = add_site(&conn, "example.com", None, None).unwrap();
        let custom_fields = vec![CustomField {
            label: "Recovery PIN".into(),
            value: "1234".into(),
            field_type: CustomFieldType::Hidden,
        }];
        add_credential(&conn, &key, site_id, Some("alice"), "secret", None, &custom_fields, false).unwrap();

        let creds = list_credentials_for_site(&conn, &key, site_id).unwrap();

        assert_eq!(creds.len(), 1);
        assert_eq!(creds[0].custom_fields.len(), 1);
        assert_eq!(creds[0].custom_fields[0].label, "Recovery PIN");
        assert_eq!(creds[0].custom_fields[0].value, "1234");
        assert_eq!(creds[0].custom_fields[0].field_type, CustomFieldType::Hidden);
    }

    #[test]
    fn list_old_credentials_excludes_credentials_on_a_soft_deleted_site() {
        // The join's other half - s.deleted_at IS NULL - has no coverage without this: a
        // credential can be individually untouched but its site soft-deleted (e.g. via Trash),
        // and it must still be excluded from the "old" report.
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[44u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();
        let site_id = add_site(&conn, "example.com", None, None).unwrap();
        let cred_id = add_credential(&conn, &key, site_id, Some("alice"), "secret", None, &[], false).unwrap();
        conn.execute(
            "UPDATE credentials SET password_updated_at = datetime('now', '-400 days') WHERE id = ?1",
            rusqlite::params![cred_id],
        )
        .unwrap();
        soft_delete_site(&conn, site_id).unwrap();

        assert!(list_old_credentials(&conn, 365).unwrap().is_empty());
    }
}
