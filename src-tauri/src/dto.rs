use pwmgr_core::vault::{Credential, SearchResult, Site, TrashedCredential};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use zeroize::Zeroizing;

#[derive(Debug, Serialize, PartialEq)]
pub struct AuthSuccess {
    pub username: String,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SiteDto {
    pub id: i64,
    pub name: String,
    pub url: Option<String>,
    pub notes: Option<String>,
}

impl From<Site> for SiteDto {
    fn from(s: Site) -> Self {
        SiteDto { id: s.id, name: s.name, url: s.url, notes: s.notes }
    }
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SearchResultDto {
    pub site_id: i64,
    pub site_name: String,
    pub credential_id: i64,
    pub username: Option<String>,
}

impl From<SearchResult> for SearchResultDto {
    fn from(r: SearchResult) -> Self {
        SearchResultDto {
            site_id: r.site_id,
            site_name: r.site_name,
            credential_id: r.credential_id,
            username: r.username,
        }
    }
}

// No #[derive(Debug)] here on purpose - mirrors pwmgr_core::vault::Credential's manual redacting
// Debug impl (core/src/vault.rs:139) so this DTO can never leak the plaintext secret through a
// stray `{:?}` or `dbg!()` on the Tauri side either. This matters more than it looks: Zeroizing<Z>
// itself derives Debug and prints the plaintext value unredacted, so this manual impl is the only
// thing preventing a leak - a `#[derive(Debug)]` here would silently defeat the wrapper below.
//
// `secret` is wrapped in `zeroize::Zeroizing<String>` since this DTO is the last consumer before
// the plaintext crosses the IPC boundary to the UI - the last real chance to zeroize on Drop.
// Residual gaps (neither closable at this layer):
// 1. `pwmgr_core::vault::Credential.secret` (upstream of this DTO) is still a plain `String`, so
//    the plaintext exists un-zeroized for the brief window between `list_credentials_for_site`
//    returning it and this `From` conversion wrapping it. Closing that fully would mean changing
//    `Credential`'s field type in `pwmgr-core` itself, which is out of scope for this DTO-only
//    task; tracked as a further-tightening opportunity (see Task 28).
// 2. Serializing this DTO (serde_json, on the way over Tauri IPC) copies the plaintext into the
//    serializer's own non-zeroizing output buffer, and the resulting JS string in the WebView is
//    never zeroizable either - `zeroize` can't reach into serde_json's buffer or the JS runtime.
//    Already covered by THREAT_MODEL.md's out-of-scope adversary O1 and Residual Risk #1
//    (credentials in active use are exposed); not a gap this task can close.
#[derive(Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CredentialDto {
    pub id: i64,
    pub site_id: i64,
    pub username: Option<String>,
    pub secret: Zeroizing<String>,
    pub remarks: Option<String>,
}

// If you add a field to CredentialDto, add it here too - a manual Debug impl gets no compiler
// warning for a field the derive macro would otherwise cover automatically.
impl std::fmt::Debug for CredentialDto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CredentialDto")
            .field("id", &self.id)
            .field("site_id", &self.site_id)
            .field("username", &self.username)
            .field("secret", &"<redacted>")
            .field("remarks", &self.remarks)
            .finish()
    }
}

impl From<Credential> for CredentialDto {
    fn from(c: Credential) -> Self {
        CredentialDto {
            id: c.id,
            site_id: c.site_id,
            username: c.username,
            secret: Zeroizing::new(c.secret),
            remarks: c.remarks,
        }
    }
}

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

// #[derive(Debug)] is safe here - neither field is secret (unlike CredentialDto above, which
// redacts manually).
#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConfigDto {
    pub theme: String,
    pub auto_lock_minutes: u32,
    pub strength_algorithm: String,
}

// Non-secret score/label output - normal `Debug` derive is fine (unlike CredentialDto above).
#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StrengthDto {
    pub score: u8,           // zxcvbn 0..=4
    pub guesses_log10: f64,  // log10(estimated guesses)
    pub label: String,       // "Very weak" .. "Very strong"
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
    use pwmgr_core::vault::{Credential, SearchResult, Site};

    #[test]
    fn site_dto_from_core_site_maps_all_fields() {
        let site = Site { id: 1, name: "example.com".into(), url: Some("https://example.com".into()), notes: None };
        let dto = SiteDto::from(site);
        assert_eq!(dto.id, 1);
        assert_eq!(dto.name, "example.com");
        assert_eq!(dto.url.as_deref(), Some("https://example.com"));
    }

    #[test]
    fn search_result_dto_from_core_maps_all_fields() {
        let result = SearchResult { site_id: 1, site_name: "example.com".into(), credential_id: 2, username: Some("alice".into()) };
        let dto = SearchResultDto::from(result);
        assert_eq!(dto.credential_id, 2);
        assert_eq!(dto.username.as_deref(), Some("alice"));
    }

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
    fn credential_dto_debug_format_redacts_secret() {
        let cred = Credential {
            id: 1,
            site_id: 1,
            username: Some("alice".into()),
            secret: "extremely-unique-secret-xyz".into(),
            remarks: None,
        };
        let dto = CredentialDto::from(cred);
        let debug_output = format!("{:?}", dto);
        assert!(!debug_output.contains("extremely-unique-secret-xyz"));
        assert!(debug_output.contains("<redacted>"));
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

    #[test]
    fn credential_dto_secret_is_wrapped_in_zeroizing() {
        let cred = Credential {
            id: 1,
            site_id: 1,
            username: Some("alice".into()),
            secret: "s3cret".into(),
            remarks: None,
        };
        let dto = CredentialDto::from(cred);
        // Zeroizing<String> derefs to &str, so equality/usage is transparent to callers - this test
        // just documents the type-level guarantee (zeroizes on Drop) rather than trying to observe
        // memory after drop, which isn't practical in safe Rust.
        let secret_str: &str = &dto.secret;
        assert_eq!(secret_str, "s3cret");
    }
}
