use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum PortioError {
    #[error("could not write CSV data")]
    Write,
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
