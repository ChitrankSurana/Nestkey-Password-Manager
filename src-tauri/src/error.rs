use serde::Serialize;

/// Safe error DTO serialized across the Tauri IPC boundary. `pwmgr-core`'s error types carry
/// internal detail (e.g. raw SQLite error text) that must never reach the frontend verbatim -
/// per THREAT_MODEL, that would leak implementation details and risks creating an information
/// oracle (e.g. distinguishing "wrong password" from "unknown username" from "database
/// corrupted"). Every `From` conversion here collapses internal error variants down to one of a
/// small set of generic, user-facing `(code, message)` pairs.
// Fields are private: the only way to construct a CommandError is through the From impls below
// (or locked_error()), so every internal error variant is forced through a sanitizing match arm.
// Public fields would let any future call site build one directly (e.g.
// `CommandError { code: "...".into(), message: format!("{e}") }`), bypassing that guarantee and
// defeating this file's whole purpose. serde's derive can still see private fields since its
// generated code lives in this same module.
#[derive(Debug, Serialize)]
pub struct CommandError {
    code: String,
    message: String,
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

impl From<pwmgr_core::generator::GenError> for CommandError {
    fn from(e: pwmgr_core::generator::GenError) -> Self {
        // GenError's Display strings are already safe, user-facing validation text - surface directly.
        invalid_input_error(&e.to_string())
    }
}

impl From<pwmgr_core::portio::PortioError> for CommandError {
    fn from(_: pwmgr_core::portio::PortioError) -> Self {
        internal_error()
    }
}

/// Not backed by an unlocked session - distinct from any pwmgr-core error because pwmgr-core has
/// no concept of "session"; that state lives entirely in this Tauri layer's AppState.
pub fn locked_error() -> CommandError {
    CommandError::new("locked", "Vault is locked. Please log in again.")
}

/// A value from the frontend failed a business-rule check server-side.
pub fn invalid_input_error(message: &str) -> CommandError {
    CommandError::new("invalid_input", message)
}

/// Generic catch-all for internal failures that must not leak detail to the frontend.
pub fn internal_error() -> CommandError {
    CommandError::new("internal_error", "Something went wrong. Please try again.")
}

/// A destructive action needs explicit user confirmation before proceeding (e.g. restoring a
/// backup over an existing username). Distinct from `invalid_input_error` so the frontend can
/// branch on the code specifically - re-asking with `confirmed: true` is the intended remedy,
/// unlike a genuine invalid_input which re-asking can't fix.
pub fn confirm_required_error(message: &str) -> CommandError {
    CommandError::new("confirm_required", message)
}

impl CommandError {
    // `code`/`message` are deliberately private (see the doc comment on the struct) so no call
    // site can construct a CommandError by hand - but callers outside this module (e.g.
    // commands.rs's tests) still need read access to assert on the code without falling back to
    // matching on the derived Debug string. A read-only accessor preserves the "no external
    // construction" guarantee (it can't be used to build an instance) while allowing that.
    #[cfg(test)]
    pub(crate) fn code(&self) -> &str {
        &self.code
    }
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
