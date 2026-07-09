use serde::Serialize;

/// Safe error DTO serialized across the Tauri IPC boundary. `pwmgr-core`'s error types carry
/// internal detail (e.g. raw SQLite error text) that must never reach the frontend verbatim -
/// per THREAT_MODEL, that would leak implementation details and risks creating an information
/// oracle (e.g. distinguishing "wrong password" from "unknown username" from "database
/// corrupted"). Every `From` conversion here collapses internal error variants down to one of a
/// small set of generic, user-facing `(code, message)` pairs.
#[derive(Debug, Serialize)]
pub struct CommandError {
    pub code: String,
    pub message: String,
}

impl CommandError {
    fn new(code: &str, message: &str) -> Self {
        CommandError {
            code: code.to_string(),
            message: message.to_string(),
        }
    }
}

impl From<pwmgr_core::auth::AuthError> for CommandError {
    fn from(e: pwmgr_core::auth::AuthError) -> Self {
        use pwmgr_core::auth::AuthError;
        match e {
            // Single generic message on any auth failure - TRD §3.2 / THREAT_MODEL wrong-password
            // oracle: no distinguishing signal about which of username/password/vault was wrong.
            AuthError::InvalidCredentials => {
                CommandError::new("invalid_credentials", "Incorrect master password.")
            }
            AuthError::UsernameTaken => {
                CommandError::new("username_taken", "That username is already taken.")
            }
            AuthError::Registry(_) | AuthError::Vault(_) | AuthError::Crypto(_) => {
                CommandError::new("internal_error", "Something went wrong. Please try again.")
            }
        }
    }
}

impl From<pwmgr_core::vault::VaultError> for CommandError {
    fn from(e: pwmgr_core::vault::VaultError) -> Self {
        use pwmgr_core::vault::VaultError;
        match e {
            VaultError::NotFound => {
                CommandError::new("not_found", "That item no longer exists.")
            }
            VaultError::UnlockFailed | VaultError::Db(_) | VaultError::Crypto(_) => {
                CommandError::new("internal_error", "Something went wrong. Please try again.")
            }
        }
    }
}

/// Not backed by an unlocked session - distinct from any pwmgr-core error because pwmgr-core has
/// no concept of "session"; that state lives entirely in this Tauri layer's AppState.
pub fn locked_error() -> CommandError {
    CommandError::new("locked", "Vault is locked. Please log in again.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use pwmgr_core::auth::AuthError;

    #[test]
    fn auth_error_invalid_credentials_maps_to_single_generic_message() {
        let err = CommandError::from(AuthError::InvalidCredentials);
        assert_eq!(err.code, "invalid_credentials");
        assert_eq!(err.message, "Incorrect master password.");
    }

    #[test]
    fn auth_error_username_taken_maps_to_specific_message() {
        let err = CommandError::from(AuthError::UsernameTaken);
        assert_eq!(err.code, "username_taken");
    }

    #[test]
    fn auth_error_internal_variants_collapse_to_generic_message_not_raw_detail() {
        // Registry/Vault/Crypto errors are internal failure detail (e.g. raw SQLite error text)
        // that must never reach the frontend verbatim - THREAT_MODEL's wrong-password-oracle
        // concern generalizes to "don't leak internals through any error path," not just login.
        let err = CommandError::from(AuthError::Registry(
            pwmgr_core::registry::RegistryError::NotFound,
        ));
        assert_eq!(err.code, "internal_error");
        assert_eq!(err.message, "Something went wrong. Please try again.");
    }
}
