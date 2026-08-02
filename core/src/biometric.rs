#[derive(Debug, thiserror::Error, PartialEq)]
pub enum BiometricError {
    #[error("biometric unlock is not supported on this device")]
    Unsupported,
    #[error("biometric verification failed or was cancelled")]
    VerificationFailed,
    #[error("could not protect or unprotect the stored key")]
    StorageFailed,
}

/// Abstracts the OS-specific "prove it's really you" gate plus secure local key storage a
/// biometric unlock path needs. The concrete implementation (Windows Hello's
/// `UserConsentVerifier` prompt + DPAPI-protected storage) lives in `src-tauri`, since it's
/// inherently platform-specific and untestable here - core only depends on this trait, so its own
/// orchestration logic (this module + the `do_*_biometric` commands) stays unit-testable with a
/// fake provider instead of a real fingerprint sensor.
pub trait BiometricProvider {
    /// Whether this device can do biometric verification at all (Windows Hello configured, on
    /// Windows; always false on other platforms/back-ends until they get their own provider).
    fn is_supported(&self) -> bool;
    /// Prompts the OS biometric/PIN gate. `Ok(false)` means the user cancelled or failed
    /// verification (not an error - a legitimate "no").
    fn verify(&self) -> Result<bool, BiometricError>;
    /// Wraps `data` (the raw vault key) using OS-backed protection tied to the current user
    /// account (e.g. DPAPI) - not a password-derived key, so there is no salt for a caller to keep.
    fn protect(&self, data: &[u8]) -> Result<Vec<u8>, BiometricError>;
    fn unprotect(&self, blob: &[u8]) -> Result<Vec<u8>, BiometricError>;
}

#[cfg(test)]
pub(crate) mod fake {
    use super::*;
    use std::cell::Cell;

    /// Round-trips via a trivial XOR "wrap" (not real crypto - a test double standing in for
    /// DPAPI, which this crate has no way to exercise without a real Windows user session).
    pub struct FakeProvider {
        pub supported: bool,
        pub verifies: Cell<bool>,
    }

    impl FakeProvider {
        pub fn new(supported: bool, verifies: bool) -> Self {
            FakeProvider { supported, verifies: Cell::new(verifies) }
        }
    }

    impl BiometricProvider for FakeProvider {
        fn is_supported(&self) -> bool {
            self.supported
        }
        fn verify(&self) -> Result<bool, BiometricError> {
            if !self.supported {
                return Err(BiometricError::Unsupported);
            }
            Ok(self.verifies.get())
        }
        fn protect(&self, data: &[u8]) -> Result<Vec<u8>, BiometricError> {
            Ok(data.iter().map(|b| b ^ 0xAA).collect())
        }
        fn unprotect(&self, blob: &[u8]) -> Result<Vec<u8>, BiometricError> {
            Ok(blob.iter().map(|b| b ^ 0xAA).collect())
        }
    }
}
