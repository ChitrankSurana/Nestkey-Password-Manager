use crate::vault::{Credential, SearchResult, Site};
use serde::Serialize;
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
// Debug impl (core/src/vault.rs) so this DTO can never leak the plaintext secret through a
// stray `{:?}` or `dbg!()` on the consuming side either. This matters more than it looks: Zeroizing<Z>
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
//    task; tracked as a further-tightening opportunity.
// 2. Serializing this DTO (serde_json, on the way over IPC) copies the plaintext into the
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

// #[derive(Debug)] is safe here - neither field is secret (unlike CredentialDto above, which
// redacts manually).
#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConfigDto {
    pub theme: String,
    pub auto_lock_minutes: u32,
    pub strength_algorithm: String,
    pub accent_color: String,
    pub old_password_days: u32,
    pub hibp_opt_in: bool,
    pub nav_collapsed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::{Credential, SearchResult, Site};

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
