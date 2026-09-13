use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum PortioError {
    #[error("could not write CSV data")]
    Write,
    #[error("could not parse CSV data")]
    Parse,
    #[error("CSV header does not match the expected {0} format")]
    HeaderMismatch(&'static str),
    #[error("column '{0}' was not found in the CSV header")]
    MissingColumn(String),
    #[error("'{0}' is not a recognized import field")]
    UnknownField(String),
}

#[derive(Clone, PartialEq)]
pub struct ImportRecord {
    pub site_name: String,
    pub url: Option<String>,
    pub notes: Option<String>,
    pub username: Option<String>,
    pub secret: String,
    pub remarks: Option<String>,
}

// Manual Debug impl (not derived) so a stray `{:?}`/`dbg!()` can never print the plaintext secret
// read from an imported CSV row - same rationale and pattern as vault::Credential.
impl std::fmt::Debug for ImportRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImportRecord")
            .field("site_name", &self.site_name)
            .field("url", &self.url)
            .field("notes", &self.notes)
            .field("username", &self.username)
            .field("secret", &"<redacted>")
            .field("remarks", &self.remarks)
            .finish()
    }
}

fn non_empty(s: Option<&str>) -> Option<String> {
    match s {
        Some(v) if !v.is_empty() => Some(v.to_string()),
        _ => None,
    }
}

const CHROMIUM_HEADER: [&str; 4] = ["name", "url", "username", "password"];

pub fn parse_chromium_csv(csv_text: &str) -> Result<Vec<ImportRecord>, PortioError> {
    let mut reader = csv::Reader::from_reader(csv_text.as_bytes());
    let headers = reader.headers().map_err(|_| PortioError::Parse)?.clone();
    if headers.iter().collect::<Vec<_>>() != CHROMIUM_HEADER.to_vec() {
        return Err(PortioError::HeaderMismatch("Chromium"));
    }
    let mut out = Vec::new();
    for result in reader.records() {
        let record = result.map_err(|_| PortioError::Parse)?;
        out.push(ImportRecord {
            site_name: record.get(0).unwrap_or("").to_string(),
            url: non_empty(record.get(1)),
            notes: None,
            username: non_empty(record.get(2)),
            secret: record.get(3).unwrap_or("").to_string(),
            remarks: None,
        });
    }
    Ok(out)
}

pub struct CsvRow {
    pub site_name: String,
    pub url: Option<String>,
    pub notes: Option<String>,
    pub username: Option<String>,
    pub secret: String,
    pub remarks: Option<String>,
}

pub fn rows_to_csv(rows: &[CsvRow]) -> Result<String, PortioError> {
    let mut writer = csv::WriterBuilder::new().from_writer(vec![]);
    writer
        .write_record(["site_name", "url", "notes", "username", "secret", "remarks"])
        .map_err(|_| PortioError::Write)?;
    for r in rows {
        writer
            .write_record([
                r.site_name.as_str(),
                r.url.as_deref().unwrap_or(""),
                r.notes.as_deref().unwrap_or(""),
                r.username.as_deref().unwrap_or(""),
                r.secret.as_str(),
                r.remarks.as_deref().unwrap_or(""),
            ])
            .map_err(|_| PortioError::Write)?;
    }
    let bytes = writer.into_inner().map_err(|_| PortioError::Write)?;
    String::from_utf8(bytes).map_err(|_| PortioError::Write)
}

const EXPORT_VERSION: u8 = 1;
const MODE_THIS_DEVICE: u8 = 0;
const MODE_WITH_PASSWORD: u8 = 1;

/// Frames a sealed CSV export. `salt` is `Some` only for the with-password mode (the this-device
/// mode derives no new key, so there is no salt to embed). This is our own file format, not a
/// standard container - the UI copy must say so plainly ("only this app can open this file").
// ponytail: no decoder; add one only if a future milestone needs to re-import this format.
pub fn encode_sealed_csv(salt: Option<&[u8; 16]>, nonce: &[u8; 12], ciphertext: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.push(EXPORT_VERSION);
    match salt {
        Some(s) => {
            out.push(MODE_WITH_PASSWORD);
            out.extend_from_slice(s);
        }
        None => out.push(MODE_THIS_DEVICE),
    }
    out.extend_from_slice(nonce);
    out.extend_from_slice(&(ciphertext.len() as u32).to_le_bytes());
    out.extend_from_slice(ciphertext);
    out
}

const FIREFOX_HEADER: [&str; 9] = [
    "url", "username", "password", "httpRealm", "formActionOrigin",
    "guid", "timeCreated", "timeLastUsed", "timePasswordChanged",
];

pub fn parse_firefox_csv(csv_text: &str) -> Result<Vec<ImportRecord>, PortioError> {
    let mut reader = csv::Reader::from_reader(csv_text.as_bytes());
    let headers = reader.headers().map_err(|_| PortioError::Parse)?.clone();
    if headers.iter().collect::<Vec<_>>() != FIREFOX_HEADER.to_vec() {
        return Err(PortioError::HeaderMismatch("Firefox"));
    }
    let mut out = Vec::new();
    for result in reader.records() {
        let record = result.map_err(|_| PortioError::Parse)?;
        let url = record.get(0).unwrap_or("").to_string();
        out.push(ImportRecord {
            site_name: url.clone(),
            url: non_empty(Some(url.as_str())),
            notes: None,
            username: non_empty(record.get(1)),
            secret: record.get(2).unwrap_or("").to_string(),
            remarks: None,
        });
    }
    Ok(out)
}

const LASTPASS_HEADER: [&str; 8] =
    ["url", "username", "password", "totp", "extra", "name", "grouping", "fav"];

/// LastPass's standard "Export to CSV" format. `name` (LastPass's item title, which can differ
/// from the URL) is preferred as the site name when present; `extra` is LastPass's free-text notes
/// field, mapped straight to `notes`.
// ponytail: `totp` (LastPass's own 2FA seed column) isn't threaded into the new TOTP field yet -
// add a totp_secret param here and to do_import_csv_commit's row handling if imported TOTP seeds
// are wanted; skipped for now since the CSV import path predates that feature and this keeps the
// two changes independent.
pub fn parse_lastpass_csv(csv_text: &str) -> Result<Vec<ImportRecord>, PortioError> {
    let mut reader = csv::Reader::from_reader(csv_text.as_bytes());
    let headers = reader.headers().map_err(|_| PortioError::Parse)?.clone();
    if headers.iter().collect::<Vec<_>>() != LASTPASS_HEADER.to_vec() {
        return Err(PortioError::HeaderMismatch("LastPass"));
    }
    let mut out = Vec::new();
    for result in reader.records() {
        let record = result.map_err(|_| PortioError::Parse)?;
        let url = record.get(0).unwrap_or("");
        let name = record.get(5).unwrap_or("");
        out.push(ImportRecord {
            site_name: if !name.is_empty() { name.to_string() } else { url.to_string() },
            url: non_empty(Some(url)),
            notes: non_empty(record.get(4)),
            username: non_empty(record.get(1)),
            secret: record.get(2).unwrap_or("").to_string(),
            remarks: None,
        });
    }
    Ok(out)
}

use std::collections::HashMap;

const IMPORT_FIELDS: [&str; 6] = ["site_name", "url", "notes", "username", "secret", "remarks"];

pub fn parse_generic_csv(
    csv_text: &str,
    mapping: &HashMap<String, String>,
) -> Result<Vec<ImportRecord>, PortioError> {
    let mut reader = csv::Reader::from_reader(csv_text.as_bytes());
    let headers = reader.headers().map_err(|_| PortioError::Parse)?.clone();

    // Resolve each mapped CSV column to its index once, up front - fails fast on a bad mapping
    // before touching a single data row.
    let mut resolved: Vec<(usize, String)> = Vec::new();
    for (csv_col, field) in mapping {
        if !IMPORT_FIELDS.contains(&field.as_str()) {
            return Err(PortioError::UnknownField(field.clone()));
        }
        let idx = headers
            .iter()
            .position(|h| h == csv_col)
            .ok_or_else(|| PortioError::MissingColumn(csv_col.clone()))?;
        resolved.push((idx, field.clone()));
    }

    let mut out = Vec::new();
    for result in reader.records() {
        let record = result.map_err(|_| PortioError::Parse)?;
        let mut rec = ImportRecord {
            site_name: String::new(),
            url: None,
            notes: None,
            username: None,
            secret: String::new(),
            remarks: None,
        };
        for (idx, field) in &resolved {
            let value = record.get(*idx).unwrap_or("").to_string();
            match field.as_str() {
                "site_name" => rec.site_name = value,
                "url" => rec.url = non_empty(Some(value.as_str())),
                "notes" => rec.notes = non_empty(Some(value.as_str())),
                "username" => rec.username = non_empty(Some(value.as_str())),
                "secret" => rec.secret = value,
                "remarks" => rec.remarks = non_empty(Some(value.as_str())),
                _ => unreachable!("validated against IMPORT_FIELDS above"),
            }
        }
        out.push(rec);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_record_debug_format_redacts_secret() {
        let record = ImportRecord {
            site_name: "example.com".to_string(),
            url: None,
            notes: None,
            username: Some("alice".to_string()),
            secret: "extremely-unique-imported-secret-xyz".to_string(),
            remarks: None,
        };
        let debug_output = format!("{:?}", record);
        assert!(!debug_output.contains("extremely-unique-imported-secret-xyz"));
        assert!(debug_output.contains("<redacted>"));
    }

    fn sample_rows() -> Vec<CsvRow> {
        vec![
            CsvRow {
                site_name: "example.com".to_string(),
                url: Some("https://example.com".to_string()),
                notes: None,
                username: Some("alice".to_string()),
                secret: "s3cret!".to_string(),
                remarks: Some("work, personal".to_string()), // comma - must be quoted
            },
        ]
    }

    #[test]
    fn rows_to_csv_writes_a_header_and_one_row_per_credential() {
        let csv = rows_to_csv(&sample_rows()).unwrap();
        let mut lines = csv.lines();

        assert_eq!(lines.next().unwrap(), "site_name,url,notes,username,secret,remarks");
        assert!(lines.next().unwrap().contains("example.com"));
        assert_eq!(lines.next(), None);
    }

    #[test]
    fn rows_to_csv_quotes_a_field_containing_a_comma() {
        let csv = rows_to_csv(&sample_rows()).unwrap();

        assert!(csv.contains("\"work, personal\""));
    }

    #[test]
    fn rows_to_csv_writes_empty_string_for_none_fields() {
        let rows = vec![CsvRow {
            site_name: "bare.com".to_string(),
            url: None,
            notes: None,
            username: None,
            secret: "x".to_string(),
            remarks: None,
        }];

        let csv = rows_to_csv(&rows).unwrap();

        assert_eq!(csv.lines().nth(1).unwrap(), "bare.com,,,,x,");
    }

    #[test]
    fn encode_sealed_csv_this_device_mode_has_no_salt_bytes() {
        let nonce = [7u8; 12];
        let ciphertext = vec![1, 2, 3];

        let out = encode_sealed_csv(None, &nonce, &ciphertext);

        assert_eq!(out[0], 1); // version
        assert_eq!(out[1], 0); // mode: this-device
        assert_eq!(&out[2..14], &nonce); // nonce follows immediately, no salt in between
        let len = u32::from_le_bytes(out[14..18].try_into().unwrap()) as usize;
        assert_eq!(len, 3);
        assert_eq!(&out[18..18 + len], &ciphertext[..]);
    }

    #[test]
    fn encode_sealed_csv_with_password_mode_includes_the_salt() {
        let salt = [9u8; 16];
        let nonce = [7u8; 12];
        let ciphertext = vec![4, 5];

        let out = encode_sealed_csv(Some(&salt), &nonce, &ciphertext);

        assert_eq!(out[1], 1); // mode: with-password
        assert_eq!(&out[2..18], &salt);
        assert_eq!(&out[18..30], &nonce);
    }

    #[test]
    fn parse_chromium_csv_maps_the_four_fixed_columns() {
        let csv = "name,url,username,password\nexample.com,https://example.com,alice,s3cret\n";

        let records = parse_chromium_csv(csv).unwrap();

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].site_name, "example.com");
        assert_eq!(records[0].url.as_deref(), Some("https://example.com"));
        assert_eq!(records[0].username.as_deref(), Some("alice"));
        assert_eq!(records[0].secret, "s3cret");
    }

    #[test]
    fn parse_chromium_csv_rejects_an_unexpected_header() {
        let csv = "site,link,user,pass\na,b,c,d\n";

        let result = parse_chromium_csv(csv);

        assert!(matches!(result, Err(PortioError::HeaderMismatch("Chromium"))));
    }

    #[test]
    fn parse_chromium_csv_treats_an_empty_url_as_none() {
        let csv = "name,url,username,password\nexample.com,,alice,s3cret\n";

        let records = parse_chromium_csv(csv).unwrap();

        assert_eq!(records[0].url, None);
    }

    const FIREFOX_SAMPLE_HEADER: &str =
        "url,username,password,httpRealm,formActionOrigin,guid,timeCreated,timeLastUsed,timePasswordChanged";

    #[test]
    fn parse_firefox_csv_uses_the_url_as_the_site_name() {
        let csv = format!("{}\nhttps://example.com,alice,s3cret,,,g1,1,2,3\n", FIREFOX_SAMPLE_HEADER);

        let records = parse_firefox_csv(&csv).unwrap();

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].site_name, "https://example.com");
        assert_eq!(records[0].url.as_deref(), Some("https://example.com"));
        assert_eq!(records[0].username.as_deref(), Some("alice"));
        assert_eq!(records[0].secret, "s3cret");
    }

    #[test]
    fn parse_firefox_csv_rejects_an_unexpected_header() {
        let csv = "url,user,pass\na,b,c\n";

        let result = parse_firefox_csv(csv);

        assert!(matches!(result, Err(PortioError::HeaderMismatch("Firefox"))));
    }

    const LASTPASS_SAMPLE_HEADER: &str = "url,username,password,totp,extra,name,grouping,fav";

    #[test]
    fn parse_lastpass_csv_prefers_the_name_column_as_site_name() {
        let csv = format!(
            "{}\nhttps://example.com,alice,s3cret,,some notes,My Example Login,Work,0\n",
            LASTPASS_SAMPLE_HEADER
        );

        let records = parse_lastpass_csv(&csv).unwrap();

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].site_name, "My Example Login");
        assert_eq!(records[0].url.as_deref(), Some("https://example.com"));
        assert_eq!(records[0].username.as_deref(), Some("alice"));
        assert_eq!(records[0].secret, "s3cret");
        assert_eq!(records[0].notes.as_deref(), Some("some notes"));
    }

    #[test]
    fn parse_lastpass_csv_falls_back_to_url_when_name_is_empty() {
        let csv = format!("{}\nhttps://example.com,alice,s3cret,,,,,0\n", LASTPASS_SAMPLE_HEADER);

        let records = parse_lastpass_csv(&csv).unwrap();

        assert_eq!(records[0].site_name, "https://example.com");
    }

    #[test]
    fn parse_lastpass_csv_rejects_an_unexpected_header() {
        let csv = "url,user,pass\na,b,c\n";

        let result = parse_lastpass_csv(csv);

        assert!(matches!(result, Err(PortioError::HeaderMismatch("LastPass"))));
    }

    use std::collections::HashMap;

    #[test]
    fn parse_generic_csv_applies_the_column_mapping() {
        let csv = "Site,Login,Pwd,Comment\nexample.com,alice,s3cret,work login\n";
        let mut mapping = HashMap::new();
        mapping.insert("Site".to_string(), "site_name".to_string());
        mapping.insert("Login".to_string(), "username".to_string());
        mapping.insert("Pwd".to_string(), "secret".to_string());
        mapping.insert("Comment".to_string(), "remarks".to_string());

        let records = parse_generic_csv(csv, &mapping).unwrap();

        assert_eq!(records[0].site_name, "example.com");
        assert_eq!(records[0].username.as_deref(), Some("alice"));
        assert_eq!(records[0].secret, "s3cret");
        assert_eq!(records[0].remarks.as_deref(), Some("work login"));
        assert_eq!(records[0].url, None); // not in the mapping - left at default
    }

    #[test]
    fn parse_generic_csv_rejects_a_mapped_column_that_does_not_exist_in_the_header() {
        let csv = "Site,Pwd\nexample.com,s3cret\n";
        let mut mapping = HashMap::new();
        mapping.insert("Site".to_string(), "site_name".to_string());
        mapping.insert("NoSuchColumn".to_string(), "secret".to_string());

        let result = parse_generic_csv(csv, &mapping);

        assert!(matches!(result, Err(PortioError::MissingColumn(_))));
    }

    #[test]
    fn parse_generic_csv_rejects_an_unrecognized_target_field_name() {
        let csv = "Site\nexample.com\n";
        let mut mapping = HashMap::new();
        mapping.insert("Site".to_string(), "not_a_real_field".to_string());

        let result = parse_generic_csv(csv, &mapping);

        assert!(matches!(result, Err(PortioError::UnknownField(_))));
    }
}
