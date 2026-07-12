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
}
