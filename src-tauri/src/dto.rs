use pwmgr_core::vault::{Credential, SearchResult, Site, TrashedCredential};
use serde::Serialize;

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
// stray `{:?}` or `dbg!()` on the Tauri side either.
#[derive(Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CredentialDto {
    pub id: i64,
    pub site_id: i64,
    pub username: Option<String>,
    pub secret: String,
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
            secret: c.secret,
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
}
