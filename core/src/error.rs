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

impl From<crate::auth::AuthError> for CommandError {
    fn from(e: crate::auth::AuthError) -> Self {
        use crate::auth::AuthError;
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

impl From<crate::vault::VaultError> for CommandError {
    fn from(e: crate::vault::VaultError) -> Self {
        use crate::vault::VaultError;
        match e {
            VaultError::NotFound => {
                CommandError::new("not_found", "That item no longer exists.")
            }
            VaultError::InvalidImportRecord(msg) => invalid_input_error(&msg),
            VaultError::UnlockFailed | VaultError::Db(_) | VaultError::Crypto(_) => {
                CommandError::new("internal_error", "Something went wrong. Please try again.")
            }
        }
    }
}

impl From<crate::generator::GenError> for CommandError {
    fn from(e: crate::generator::GenError) -> Self {
        // GenError's Display strings are already safe, user-facing validation text - surface directly.
        invalid_input_error(&e.to_string())
    }
}

impl From<crate::totp::TotpError> for CommandError {
    fn from(e: crate::totp::TotpError) -> Self {
        // TotpError's one variant's Display text is already safe, user-facing validation text.
        invalid_input_error(&e.to_string())
    }
}

impl From<crate::portio::PortioError> for CommandError {
    fn from(e: crate::portio::PortioError) -> Self {
        use crate::portio::PortioError;
        match e {
            PortioError::Write => internal_error(),
            PortioError::Parse
            | PortioError::HeaderMismatch(_)
            | PortioError::MissingColumn(_)
            | PortioError::UnknownField(_) => invalid_input_error(&e.to_string()),
        }
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

/// Not backed by an auth/session failure - a well-formed, authenticated request rejected purely
/// on a request-shape ground (e.g. pwmgr-web's CSRF header check). Distinct from `locked_error`
/// so the frontend/caller can tell "please log in again" apart from "this specific request was
/// malformed."
pub fn forbidden_error(message: &str) -> CommandError {
    CommandError::new("forbidden", message)
}

impl CommandError {
    // `code`/`message` are deliberately private (see the doc comment on the struct) so no call
    // site can construct a CommandError by hand - but callers outside this module still need
    // read access to assert on the code without falling back to matching on the derived Debug
    // string. A read-only accessor preserves the "no external construction" guarantee (it can't
    // be used to build an instance) while allowing that.
    //
    // Unconditional `pub fn`, not `#[cfg(test)] pub(crate)`: `#[cfg(test)]` only compiles in when
    // this defining crate (pwmgr-core) is itself under `cargo test`, not when a dependent crate
    // (src-tauri, viewer-src-tauri) runs its own tests against this now-external type. Both
    // binaries' test suites need `.code()` on errors returned from their promoted do_* functions.
    // Not a new information exposure: `code`/`message` are already Serialize'd across IPC to the
    // frontend on every error today: this just adds a same-information read accessor for Rust
    // callers too.
    pub fn code(&self) -> &str {
        &self.code
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::AuthError;

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
            crate::registry::RegistryError::NotFound,
        ));
        assert_eq!(err.code, "internal_error");
        assert_eq!(err.message, "Something went wrong. Please try again.");
    }
}
