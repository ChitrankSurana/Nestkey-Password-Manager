//! Full-fidelity JSON vault export/import (Wave 4 follow-up from the UAT remediation plan).
//! CSV export/import (`portio.rs`) only ever carried site name/URL/notes/username/password/
//! remarks - no folders, favorites, item type, custom fields, TOTP, card hints, or password
//! history. This format carries all of it, so a vault can be exported and reconstructed without
//! silently dropping data. Uses the exact same sealed-envelope framing CSV export already
//! established (`portio::encode_sealed_csv`/`decode_sealed_bytes`) - this app's own file format,
//! not a standard container, so the UI copy must say plainly that only this app can open it.

use crate::crypto::VaultKey;
use crate::vault::CustomField;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

pub const JSON_EXPORT_VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct JsonExportHistoryEntry {
    pub secret: String,
    pub changed_at: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct JsonExportCredential {
    pub username: Option<String>,
    pub secret: String,
    pub remarks: Option<String>,
    pub custom_fields: Vec<CustomField>,
    pub reprompt_exempt: bool,
    pub totp_secret: Option<String>,
    pub history: Vec<JsonExportHistoryEntry>,
}

// Manual Debug so a stray `{:?}`/`dbg!()` on the whole export can never print plaintext secrets -
// same rule Credential/CredentialDto already follow, extended to every secret this format carries
// (the live password, every custom field value including Hidden ones, the TOTP seed, and every
// historical password).
impl std::fmt::Debug for JsonExportCredential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JsonExportCredential")
            .field("username", &self.username)
            .field("secret", &"<redacted>")
            .field("remarks", &self.remarks)
            .field("custom_fields", &format!("<{} field(s)>", self.custom_fields.len()))
            .field("reprompt_exempt", &self.reprompt_exempt)
            .field("totp_secret", &self.totp_secret.as_ref().map(|_| "<redacted>"))
            .field("history", &format!("<{} entrie(s)>", self.history.len()))
            .finish()
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct JsonExportSite {
    pub name: String,
    pub url: Option<String>,
    pub notes: Option<String>,
    pub favorite: bool,
    pub item_type: String,
    pub card_brand: Option<String>,
    pub card_last4: Option<String>,
    pub folder_name: Option<String>,
    pub credentials: Vec<JsonExportCredential>,
}

impl std::fmt::Debug for JsonExportSite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JsonExportSite")
            .field("name", &self.name)
            .field("credentials", &format!("<{} credential(s)>", self.credentials.len()))
            .finish()
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct JsonExportVault {
    pub version: u32,
    pub folders: Vec<String>,
    pub sites: Vec<JsonExportSite>,
}

impl std::fmt::Debug for JsonExportVault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JsonExportVault")
            .field("version", &self.version)
            .field("folders", &format!("<{} folder(s)>", self.folders.len()))
            .field("sites", &format!("<{} site(s)>", self.sites.len()))
            .finish()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum JsonExportError {
    #[error("vault error")]
    Vault(#[from] crate::vault::VaultError),
    #[error("could not serialize the export")]
    Serialize,
    #[error("that file doesn't look like a vault export")]
    Deserialize,
}

/// Builds the whole export in memory - decrypts every credential, custom field, TOTP secret, and
/// history entry (each individually sealed at rest) into one plaintext structure ready to be
/// serialized and re-sealed as a unit. Fine at personal-vault scale (a few hundred items); this is
/// the same tradeoff `vault::search` already documents for its own full-vault decrypt-and-scan.
pub fn build_json_export(conn: &Connection, key: &VaultKey) -> Result<JsonExportVault, JsonExportError> {
    let folders = crate::vault::list_folders(conn)?;
    let folder_name_by_id: std::collections::HashMap<i64, String> =
        folders.iter().map(|f| (f.id, f.name.clone())).collect();

    let mut sites = Vec::new();
    for site in crate::vault::list_sites(conn)? {
        let mut credentials = Vec::new();
        for c in crate::vault::list_credentials_for_site(conn, key, site.id)? {
            let totp_secret = crate::vault::get_totp_secret(conn, key, c.id)?;
            let history = crate::vault::list_credential_history(conn, key, c.id)?
                .into_iter()
                .map(|h| JsonExportHistoryEntry { secret: h.secret, changed_at: h.changed_at })
                .collect();
            credentials.push(JsonExportCredential {
                username: c.username,
                secret: c.secret,
                remarks: c.remarks,
                custom_fields: c.custom_fields,
                reprompt_exempt: c.reprompt_exempt,
                totp_secret,
                history,
            });
        }
        sites.push(JsonExportSite {
            name: site.name,
            url: site.url,
            notes: site.notes,
            favorite: site.favorite,
            item_type: site.item_type,
            card_brand: site.card_brand,
            card_last4: site.card_last4,
            folder_name: site.folder_id.and_then(|id| folder_name_by_id.get(&id).cloned()),
            credentials,
        });
    }

    Ok(JsonExportVault {
        version: JSON_EXPORT_VERSION,
        folders: folders.into_iter().map(|f| f.name).collect(),
        sites,
    })
}

pub fn export_to_json_bytes(export: &JsonExportVault) -> Result<Vec<u8>, JsonExportError> {
    serde_json::to_vec(export).map_err(|_| JsonExportError::Serialize)
}

pub fn parse_json_export(bytes: &[u8]) -> Result<JsonExportVault, JsonExportError> {
    serde_json::from_slice(bytes).map_err(|_| JsonExportError::Deserialize)
}

/// A quick, read-only summary for the import preview screen - counts plus which sites/credentials
/// would be skipped as duplicates, computed without writing anything (mirrors CSV import's
/// preview/commit split, so the user sees the outcome before it happens).
#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct JsonImportPreview {
    pub folder_count: usize,
    pub site_count: usize,
    pub credential_count: usize,
    pub duplicate_credential_count: usize,
}

/// One transaction for the whole file, same rollback-on-early-return shape `import_credentials`
/// already uses for CSV. Folder/site matching is by name (folders case-insensitively, matching
/// their own uniqueness constraint; sites exactly, matching CSV import's own precedent) - an
/// existing site's own metadata (favorite/folder/item type/card hint) is left untouched, only its
/// credential list is merged into, so re-importing an export you already partially have doesn't
/// clobber changes made since. A credential is a duplicate (skipped, counted, not an error) under
/// the exact same site-name + username rule CSV import uses.
pub fn import_json_vault(conn: &mut Connection, key: &VaultKey, export: &JsonExportVault) -> Result<crate::vault::ImportSummary, crate::vault::VaultError> {
    let tx = conn.transaction()?;
    let mut imported = 0usize;
    let mut duplicates = 0usize;

    for name in &export.folders {
        if crate::vault::find_folder_id_by_name(&tx, name)?.is_none() {
            // A blank/whitespace-only folder name in a hand-edited export file is skipped rather
            // than failing the whole import - clean_folder_name's own validation lives behind
            // add_folder, so this mirrors add_folder's error into a skip instead of a hard abort.
            let _ = crate::vault::add_folder(&tx, name);
        }
    }

    for site in &export.sites {
        let existing_site_id = crate::vault::find_site_id_by_name(&tx, &site.name)?;
        let site_id = match existing_site_id {
            Some(id) => id,
            None => {
                let id = crate::vault::add_site(&tx, &site.name, site.url.as_deref(), site.notes.as_deref())?;
                if site.favorite {
                    crate::vault::set_site_favorite(&tx, id, true)?;
                }
                if site.item_type != "login" {
                    crate::vault::set_site_item_type(&tx, id, &site.item_type)?;
                }
                if let Some(folder_name) = &site.folder_name {
                    if let Some(folder_id) = crate::vault::find_folder_id_by_name(&tx, folder_name)? {
                        crate::vault::set_site_folder(&tx, id, Some(folder_id))?;
                    }
                }
                if site.card_brand.is_some() || site.card_last4.is_some() {
                    // set_site_card_hint validates last4 is exactly 4 digits - a malformed value in
                    // a hand-edited export must not abort the whole import over one cosmetic hint.
                    let _ = crate::vault::set_site_card_hint(&tx, id, site.card_brand.as_deref(), site.card_last4.as_deref());
                }
                id
            }
        };

        for cred in &site.credentials {
            if crate::vault::credential_exists_for_import_check(&tx, site_id, cred.username.as_deref())? {
                duplicates += 1;
                continue;
            }
            // add_credential validates custom-field labels against the site's item type
            // (validate_fields_for_site) - a mismatch in a hand-edited or cross-version export
            // must not abort every other credential in the batch, so skip just this one.
            let credential_id = match crate::vault::add_credential(
                &tx, key, site_id, cred.username.as_deref(), &cred.secret, cred.remarks.as_deref(),
                &cred.custom_fields, cred.reprompt_exempt,
            ) {
                Ok(id) => id,
                Err(_) => continue,
            };
            if let Some(totp) = &cred.totp_secret {
                // vault::set_totp_secret itself does no validation (do_set_totp_secret validates
                // before calling it) - a TOTP secret that no longer decodes as base32 (hand-edited
                // file, future format change) must not fail the whole import, so validate here and
                // skip just the TOTP attach rather than erroring the batch or storing garbage.
                if crate::totp::generate_code(totp, 0).is_ok() {
                    let normalized = crate::totp::normalize_secret(totp);
                    let _ = crate::vault::set_totp_secret(&tx, key, credential_id, &normalized);
                }
            }
            for entry in &cred.history {
                let _ = crate::vault::insert_history_entry(&tx, key, credential_id, &entry.secret, &entry.changed_at);
            }
            imported += 1;
        }
    }

    tx.commit()?;
    Ok(crate::vault::ImportSummary { imported, duplicates })
}

pub fn preview_json_import(conn: &Connection, export: &JsonExportVault) -> Result<JsonImportPreview, crate::vault::VaultError> {
    let mut credential_count = 0usize;
    let mut duplicate_credential_count = 0usize;
    for site in &export.sites {
        let existing_site_id = crate::vault::find_site_id_by_name(conn, &site.name)?;
        for cred in &site.credentials {
            credential_count += 1;
            if let Some(site_id) = existing_site_id {
                if crate::vault::credential_exists_for_import_check(conn, site_id, cred.username.as_deref())? {
                    duplicate_credential_count += 1;
                }
            }
        }
    }
    Ok(JsonImportPreview {
        folder_count: export.folders.len(),
        site_count: export.sites.len(),
        credential_count,
        duplicate_credential_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::derive_key;
    use crate::vault::{add_credential, add_folder, add_site, open_vault, set_site_favorite, set_site_folder, set_totp_secret, CustomFieldType};
    use tempfile::tempdir;

    fn field(label: &str, value: &str, t: CustomFieldType) -> CustomField {
        CustomField { label: label.into(), value: value.into(), field_type: t }
    }

    #[test]
    fn build_then_parse_json_export_round_trips_every_field() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[1u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();

        let folder_id = add_folder(&conn, "Work").unwrap();
        let site_id = add_site(&conn, "example.com", Some("https://example.com"), Some("a note")).unwrap();
        set_site_favorite(&conn, site_id, true).unwrap();
        set_site_folder(&conn, site_id, Some(folder_id)).unwrap();
        let cred_id = add_credential(
            &conn, &key, site_id, Some("alice"), "s3cret", Some("remark"),
            &[field("Platform", "linux", CustomFieldType::Text)],
            true,
        ).unwrap();
        set_totp_secret(&conn, &key, cred_id, "JBSWY3DPEHPK3PXP").unwrap();
        // A second update creates a history entry for the original secret.
        crate::vault::update_credential(&conn, &key, cred_id, Some("alice"), "n3wsecret", Some("remark"), &[field("Platform", "linux", CustomFieldType::Text)], true).unwrap();

        let export = build_json_export(&conn, &key).unwrap();
        assert_eq!(export.folders, vec!["Work".to_string()]);
        assert_eq!(export.sites.len(), 1);
        let site = &export.sites[0];
        assert_eq!(site.name, "example.com");
        assert_eq!(site.favorite, true);
        assert_eq!(site.folder_name.as_deref(), Some("Work"));
        assert_eq!(site.credentials.len(), 1);
        let cred = &site.credentials[0];
        assert_eq!(cred.secret, "n3wsecret");
        assert_eq!(cred.reprompt_exempt, true);
        assert_eq!(cred.totp_secret.as_deref(), Some("JBSWY3DPEHPK3PXP"));
        assert_eq!(cred.history.len(), 1);
        assert_eq!(cred.history[0].secret, "s3cret");
        assert_eq!(cred.custom_fields.len(), 1);

        let bytes = export_to_json_bytes(&export).unwrap();
        let parsed = parse_json_export(&bytes).unwrap();
        assert_eq!(parsed, export);
    }

    #[test]
    fn parse_json_export_rejects_garbage() {
        assert!(parse_json_export(b"not json").is_err());
    }

    #[test]
    fn import_json_vault_reconstructs_folders_sites_and_every_credential_field() {
        let dir = tempdir().unwrap();
        let source_key = derive_key("source-pw", &[3u8; 16]).unwrap();
        let source_conn = open_vault(&dir.path().join("source.vault"), &source_key).unwrap();
        let folder_id = add_folder(&source_conn, "Work").unwrap();
        let site_id = add_site(&source_conn, "example.com", Some("https://example.com"), None).unwrap();
        set_site_favorite(&source_conn, site_id, true).unwrap();
        set_site_folder(&source_conn, site_id, Some(folder_id)).unwrap();
        let cred_id = add_credential(
            &source_conn, &source_key, site_id, Some("alice"), "s3cret", None,
            &[field("Platform", "linux", CustomFieldType::Text)], true,
        ).unwrap();
        set_totp_secret(&source_conn, &source_key, cred_id, "JBSWY3DPEHPK3PXP").unwrap();
        let export = build_json_export(&source_conn, &source_key).unwrap();

        // Import into a DIFFERENT vault under a DIFFERENT key - this is the actual portability use
        // case (moving data between vaults), not just a round trip against the source.
        let target_key = derive_key("target-pw", &[4u8; 16]).unwrap();
        let mut target_conn = open_vault(&dir.path().join("target.vault"), &target_key).unwrap();

        let summary = import_json_vault(&mut target_conn, &target_key, &export).unwrap();
        assert_eq!(summary.imported, 1);
        assert_eq!(summary.duplicates, 0);

        let folders = crate::vault::list_folders(&target_conn).unwrap();
        assert_eq!(folders.len(), 1);
        assert_eq!(folders[0].name, "Work");

        let sites = crate::vault::list_sites(&target_conn).unwrap();
        assert_eq!(sites.len(), 1);
        assert_eq!(sites[0].name, "example.com");
        assert_eq!(sites[0].favorite, true);
        assert_eq!(sites[0].folder_id, Some(folders[0].id));

        let creds = crate::vault::list_credentials_for_site(&target_conn, &target_key, sites[0].id).unwrap();
        assert_eq!(creds.len(), 1);
        assert_eq!(creds[0].secret, "s3cret");
        assert_eq!(creds[0].reprompt_exempt, true);
        assert_eq!(creds[0].custom_fields.len(), 1);
        let totp = crate::vault::get_totp_secret(&target_conn, &target_key, creds[0].id).unwrap();
        assert_eq!(totp.as_deref(), Some("JBSWY3DPEHPK3PXP"));

        // Re-importing the same export merges (skips the duplicate credential) instead of
        // creating a second copy or erroring - matches CSV import's own duplicate contract.
        let second = import_json_vault(&mut target_conn, &target_key, &export).unwrap();
        assert_eq!(second.imported, 0);
        assert_eq!(second.duplicates, 1);
        assert_eq!(crate::vault::list_sites(&target_conn).unwrap().len(), 1);
    }

    #[test]
    fn preview_json_import_counts_sites_credentials_and_duplicates() {
        let dir = tempdir().unwrap();
        let key = derive_key("pw", &[2u8; 16]).unwrap();
        let conn = open_vault(&dir.path().join("t.vault"), &key).unwrap();
        let site_id = add_site(&conn, "existing.com", None, None).unwrap();
        add_credential(&conn, &key, site_id, Some("carol"), "pw", None, &[], false).unwrap();

        let export = JsonExportVault {
            version: JSON_EXPORT_VERSION,
            folders: vec!["Work".into()],
            sites: vec![
                JsonExportSite {
                    name: "existing.com".into(), url: None, notes: None, favorite: false,
                    item_type: "login".into(), card_brand: None, card_last4: None, folder_name: None,
                    credentials: vec![JsonExportCredential {
                        username: Some("carol".into()), secret: "pw".into(), remarks: None,
                        custom_fields: vec![], reprompt_exempt: false, totp_secret: None, history: vec![],
                    }],
                },
                JsonExportSite {
                    name: "new.com".into(), url: None, notes: None, favorite: false,
                    item_type: "login".into(), card_brand: None, card_last4: None, folder_name: None,
                    credentials: vec![JsonExportCredential {
                        username: Some("dave".into()), secret: "pw2".into(), remarks: None,
                        custom_fields: vec![], reprompt_exempt: false, totp_secret: None, history: vec![],
                    }],
                },
            ],
        };

        let preview = preview_json_import(&conn, &export).unwrap();
        assert_eq!(preview.folder_count, 1);
        assert_eq!(preview.site_count, 2);
        assert_eq!(preview.credential_count, 2);
        assert_eq!(preview.duplicate_credential_count, 1);
    }
}
