use pwmgr_core::vault::TrashedCredential;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use zeroize::Zeroizing;

// No secret field here at all (matches `list_trash`'s core signature - nothing to redact).
#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TrashedCredentialDto {
    pub credential_id: i64,
    pub site_id: i64,
    pub site_name: String,
    pub username: Option<String>,
    pub deleted_at: String,
}

impl From<TrashedCredential> for TrashedCredentialDto {
    fn from(t: TrashedCredential) -> Self {
        TrashedCredentialDto {
            credential_id: t.credential_id,
            site_id: t.site_id,
            site_name: t.site_name,
            username: t.username,
            deleted_at: t.deleted_at,
        }
    }
}

// Non-secret score/label output - normal `Debug` derive is fine (unlike CredentialDto above).
#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StrengthDto {
    pub score: u8,           // zxcvbn 0..=4
    pub guesses_log10: f64,  // log10(estimated guesses)
    pub label: String,       // "Very weak" .. "Very strong"
}

// None of these carry a secret - plain `#[derive(Debug)]` is safe, same reasoning as
// SiteDto/TrashedCredentialDto above.
#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HealthEntryDto {
    pub credential_id: i64,
    pub site_id: i64,
    pub site_name: String,
    pub username: Option<String>,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReusedGroupDto {
    pub entries: Vec<HealthEntryDto>,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HealthReportDto {
    pub weak: Vec<HealthEntryDto>,
    pub reused: Vec<ReusedGroupDto>,
    pub old: Vec<HealthEntryDto>,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BreachReportDto {
    pub breached: Vec<HealthEntryDto>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", tag = "method")]
pub enum GenerateRequest {
    Random { length: usize, uppercase: bool, lowercase: bool, digits: bool, symbols: bool },
    Memorable { words: usize, separator: String },
    Pattern { template: String },
    Pronounceable { length: usize },
}

// No #[derive(Debug)] here on purpose - `WithPassword` carries a plaintext export password,
// same rationale as CredentialDto above. Manual impl redacts it.
#[derive(Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", tag = "mode")]
pub enum ExportMode {
    ThisDevice,
    WithPassword { password: String },
    Plaintext,
}

impl std::fmt::Debug for ExportMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExportMode::ThisDevice => f.write_str("ThisDevice"),
            ExportMode::WithPassword { .. } => f
                .debug_struct("WithPassword")
                .field("password", &"<redacted>")
                .finish(),
            ExportMode::Plaintext => f.write_str("Plaintext"),
        }
    }
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", tag = "format")]
pub enum ImportFormat {
    Chromium,
    Firefox,
    Generic { mapping: HashMap<String, String> },
}

// Manual Debug impl for the same reason as CredentialDto (dto.rs above): `secret` is plaintext
// crossing IPC, and this DTO is serialized/deserialized in bulk (a whole import batch), so a
// stray `{:?}`/`dbg!()` on a `Vec<ImportRecordDto>` must never print N secrets at once.
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ImportRecordDto {
    pub site_name: String,
    pub url: Option<String>,
    pub notes: Option<String>,
    pub username: Option<String>,
    pub secret: Zeroizing<String>,
    pub remarks: Option<String>,
}

impl std::fmt::Debug for ImportRecordDto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImportRecordDto")
            .field("site_name", &self.site_name)
            .field("url", &self.url)
            .field("notes", &self.notes)
            .field("username", &self.username)
            .field("secret", &"<redacted>")
            .field("remarks", &self.remarks)
            .finish()
    }
}

impl From<pwmgr_core::portio::ImportRecord> for ImportRecordDto {
    fn from(r: pwmgr_core::portio::ImportRecord) -> Self {
        ImportRecordDto {
            site_name: r.site_name,
            url: r.url,
            notes: r.notes,
            username: r.username,
            secret: Zeroizing::new(r.secret),
            remarks: r.remarks,
        }
    }
}

impl From<ImportRecordDto> for pwmgr_core::portio::ImportRecord {
    fn from(d: ImportRecordDto) -> Self {
        pwmgr_core::portio::ImportRecord {
            site_name: d.site_name,
            url: d.url,
            notes: d.notes,
            username: d.username,
            secret: (*d.secret).clone(),
            remarks: d.remarks,
        }
    }
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummaryDto {
    pub imported: usize,
    pub duplicates: usize,
}

impl From<pwmgr_core::vault::ImportSummary> for ImportSummaryDto {
    fn from(s: pwmgr_core::vault::ImportSummary) -> Self {
        ImportSummaryDto { imported: s.imported, duplicates: s.duplicates }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_request_deserializes_from_the_frontend_s_camelcase_tag_json() {
        // Guards the Rust-variant-name <-> frontend-literal coupling: serde's `rename_all =
        // "camelCase"` lowercases these PascalCase variant names for the "method" tag, and the TS
        // union in src/lib/api.ts hardcodes the lowercase strings independently. Nothing else
        // catches a drift between the two sides except this round trip through real JSON.
        let random: GenerateRequest = serde_json::from_str(
            r#"{"method":"random","length":20,"uppercase":true,"lowercase":true,"digits":true,"symbols":false}"#,
        ).unwrap();
        assert_eq!(random, GenerateRequest::Random { length: 20, uppercase: true, lowercase: true, digits: true, symbols: false });

        let memorable: GenerateRequest =
            serde_json::from_str(r#"{"method":"memorable","words":4,"separator":"-"}"#).unwrap();
        assert_eq!(memorable, GenerateRequest::Memorable { words: 4, separator: "-".into() });

        let pattern: GenerateRequest =
            serde_json::from_str(r#"{"method":"pattern","template":"Cvcv-####"}"#).unwrap();
        assert_eq!(pattern, GenerateRequest::Pattern { template: "Cvcv-####".into() });

        let pronounceable: GenerateRequest =
            serde_json::from_str(r#"{"method":"pronounceable","length":12}"#).unwrap();
        assert_eq!(pronounceable, GenerateRequest::Pronounceable { length: 12 });

        // PascalCase tag values (what you'd get without `rename_all` applying to the tag) must NOT
        // parse - this is the failure mode a Rust-side variant rename could silently reintroduce.
        assert!(serde_json::from_str::<GenerateRequest>(r#"{"method":"Random","length":20,"uppercase":true,"lowercase":true,"digits":true,"symbols":false}"#).is_err());
    }

    #[test]
    fn export_mode_debug_format_redacts_password() {
        let mode = ExportMode::WithPassword { password: "extremely-unique-export-pw-xyz".into() };
        let debug_output = format!("{:?}", mode);
        assert!(!debug_output.contains("extremely-unique-export-pw-xyz"));
        assert!(debug_output.contains("<redacted>"));
    }

    #[test]
    fn import_record_dto_debug_format_redacts_secret() {
        let dto = ImportRecordDto {
            site_name: "example.com".to_string(),
            url: None,
            notes: None,
            username: Some("alice".into()),
            secret: Zeroizing::new("extremely-unique-imported-secret-xyz".to_string()),
            remarks: None,
        };
        let debug_output = format!("{:?}", dto);
        assert!(!debug_output.contains("extremely-unique-imported-secret-xyz"));
        assert!(debug_output.contains("<redacted>"));
    }
}
