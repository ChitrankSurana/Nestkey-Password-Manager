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
    #[error("invalid input: {0}")]
    InvalidInput(String),
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
        CREATE TABLE IF NOT EXISTS folders (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE COLLATE NOCASE
        );
        CREATE TABLE IF NOT EXISTS credential_history (
            id INTEGER PRIMARY KEY,
            credential_id INTEGER NOT NULL REFERENCES credentials(id) ON DELETE CASCADE,
            secret_enc BLOB NOT NULL,
            secret_nonce BLOB NOT NULL,
            changed_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_sites_name ON sites(name);
        CREATE INDEX IF NOT EXISTS idx_credentials_site ON credentials(site_id);
        CREATE INDEX IF NOT EXISTS idx_credential_history_credential ON credential_history(credential_id);",
    )?;
    add_credential_columns_if_missing(conn)?;
    add_site_columns_if_missing(conn)?;
    Ok(())
}

// Same mechanism (and same reason) as add_credential_columns_if_missing, for `sites`: the Make UI
// metadata columns don't exist in vault files created by earlier builds, and CREATE TABLE IF NOT
// EXISTS won't add them. Every column here is plaintext METADATA (SQLCipher still encrypts the
// file) - card numbers, CVVs and note bodies never get a column, they live in the sealed
// custom-fields blob under the reserved labels below.
fn add_site_columns_if_missing(conn: &Connection) -> Result<(), VaultError> {
    let mut stmt = conn.prepare("PRAGMA table_info(sites)")?;
    let existing: Vec<String> = stmt.query_map([], |row| row.get::<_, String>(1))?.collect::<Result<_, _>>()?;
    drop(stmt);
    let wanted = [
        ("favorite", "INTEGER NOT NULL DEFAULT 0"),
        ("folder_id", "INTEGER REFERENCES folders(id)"),
        ("item_type", "TEXT NOT NULL DEFAULT 'login'"),
        ("card_brand", "TEXT"),
        ("card_last4", "TEXT"),
    ];
    for (name, decl) in wanted {
        if !existing.iter().any(|c| c == name) {
            conn.execute(&format!("ALTER TABLE sites ADD COLUMN {name} {decl}"), [])?;
        }
    }
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
    pub favorite: bool,
    pub folder_id: Option<i64>,
    /// One of ITEM_TYPES. Anything that isn't "login" has exactly one credential row whose sealed
    /// custom fields carry the type's reserved labels (see reserved_fields_for).
    pub item_type: String,
    /// Non-secret display hints (e.g. "Visa", "3921") so a card row can show `•••• 3921` without
    /// decrypting anything. The full number is only ever in the sealed blob.
    pub card_brand: Option<String>,
    pub card_last4: Option<String>,
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
        "SELECT id, name, url, notes, favorite, folder_id, item_type, card_brand, card_last4
         FROM sites WHERE deleted_at IS NULL ORDER BY name",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Site {
            id: row.get(0)?,
            name: row.get(1)?,
            url: row.get(2)?,
            notes: row.get(3)?,
            favorite: row.get(4)?,
            folder_id: row.get(5)?,
            item_type: row.get(6)?,
            card_brand: row.get(7)?,
            card_last4: row.get(8)?,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub const ITEM_TYPES: [&str; 4] = ["login", "note", "card", "identity"];

// Reserved custom-field labels for non-login items. The label IS the field's identity, so it is a
// validated constant, never a free string: a typo must fail instead of silently forking a field.
pub const CARD_NUMBER: &str = "Card Number";
pub const CARD_EXPIRY: &str = "Expiration Date";
pub const CARD_CVV: &str = "Security Code (CVV)";
pub const NOTE_BODY: &str = "Note";
pub const IDENTITY_NAME: &str = "Full Name";

/// (label, required field type) pairs a non-login item type may carry. None = free-form (login).
fn reserved_fields_for(item_type: &str) -> Option<&'static [(&'static str, CustomFieldType)]> {
    use CustomFieldType::{Hidden, Text};
    match item_type {
        "card" => Some(&[(CARD_NUMBER, Hidden), (CARD_EXPIRY, Text), (CARD_CVV, Hidden)]),
        "note" => Some(&[(NOTE_BODY, Hidden)]),
        "identity" => Some(&[(IDENTITY_NAME, Text)]),
        _ => None,
    }
}

fn validate_fields_for_site(conn: &Connection, site_id: i64, fields: &[CustomField]) -> Result<(), VaultError> {
    let item_type: Option<String> = conn
        .query_row("SELECT item_type FROM sites WHERE id = ?1", rusqlite::params![site_id], |r| r.get(0))
        .optional()?;
    let item_type = item_type.ok_or(VaultError::NotFound)?;
    let Some(allowed) = reserved_fields_for(&item_type) else { return Ok(()) };
    for f in fields {
        match allowed.iter().find(|(label, _)| *label == f.label) {
            None => {
                return Err(VaultError::InvalidInput(format!("\"{}\" is not a valid field for a {} item.", f.label, item_type)))
            }
            Some((label, ty)) if *ty != f.field_type => {
                return Err(VaultError::InvalidInput(format!("\"{label}\" must be a {ty:?} field.")))
            }
            Some(_) => {}
        }
    }
    Ok(())
}

fn expect_one_row(updated: usize) -> Result<(), VaultError> {
    if updated == 0 { Err(VaultError::NotFound) } else { Ok(()) }
}

pub fn set_site_favorite(conn: &Connection, site_id: i64, favorite: bool) -> Result<(), VaultError> {
    expect_one_row(conn.execute(
        "UPDATE sites SET favorite = ?1 WHERE id = ?2 AND deleted_at IS NULL",
        rusqlite::params![favorite, site_id],
    )?)
}

pub fn set_site_item_type(conn: &Connection, site_id: i64, item_type: &str) -> Result<(), VaultError> {
    if !ITEM_TYPES.contains(&item_type) {
        return Err(VaultError::InvalidInput("Unknown item type.".into()));
    }
    expect_one_row(conn.execute(
        "UPDATE sites SET item_type = ?1 WHERE id = ?2 AND deleted_at IS NULL",
        rusqlite::params![item_type, site_id],
    )?)
}

pub fn set_site_card_hint(
    conn: &Connection,
    site_id: i64,
    brand: Option<&str>,
    last4: Option<&str>,
) -> Result<(), VaultError> {
    if let Some(l) = last4 {
        if l.len() != 4 || !l.chars().all(|c| c.is_ascii_digit()) {
            return Err(VaultError::InvalidInput("Card hint must be exactly the last 4 digits.".into()));
        }
    }
    expect_one_row(conn.execute(
        "UPDATE sites SET card_brand = ?1, card_last4 = ?2 WHERE id = ?3 AND deleted_at IS NULL",
        rusqlite::params![brand, last4, site_id],
    )?)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Folder {
    pub id: i64,
    pub name: String,
}

fn clean_folder_name(name: &str) -> Result<&str, VaultError> {
    let n = name.trim();
    if n.is_empty() {
        return Err(VaultError::InvalidInput("Folder name can't be empty.".into()));
    }
    Ok(n)
}

// UNIQUE ... COLLATE NOCASE on folders.name is the source of truth for duplicates; translate its
// constraint violation into a user-facing InvalidInput rather than a generic Db error.
fn folder_write_err(e: rusqlite::Error) -> VaultError {
    match e {
        rusqlite::Error::SqliteFailure(f, _) if f.code == rusqlite::ErrorCode::ConstraintViolation => {
            VaultError::InvalidInput("A folder with that name already exists.".into())
        }
        other => VaultError::Db(other),
    }
}

pub fn add_folder(conn: &Connection, name: &str) -> Result<i64, VaultError> {
    let name = clean_folder_name(name)?;
    conn.execute("INSERT INTO folders (name) VALUES (?1)", rusqlite::params![name]).map_err(folder_write_err)?;
    Ok(conn.last_insert_rowid())
}

pub fn list_folders(conn: &Connection) -> Result<Vec<Folder>, VaultError> {
    let mut stmt = conn.prepare("SELECT id, name FROM folders ORDER BY name COLLATE NOCASE")?;
    let rows = stmt.query_map([], |r| Ok(Folder { id: r.get(0)?, name: r.get(1)? }))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub fn rename_folder(conn: &Connection, folder_id: i64, name: &str) -> Result<(), VaultError> {
    let name = clean_folder_name(name)?;
    expect_one_row(
        conn.execute("UPDATE folders SET name = ?1 WHERE id = ?2", rusqlite::params![name, folder_id])
            .map_err(folder_write_err)?,
    )
}

/// Deleting a folder UNFILES its items (folder_id -> NULL); it never deletes them. One transaction
/// so a crash can't leave items pointing at a folder that no longer exists.
pub fn delete_folder(conn: &Connection, folder_id: i64) -> Result<(), VaultError> {
    let tx = conn.unchecked_transaction()?;
    tx.execute("UPDATE sites SET folder_id = NULL WHERE folder_id = ?1", rusqlite::params![folder_id])?;
    expect_one_row(tx.execute("DELETE FROM folders WHERE id = ?1", rusqlite::params![folder_id])?)?;
    tx.commit()?;
    Ok(())
}

pub fn set_site_folder(conn: &Connection, site_id: i64, folder_id: Option<i64>) -> Result<(), VaultError> {
    if let Some(fid) = folder_id {
        let exists: bool = conn.query_row("SELECT count(*) FROM folders WHERE id = ?1", rusqlite::params![fid], |r| r.get::<_, i64>(0))? > 0;
        if !exists {
            return Err(VaultError::NotFound);
        }
    }
    expect_one_row(conn.execute(
        "UPDATE sites SET folder_id = ?1 WHERE id = ?2 AND deleted_at IS NULL",
        rusqlite::params![folder_id, site_id],
    )?)
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

/// Soft-deletes a site AND cascades the delete to every one of its still-live credentials, in one
/// transaction. A half-applied delete (site marked deleted but its credentials left live) would
/// orphan nothing by itself, but the reverse - credentials marked deleted while their site stays
/// live - is never reachable through this function's own statement order, and a mid-transaction
/// crash must not be able to produce either partial state, since `list_credentials_for_site`
/// filters on both the credential's own `deleted_at` and its parent site's. Uses
/// `unchecked_transaction` (not `conn.transaction()`, which needs `&mut Connection`) because this
/// function only ever receives `&Connection` - the command layer holds it behind a mutex guard it
/// can't hand out `&mut` from.
pub fn soft_delete_site(conn: &Connection, site_id: i64) -> Result<(), VaultError> {
    let tx = conn.unchecked_transaction()?;
    let updated = tx.execute(
        "UPDATE sites SET deleted_at = datetime('now') WHERE id = ?1 AND deleted_at IS NULL",
        rusqlite::params![site_id],
    )?;
    if updated == 0 {
        return Err(VaultError::NotFound);
    }
    // `AND deleted_at IS NULL` here is required so a credential the user already trashed
    // separately keeps its own original timestamp and its place in Trash's deleted_at ordering,
    // instead of being silently re-stamped with "now" by this cascade.
    tx.execute(
        "UPDATE credentials SET deleted_at = datetime('now') WHERE site_id = ?1 AND deleted_at IS NULL",
        rusqlite::params![site_id],
    )?;
    tx.commit()?;
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
    validate_fields_for_site(conn, site_id, custom_fields)?;
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
    let owner: Option<i64> = conn
        .query_row(
            "SELECT site_id FROM credentials WHERE id = ?1 AND deleted_at IS NULL",
            rusqlite::params![credential_id],
            |r| r.get(0),
        )
        .optional()?;
    validate_fields_for_site(conn, owner.ok_or(VaultError::NotFound)?, custom_fields)?;
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
    if secret_changed {
        // B1-4: the outgoing secret is already sealed (old_enc/old_nonce_bytes) - no extra
        // encrypt work needed, just carry the existing ciphertext into the history table. Every
        // historical password is as sensitive as the live one, so it gets the exact same seal,
        // never a weaker one. Capped at 5 (oldest beyond that pruned immediately) - unbounded
        // growth was never the intent (see the plan's own B1-4 entry).
        conn.execute(
            "INSERT INTO credential_history (credential_id, secret_enc, secret_nonce) VALUES (?1, ?2, ?3)",
            rusqlite::params![credential_id, old_enc, old_nonce_bytes],
        )?;
        conn.execute(
            "DELETE FROM credential_history WHERE credential_id = ?1 AND id NOT IN (
                SELECT id FROM credential_history WHERE credential_id = ?1 ORDER BY changed_at DESC, id DESC LIMIT 5
             )",
            rusqlite::params![credential_id],
        )?;
    }
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

#[derive(Clone, PartialEq)]
pub struct HistoryEntry {
    pub secret: String,
    pub changed_at: String,
}

// No `Debug` derive (same rule as `Credential` above) - this carries a past plaintext password,
// so an accidental log/panic message must not be able to print it.
impl std::fmt::Debug for HistoryEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HistoryEntry").field("secret", &"<redacted>").field("changed_at", &self.changed_at).finish()
    }
}

/// Up to 5 past secrets for one credential, most recent first - what `update_credential` wrote to
/// `credential_history` (B1-4). Every row was sealed with the same key as the live secret.
pub fn list_credential_history(conn: &Connection, key: &VaultKey, credential_id: i64) -> Result<Vec<HistoryEntry>, VaultError> {
    let mut stmt = conn.prepare(
        "SELECT secret_enc, secret_nonce, changed_at FROM credential_history
         WHERE credential_id = ?1 ORDER BY changed_at DESC, id DESC",
    )?;
    let rows = stmt.query_map(rusqlite::params![credential_id], |row| {
        Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, Vec<u8>>(1)?, row.get::<_, String>(2)?))
    })?;
    let mut out = Vec::new();
    for r in rows {
        let (enc, nonce_bytes, changed_at) = r?;
        if nonce_bytes.len() != 12 {
            return Err(VaultError::UnlockFailed);
        }
        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(&nonce_bytes);
        let plaintext = crate::crypto::open(key, &enc, &nonce)?;
        let secret = String::from_utf8(plaintext).map_err(|_| VaultError::UnlockFailed)?;
        out.push(HistoryEntry { secret, changed_at });
    }
    Ok(out)
}

fn reseal_bytes(old_key: &VaultKey, new_key: &VaultKey, enc: &[u8], nonce_bytes: &[u8]) -> Result<(Vec<u8>, Vec<u8>), VaultError> {
    if nonce_bytes.len() != 12 {
        return Err(VaultError::UnlockFailed);
    }
    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(nonce_bytes);
    let plaintext = crate::crypto::open(old_key, enc, &nonce)?;
    let (new_enc, new_nonce) = crate::crypto::seal(new_key, &plaintext)?;
    Ok((new_enc, new_nonce.to_vec()))
}

/// B1-6, step 2a: re-seals every app-level sealed blob - `credentials.secret_enc`,
/// `.custom_fields_enc` (nullable), `.totp_secret_enc` (nullable), `credential_history.secret_enc`,
/// `ssh_keys.private_key_enc` - decrypting with `old_key` and writing back under `new_key`.
///
/// Must run BEFORE `rekey_vault`, not after: these are ordinary `UPDATE`s against a database that
/// right now is still opened, at the SQLCipher/file level, under `old_key` - this function only
/// ever changes the *app-level* key (the one `derive_key` hands out), which is layered on top of
/// and entirely independent of SQLCipher's own page encryption. Running this first means a
/// rolled-back transaction here leaves every blob exactly as it was, still fully readable under
/// `old_key` - the database file itself was never touched, so no separate recovery is needed for
/// a failure at this stage specifically (only for a failure at the `rekey_vault` stage after this
/// one commits - see that function's own doc comment).
///
/// Scans every row unconditionally, regardless of `deleted_at` - a soft-deleted-but-not-yet-purged
/// credential must stay restorable after a password change, not silently become undecryptable.
pub fn reseal_all_blobs(conn: &Connection, old_key: &VaultKey, new_key: &VaultKey) -> Result<(), VaultError> {
    let tx = conn.unchecked_transaction()?;

    type CredRow = (i64, Vec<u8>, Vec<u8>, Option<Vec<u8>>, Option<Vec<u8>>, Option<Vec<u8>>, Option<Vec<u8>>);
    let cred_rows: Vec<CredRow> = {
        let mut stmt = tx.prepare(
            "SELECT id, secret_enc, secret_nonce, custom_fields_enc, custom_fields_nonce,
                    totp_secret_enc, totp_secret_nonce
             FROM credentials",
        )?;
        let mapped = stmt.query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?))
        })?
        .collect::<Result<_, _>>()?;
        mapped
    };
    for (id, secret_enc, secret_nonce, fields_enc, fields_nonce, totp_enc, totp_nonce) in cred_rows {
        let (new_secret_enc, new_secret_nonce) = reseal_bytes(old_key, new_key, &secret_enc, &secret_nonce)?;
        let (new_fields_enc, new_fields_nonce) = match (fields_enc, fields_nonce) {
            (Some(e), Some(n)) => {
                let (ne, nn) = reseal_bytes(old_key, new_key, &e, &n)?;
                (Some(ne), Some(nn))
            }
            _ => (None, None),
        };
        let (new_totp_enc, new_totp_nonce) = match (totp_enc, totp_nonce) {
            (Some(e), Some(n)) => {
                let (ne, nn) = reseal_bytes(old_key, new_key, &e, &n)?;
                (Some(ne), Some(nn))
            }
            _ => (None, None),
        };
        tx.execute(
            "UPDATE credentials SET secret_enc = ?1, secret_nonce = ?2,
             custom_fields_enc = ?3, custom_fields_nonce = ?4,
             totp_secret_enc = ?5, totp_secret_nonce = ?6
             WHERE id = ?7",
            rusqlite::params![new_secret_enc, new_secret_nonce, new_fields_enc, new_fields_nonce, new_totp_enc, new_totp_nonce, id],
        )?;
    }

    let history_rows: Vec<(i64, Vec<u8>, Vec<u8>)> = {
        let mut stmt = tx.prepare("SELECT id, secret_enc, secret_nonce FROM credential_history")?;
        let mapped = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?.collect::<Result<_, _>>()?;
        mapped
    };
    for (id, enc, nonce) in history_rows {
        let (new_enc, new_nonce) = reseal_bytes(old_key, new_key, &enc, &nonce)?;
        tx.execute(
            "UPDATE credential_history SET secret_enc = ?1, secret_nonce = ?2 WHERE id = ?3",
            rusqlite::params![new_enc, new_nonce, id],
        )?;
    }

    let ssh_rows: Vec<(i64, Vec<u8>, Vec<u8>)> = {
        let mut stmt = tx.prepare("SELECT id, private_key_enc, private_key_nonce FROM ssh_keys")?;
        let mapped = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?.collect::<Result<_, _>>()?;
        mapped
    };
    for (id, enc, nonce) in ssh_rows {
        let (new_enc, new_nonce) = reseal_bytes(old_key, new_key, &enc, &nonce)?;
        tx.execute(
            "UPDATE ssh_keys SET private_key_enc = ?1, private_key_nonce = ?2 WHERE id = ?3",
            rusqlite::params![new_enc, new_nonce, id],
        )?;
    }

    tx.commit()?;
    Ok(())
}

/// B1-6, step 2b: re-encrypts the database FILE itself under `new_key` - SQLCipher's own
/// page-level encryption, entirely separate from the app-level blob sealing `reseal_all_blobs`
/// just finished. Uses the same raw-key `x'<hex>'` PRAGMA syntax `open_vault` uses for `PRAGMA
/// key`, so SQLCipher's own internal PBKDF2 is bypassed here too - this crate's Argon2id
/// `derive_key` already did the password -> key work, `new_key` is already the final key.
///
/// Must run AFTER `reseal_all_blobs` commits, not before - see that function's doc comment for
/// why. If this call itself fails or the process dies mid-call, the two key layers are left out
/// of sync (blobs sealed under `new_key`, file still keyed under `old_key`, or vice versa if
/// SQLCipher's own rekey didn't complete atomically) - this is the one window a pre-flight vault
/// backup exists to rescue: restoring it discards both the reseal and the rekey, returning to a
/// fully `old_key`-consistent file.
pub fn rekey_vault(conn: &Connection, new_key: &VaultKey) -> Result<(), VaultError> {
    let key_hex: zeroize::Zeroizing<String> =
        zeroize::Zeroizing::new(new_key.as_bytes().iter().map(|b| format!("{:02x}", b)).collect());
    conn.pragma_update(None, "rekey", format!("x'{}'", key_hex.as_str()))?;
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
    // Cascade-delete decision (Option A, 08-09-PLAN.md Task 1): soft_delete_site cascades to a
    // site's live credentials, and those cascaded rows surface in Trash under the deleted site's
    // name. Without this, restoring one of them here would clear the credential's own deleted_at
    // while its parent site stayed deleted - list_credentials_for_site filters on both, so the
    // credential would become live-but-unreachable with no restore-site UI anywhere to fix it.
    // This UPDATE is a no-op (0 rows) whenever the site was already live, so restoring an
    // individually-trashed credential under a live site is unaffected.
    conn.execute(
        "UPDATE sites SET deleted_at = NULL
         WHERE id = (SELECT site_id FROM credentials WHERE id = ?1) AND deleted_at IS NOT NULL",
        rusqlite::params![credential_id],
    )?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrashedCredential {
    pub credential_id: i64,
    pub site_id: i64,
    pub site_name: String,
    pub username: Option<String>,
    pub deleted_at: String,
    /// The site's item type (D-11 gap fix) - lets a trash view show a type-appropriate icon
    /// instead of assuming every trashed row is a login.
    pub item_type: String,
}

pub fn list_trash(conn: &Connection) -> Result<Vec<TrashedCredential>, VaultError> {
    let mut stmt = conn.prepare(
        "SELECT c.id, c.site_id, s.name, c.username, c.deleted_at, s.item_type
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
            item_type: row.get(5)?,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

// B1-5: purging trash only ever hard-deleted credentials - a site trashed with zero credentials
// (e.g. an empty login that was never given one, or a site whose sole credential was already
// purged in an earlier call) had no purge path of its own: it isn't in `list_sites` (deleted_at
// set) and it isn't in Trash either (Trash lists credentials, not sites), so it lived in the
// database forever with no UI ever able to reach it again. Fix: after credentials are purged,
// also hard-delete any trashed site left with no credential row (live or trashed) still
// referencing it - same transaction, so a crash between the two deletes can't leave one without
// the other.
pub fn purge_trash(conn: &Connection) -> Result<usize, VaultError> {
    let tx = conn.unchecked_transaction()?;
    let count = tx.execute("DELETE FROM credentials WHERE deleted_at IS NOT NULL", [])?;
    tx.execute(
        "DELETE FROM sites WHERE deleted_at IS NOT NULL
         AND id NOT IN (SELECT DISTINCT site_id FROM credentials)",
        [],
    )?;
    tx.commit()?;
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

// B1-7: search covers metadata (site name/notes, username, remarks - all plaintext columns, still
// pushed down as a SQL LIKE) AND secret content (D-I) - a non-login item's whole reason for
// existing lives in its sealed custom fields (note body, card number/expiry/CVV, full name), so a
// search that only looked at plaintext columns would never find a note or card by its content.
// There is no SQL-level index into an encrypted blob, so the secret-content half necessarily scans
// every live credential and decrypts its custom-fields blob in Rust - O(n) per query, fine at
// personal-vault scale (a few hundred items), revisit only if it measurably drags (same ceiling
// B1-7's plan entry already named).
pub fn search(conn: &Connection, key: &VaultKey, query: &str) -> Result<Vec<SearchResult>, VaultError> {
    let needle = query.to_lowercase();
    if needle.is_empty() {
        return Ok(Vec::new());
    }

    let mut stmt = conn.prepare(
        "SELECT s.id, s.name, s.notes, c.id, c.username, c.remarks, c.custom_fields_enc, c.custom_fields_nonce
         FROM credentials c
         JOIN sites s ON s.id = c.site_id
         WHERE c.deleted_at IS NULL AND s.deleted_at IS NULL
         ORDER BY s.name",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, Option<String>>(5)?,
            row.get::<_, Option<Vec<u8>>>(6)?,
            row.get::<_, Option<Vec<u8>>>(7)?,
        ))
    })?;

    let mut out = Vec::new();
    for r in rows {
        let (site_id, site_name, site_notes, credential_id, username, remarks, fields_enc, fields_nonce) = r?;

        let plaintext_match = site_name.to_lowercase().contains(&needle)
            || site_notes.as_deref().unwrap_or("").to_lowercase().contains(&needle)
            || username.as_deref().unwrap_or("").to_lowercase().contains(&needle)
            || remarks.as_deref().unwrap_or("").to_lowercase().contains(&needle);

        let secret_match = if plaintext_match {
            false // already matched, no need to decrypt
        } else {
            open_custom_fields(key, fields_enc, fields_nonce)?
                .iter()
                .any(|f| f.value.to_lowercase().contains(&needle))
        };

        if plaintext_match || secret_match {
            out.push(SearchResult { site_id, site_name, credential_id, username });
        }
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
    fn soft_delete_site_cascades_to_its_live_credentials() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[50u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();
        let site_id = add_site(&conn, "example.com", None, None).unwrap();
        add_credential(&conn, &key, site_id, Some("alice"), "s3cret-a", None, &[], false).unwrap();
        add_credential(&conn, &key, site_id, Some("bob"), "s3cret-b", None, &[], false).unwrap();

        soft_delete_site(&conn, site_id).unwrap();

        let live: i64 = conn
            .query_row(
                "SELECT count(*) FROM credentials WHERE site_id = ?1 AND deleted_at IS NULL",
                rusqlite::params![site_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(live, 0);
    }

    #[test]
    fn soft_delete_site_does_not_rewrite_an_already_trashed_credential() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[51u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();
        let site_id = add_site(&conn, "example.com", None, None).unwrap();
        let cred_id = add_credential(&conn, &key, site_id, Some("alice"), "s3cret-a", None, &[], false).unwrap();
        soft_delete_credential(&conn, cred_id).unwrap();
        let original_deleted_at: String = conn
            .query_row(
                "SELECT deleted_at FROM credentials WHERE id = ?1",
                rusqlite::params![cred_id],
                |row| row.get(0),
            )
            .unwrap();

        soft_delete_site(&conn, site_id).unwrap();

        let after_cascade: String = conn
            .query_row(
                "SELECT deleted_at FROM credentials WHERE id = ?1",
                rusqlite::params![cred_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(after_cascade, original_deleted_at);
    }

    #[test]
    fn soft_delete_site_on_an_already_deleted_site_still_returns_not_found() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[52u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();
        let site_id = add_site(&conn, "example.com", None, None).unwrap();
        soft_delete_site(&conn, site_id).unwrap();

        let result = soft_delete_site(&conn, site_id);

        assert!(matches!(result, Err(VaultError::NotFound)));
    }

    #[test]
    fn cascade_deleted_credentials_appear_in_trash() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[53u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();
        let site_id = add_site(&conn, "example.com", None, None).unwrap();
        add_credential(&conn, &key, site_id, Some("alice"), "s3cret-a", None, &[], false).unwrap();
        add_credential(&conn, &key, site_id, Some("bob"), "s3cret-b", None, &[], false).unwrap();

        soft_delete_site(&conn, site_id).unwrap();

        let trash = list_trash(&conn).unwrap();
        assert_eq!(trash.len(), 2);
        assert!(trash.iter().all(|t| t.site_name == "example.com"));
    }

    #[test]
    fn restore_credential_revives_its_cascade_deleted_site() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[54u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();
        let site_id = add_site(&conn, "example.com", None, None).unwrap();
        let cred_id = add_credential(&conn, &key, site_id, Some("alice"), "s3cret-a", None, &[], false).unwrap();
        soft_delete_site(&conn, site_id).unwrap();
        assert!(list_sites(&conn).unwrap().is_empty());

        restore_credential(&conn, cred_id).unwrap();

        let sites = list_sites(&conn).unwrap();
        assert_eq!(sites.len(), 1);
        assert_eq!(sites[0].id, site_id);
        let creds = list_credentials_for_site(&conn, &key, site_id).unwrap();
        assert_eq!(creds.len(), 1);
        assert_eq!(creds[0].id, cred_id);
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
    fn update_credential_writes_history_only_when_the_secret_actually_changes() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[21u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();
        let site_id = add_site(&conn, "example.com", None, None).unwrap();
        let cred_id = add_credential(&conn, &key, site_id, Some("alice"), "v1", None, &[], false).unwrap();

        // Same secret, different username - not a secret change, no history row.
        update_credential(&conn, &key, cred_id, Some("alice2"), "v1", None, &[], false).unwrap();
        assert!(list_credential_history(&conn, &key, cred_id).unwrap().is_empty());

        update_credential(&conn, &key, cred_id, Some("alice2"), "v2", None, &[], false).unwrap();
        let history = list_credential_history(&conn, &key, cred_id).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].secret, "v1"); // the outgoing secret, not the new one
    }

    #[test]
    fn credential_history_caps_at_five_most_recent() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[22u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();
        let site_id = add_site(&conn, "example.com", None, None).unwrap();
        let cred_id = add_credential(&conn, &key, site_id, None, "v0", None, &[], false).unwrap();

        for i in 1..=6 {
            update_credential(&conn, &key, cred_id, None, &format!("v{i}"), None, &[], false).unwrap();
        }

        // 6 changes (v0->v1 .. v5->v6) means 6 outgoing secrets went to history; capped at 5, and
        // the oldest (v0) is the one pruned - the 5 most recent outgoing secrets survive.
        let history = list_credential_history(&conn, &key, cred_id).unwrap();
        assert_eq!(history.len(), 5);
        let secrets: Vec<&str> = history.iter().map(|h| h.secret.as_str()).collect();
        assert_eq!(secrets, vec!["v5", "v4", "v3", "v2", "v1"]); // most recent first, v0 pruned
    }

    #[test]
    fn purging_a_trashed_credential_cascades_its_history() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[23u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();
        let site_id = add_site(&conn, "example.com", None, None).unwrap();
        let cred_id = add_credential(&conn, &key, site_id, None, "v1", None, &[], false).unwrap();
        update_credential(&conn, &key, cred_id, None, "v2", None, &[], false).unwrap();
        assert_eq!(list_credential_history(&conn, &key, cred_id).unwrap().len(), 1);

        soft_delete_credential(&conn, cred_id).unwrap();
        purge_trash(&conn).unwrap();

        // ON DELETE CASCADE on credential_history.credential_id - no orphaned history row left
        // pointing at a credential id that no longer exists.
        assert!(list_credential_history(&conn, &key, cred_id).unwrap().is_empty());
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
    fn purge_trash_also_removes_an_orphaned_trashed_site_with_no_credentials() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[26u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();

        // A site with zero credentials, trashed directly - before B1-5 this row had no purge path
        // at all: not in list_sites (deleted_at set), not in Trash (Trash lists credentials).
        let empty_site = add_site(&conn, "empty.example", None, None).unwrap();
        soft_delete_site(&conn, empty_site).unwrap();

        // A second site whose sole credential gets purged in this same call - also orphaned.
        let credless_after_purge = add_site(&conn, "purged-away.example", None, None).unwrap();
        let cred_id = add_credential(&conn, &key, credless_after_purge, None, "s3cret", None, &[], false).unwrap();
        soft_delete_site(&conn, credless_after_purge).unwrap();
        assert!(list_credentials_for_site(&conn, &key, credless_after_purge).unwrap().is_empty()); // cascaded to trash

        // Control: a site with a still-live credential must survive.
        let untouched_site = add_site(&conn, "untouched.example", None, None).unwrap();
        add_credential(&conn, &key, untouched_site, None, "keep-me", None, &[], false).unwrap();

        purge_trash(&conn).unwrap();

        let site_exists = |id: i64| -> bool {
            conn.query_row("SELECT count(*) FROM sites WHERE id = ?1", rusqlite::params![id], |r| r.get::<_, i64>(0))
                .unwrap()
                > 0
        };
        assert!(!site_exists(empty_site));
        assert!(!site_exists(credless_after_purge));
        assert!(site_exists(untouched_site));
        let _ = cred_id; // only needed to construct the scenario
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

        let by_site_name = search(&conn, &key, "github").unwrap();
        assert_eq!(by_site_name.len(), 1);
        assert_eq!(by_site_name[0].site_name, "GitHub");

        let by_username = search(&conn, &key, "bob").unwrap();
        assert_eq!(by_username.len(), 1);
        assert_eq!(by_username[0].username.as_deref(), Some("bob"));

        let by_remarks = search(&conn, &key, "work").unwrap();
        assert_eq!(by_remarks.len(), 1);
        assert_eq!(by_remarks[0].site_name, "GitHub");

        let no_match = search(&conn, &key, "nonexistent").unwrap();
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
        assert_eq!(search(&conn, &key, "50% off").unwrap().len(), 1);

        // A literal '_' must not behave like "match any single character".
        assert!(search(&conn, &key, "jane.doe").unwrap().is_empty());
        assert_eq!(search(&conn, &key, "jane_doe").unwrap().len(), 1);
    }

    #[test]
    fn search_matches_sealed_custom_field_content_but_not_the_plaintext_columns() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[12u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();

        let note_site = add_site(&conn, "Bank Note", None, None).unwrap();
        set_site_item_type(&conn, note_site, "note").unwrap();
        add_credential(
            &conn, &key, note_site, None, "", None,
            &[CustomField { label: NOTE_BODY.into(), value: "routing number 021000021".into(), field_type: CustomFieldType::Hidden }],
            false,
        ).unwrap();

        let login_site = add_site(&conn, "Unrelated Login", None, None).unwrap();
        add_credential(&conn, &key, login_site, Some("carol"), "secret", None, &[], false).unwrap();

        // The needle only appears inside the sealed custom-fields blob - a plaintext-only search
        // (the pre-B1-7 behavior) would find nothing here.
        let results = search(&conn, &key, "routing number").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].site_id, note_site);

        assert!(search(&conn, &key, "unrelated").unwrap().iter().any(|r| r.site_id == login_site));
    }

    #[test]
    fn search_excludes_soft_deleted_sites_and_credentials() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[10u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();

        let site_id = add_site(&conn, "deleteme.com", None, None).unwrap();
        let cred_id = add_credential(&conn, &key, site_id, Some("carol"), "secret", None, &[], false).unwrap();
        assert_eq!(search(&conn, &key, "deleteme").unwrap().len(), 1);

        soft_delete_credential(&conn, cred_id).unwrap();
        assert!(search(&conn, &key, "deleteme").unwrap().is_empty());

        restore_credential(&conn, cred_id).unwrap();
        assert_eq!(search(&conn, &key, "deleteme").unwrap().len(), 1);

        soft_delete_site(&conn, site_id).unwrap();
        assert!(search(&conn, &key, "deleteme").unwrap().is_empty());
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

    // ---- Make UI: favorites / folders / item types (plan B1-0..B1-3) ----

    /// Builds a vault file with the ORIGINAL pre-Make `sites` table (no favorite/folder/type
    /// columns), the way a real existing user's vault looks, then reopens it through open_vault.
    fn open_pre_migration_vault(dir: &std::path::Path, key: &VaultKey) -> Connection {
        let path = dir.join("old.vault");
        {
            let raw = Connection::open(&path).unwrap();
            let key_hex: String = key.as_bytes().iter().map(|b| format!("{:02x}", b)).collect();
            raw.pragma_update(None, "key", format!("x'{}'", key_hex)).unwrap();
            raw.execute_batch(
                "CREATE TABLE sites (
                    id INTEGER PRIMARY KEY, name TEXT NOT NULL, url TEXT, notes TEXT,
                    created_at TEXT NOT NULL DEFAULT (datetime('now')),
                    updated_at TEXT NOT NULL DEFAULT (datetime('now')), deleted_at TEXT);
                 INSERT INTO sites (name, url) VALUES ('legacy.com', 'https://legacy.com');",
            )
            .unwrap();
        }
        open_vault(&path, key).unwrap()
    }

    fn site_columns(conn: &Connection) -> Vec<String> {
        let mut stmt = conn.prepare("PRAGMA table_info(sites)").unwrap();
        stmt.query_map([], |r| r.get::<_, String>(1)).unwrap().map(|c| c.unwrap()).collect()
    }

    #[test]
    fn opening_a_pre_migration_vault_adds_site_columns_with_defaults_and_is_idempotent() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[60u8; 16]).unwrap();
        let conn = open_pre_migration_vault(dir.path(), &key);

        for col in ["favorite", "folder_id", "item_type", "card_brand", "card_last4"] {
            assert!(site_columns(&conn).iter().any(|c| c == col), "missing column {col}");
        }
        let sites = list_sites(&conn).unwrap();
        assert_eq!(sites.len(), 1);
        assert_eq!(sites[0].name, "legacy.com");
        assert!(!sites[0].favorite);
        assert_eq!(sites[0].folder_id, None);
        assert_eq!(sites[0].item_type, "login");
        drop(conn);

        // Second open must not try to re-add columns.
        let again = open_vault(&dir.path().join("old.vault"), &key).unwrap();
        assert_eq!(list_sites(&again).unwrap().len(), 1);
    }

    #[test]
    fn set_site_favorite_toggles_and_unknown_site_is_not_found() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[61u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();
        let id = add_site(&conn, "a.com", None, None).unwrap();

        set_site_favorite(&conn, id, true).unwrap();
        assert!(list_sites(&conn).unwrap()[0].favorite);
        set_site_favorite(&conn, id, false).unwrap();
        assert!(!list_sites(&conn).unwrap()[0].favorite);
        assert!(matches!(set_site_favorite(&conn, 999, true), Err(VaultError::NotFound)));
    }

    #[test]
    fn folders_crud_assign_and_delete_unfiles_sites_without_deleting_them() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[62u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();
        let site = add_site(&conn, "a.com", None, None).unwrap();

        let work = add_folder(&conn, "  Work ").unwrap();
        let dev = add_folder(&conn, "Development").unwrap();
        assert_eq!(list_folders(&conn).unwrap().iter().map(|f| f.name.as_str()).collect::<Vec<_>>(), ["Development", "Work"]);

        assert!(matches!(add_folder(&conn, "work"), Err(VaultError::InvalidInput(_))), "case-insensitive duplicate");
        assert!(matches!(add_folder(&conn, "   "), Err(VaultError::InvalidInput(_))), "blank name");

        rename_folder(&conn, work, "Office").unwrap();
        assert!(matches!(rename_folder(&conn, work, "development"), Err(VaultError::InvalidInput(_))));
        assert!(matches!(rename_folder(&conn, 999, "x"), Err(VaultError::NotFound)));

        set_site_folder(&conn, site, Some(dev)).unwrap();
        assert_eq!(list_sites(&conn).unwrap()[0].folder_id, Some(dev));
        assert!(matches!(set_site_folder(&conn, site, Some(999)), Err(VaultError::NotFound)));

        delete_folder(&conn, dev).unwrap();
        let sites = list_sites(&conn).unwrap();
        assert_eq!(sites.len(), 1, "deleting a folder must never delete its items");
        assert_eq!(sites[0].folder_id, None);
        assert!(matches!(delete_folder(&conn, dev), Err(VaultError::NotFound)));

        set_site_folder(&conn, site, Some(work)).unwrap();
        set_site_folder(&conn, site, None).unwrap();
        assert_eq!(list_sites(&conn).unwrap()[0].folder_id, None);
    }

    #[test]
    fn item_type_defaults_to_login_and_rejects_unknown_values() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[63u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();
        let id = add_site(&conn, "a.com", None, None).unwrap();
        assert_eq!(list_sites(&conn).unwrap()[0].item_type, "login");

        for t in ["note", "card", "identity", "login"] {
            set_site_item_type(&conn, id, t).unwrap();
            assert_eq!(list_sites(&conn).unwrap()[0].item_type, t);
        }
        assert!(matches!(set_site_item_type(&conn, id, "passport"), Err(VaultError::InvalidInput(_))));
        assert!(matches!(set_site_item_type(&conn, 999, "note"), Err(VaultError::NotFound)));
    }

    #[test]
    fn card_hint_accepts_brand_and_exactly_four_digits_only() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[64u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();
        let id = add_site(&conn, "Chase", None, None).unwrap();

        set_site_card_hint(&conn, id, Some("Visa"), Some("3921")).unwrap();
        let s = &list_sites(&conn).unwrap()[0];
        assert_eq!((s.card_brand.as_deref(), s.card_last4.as_deref()), (Some("Visa"), Some("3921")));

        for bad in ["392", "39211", "39a1", "3921 "] {
            assert!(matches!(set_site_card_hint(&conn, id, None, Some(bad)), Err(VaultError::InvalidInput(_))), "{bad}");
        }
        set_site_card_hint(&conn, id, None, None).unwrap();
        assert_eq!(list_sites(&conn).unwrap()[0].card_last4, None);
    }

    #[test]
    fn non_login_items_only_accept_their_reserved_field_labels() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[65u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();
        let card = add_site(&conn, "Chase", None, None).unwrap();
        set_site_item_type(&conn, card, "card").unwrap();
        let field = |label: &str, t: CustomFieldType| CustomField { label: label.into(), value: "v".into(), field_type: t };

        let ok = [
            field(CARD_NUMBER, CustomFieldType::Hidden),
            field(CARD_EXPIRY, CustomFieldType::Text),
            field(CARD_CVV, CustomFieldType::Hidden),
        ];
        let cred = add_credential(&conn, &key, card, None, "", None, &ok, false).unwrap();

        // A typo must fail loudly instead of silently forking the field.
        let typo = [field("Card Numbr", CustomFieldType::Hidden)];
        assert!(matches!(add_credential(&conn, &key, card, None, "", None, &typo, false), Err(VaultError::InvalidInput(_))));
        assert!(matches!(update_credential(&conn, &key, cred, None, "", None, &typo, false), Err(VaultError::InvalidInput(_))));
        // Another type's label is also rejected.
        let wrong = [field(NOTE_BODY, CustomFieldType::Hidden)];
        assert!(matches!(add_credential(&conn, &key, card, None, "", None, &wrong, false), Err(VaultError::InvalidInput(_))));

        // Secret-bearing reserved fields must be Hidden so they never render as plain text.
        let leaky = [field(CARD_CVV, CustomFieldType::Text)];
        assert!(matches!(add_credential(&conn, &key, card, None, "", None, &leaky, false), Err(VaultError::InvalidInput(_))));

        // Logins stay free-form.
        let login = add_site(&conn, "GitHub", None, None).unwrap();
        let any = [field("Recovery Code", CustomFieldType::Hidden)];
        add_credential(&conn, &key, login, Some("u"), "pw", None, &any, false).unwrap();
    }

    #[test]
    fn reseal_all_blobs_then_rekey_vault_makes_every_sealed_value_readable_under_the_new_key_only() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("t.vault");
        let old_key = derive_key("old password", &[70u8; 16]).unwrap();
        let conn = open_vault(&path, &old_key).unwrap();

        let site = add_site(&conn, "example.com", None, None).unwrap();
        let fields = [CustomField { label: "Recovery Code".into(), value: "8842-1923".into(), field_type: CustomFieldType::Hidden }];
        let cred = add_credential(&conn, &old_key, site, Some("alice"), "v1", None, &fields, false).unwrap();
        update_credential(&conn, &old_key, cred, Some("alice"), "v2", None, &fields, false).unwrap(); // writes one history row
        set_totp_secret(&conn, &old_key, cred, "JBSWY3DPEHPK3PXP").unwrap();
        crate::ssh_keys::add_ssh_key(&conn, &old_key, "laptop", "ssh-ed25519 AAAA...fake-key-material").unwrap();

        let new_key = derive_key("new password", &[71u8; 16]).unwrap();
        reseal_all_blobs(&conn, &old_key, &new_key).unwrap();
        rekey_vault(&conn, &new_key).unwrap();

        // The still-open connection keeps working transparently once rekey completes.
        let creds = list_credentials_for_site(&conn, &new_key, site).unwrap();
        assert_eq!(creds[0].secret, "v2");
        assert_eq!(creds[0].custom_fields[0].value, "8842-1923");
        assert!(creds[0].has_totp);
        let history = list_credential_history(&conn, &new_key, cred).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].secret, "v1");
        let ssh_keys = crate::ssh_keys::list_ssh_keys(&conn, &new_key).unwrap();
        assert_eq!(ssh_keys[0].private_key_openssh, "ssh-ed25519 AAAA...fake-key-material");

        // Reopening the file from scratch proves the rekey persisted to disk, not just to the
        // live connection's in-memory state - and that the old key no longer opens it at all.
        assert!(open_vault(&path, &new_key).is_ok());
        drop(conn);
        assert!(matches!(open_vault(&path, &old_key), Err(VaultError::UnlockFailed)));
    }

    #[test]
    fn reseal_all_blobs_with_the_wrong_old_key_fails_and_changes_nothing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("t.vault");
        let real_key = derive_key("real password", &[72u8; 16]).unwrap();
        let conn = open_vault(&path, &real_key).unwrap();
        let site = add_site(&conn, "example.com", None, None).unwrap();
        add_credential(&conn, &real_key, site, Some("alice"), "s3cret", None, &[], false).unwrap();

        let wrong_old_key = derive_key("not the real password", &[73u8; 16]).unwrap();
        let new_key = derive_key("new password", &[74u8; 16]).unwrap();
        let result = reseal_all_blobs(&conn, &wrong_old_key, &new_key);

        assert!(result.is_err());
        // The transaction must have rolled back - the credential is still readable under the
        // original key, exactly as if reseal_all_blobs had never been called.
        let creds = list_credentials_for_site(&conn, &real_key, site).unwrap();
        assert_eq!(creds[0].secret, "s3cret");
    }
}
