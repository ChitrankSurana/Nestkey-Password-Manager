use nestkey_core::biometric::{BiometricError, BiometricProvider};

/// Windows Hello (`UserConsentVerifier`) as the "prove it's really you" gate, DPAPI
/// (`CryptProtectData`/`CryptUnprotectData`, tied to the current Windows user account) as the key
/// storage. Both are real OS integrations this crate has no way to exercise against actual
/// hardware/an enrolled Windows Hello account in an automated build - verify manually after this
/// lands. Compiles to an always-unsupported stub on non-Windows targets.
pub struct WindowsHelloProvider;

#[cfg(target_os = "windows")]
mod imp {
    use super::{BiometricError, BiometricProvider, WindowsHelloProvider};
    use windows::core::HSTRING;
    use windows::Security::Credentials::UI::{
        UserConsentVerificationResult, UserConsentVerifier, UserConsentVerifierAvailability,
    };
    use windows::Win32::Foundation::LocalFree;
    use windows::Win32::Security::Cryptography::{
        CryptProtectData, CryptUnprotectData, CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN,
    };

    // Frees the CryptoAPI-allocated output blob and copies it into an owned Vec - both DPAPI calls
    // below need this same cleanup, so it's factored out rather than duplicated.
    unsafe fn take_blob(blob: CRYPT_INTEGER_BLOB) -> Vec<u8> {
        let data = std::slice::from_raw_parts(blob.pbData, blob.cbData as usize).to_vec();
        let _ = LocalFree(windows::Win32::Foundation::HLOCAL(blob.pbData as *mut core::ffi::c_void));
        data
    }

    impl BiometricProvider for WindowsHelloProvider {
        fn is_supported(&self) -> bool {
            (|| -> windows::core::Result<bool> {
                let availability = UserConsentVerifier::CheckAvailabilityAsync()?.get()?;
                Ok(availability == UserConsentVerifierAvailability::Available)
            })()
            .unwrap_or(false)
        }

        fn verify(&self) -> Result<bool, BiometricError> {
            let message = HSTRING::from("Unlock your password vault");
            let op = UserConsentVerifier::RequestVerificationAsync(&message)
                .map_err(|_| BiometricError::Unsupported)?;
            let result = op.get().map_err(|_| BiometricError::VerificationFailed)?;
            Ok(result == UserConsentVerificationResult::Verified)
        }

        fn protect(&self, data: &[u8]) -> Result<Vec<u8>, BiometricError> {
            unsafe {
                let input = CRYPT_INTEGER_BLOB { cbData: data.len() as u32, pbData: data.as_ptr() as *mut u8 };
                let mut output = CRYPT_INTEGER_BLOB::default();
                CryptProtectData(
                    &input,
                    windows::core::PCWSTR::null(),
                    None,
                    None,
                    None,
                    CRYPTPROTECT_UI_FORBIDDEN,
                    &mut output,
                )
                .map_err(|_| BiometricError::StorageFailed)?;
                Ok(take_blob(output))
            }
        }

        fn unprotect(&self, blob: &[u8]) -> Result<Vec<u8>, BiometricError> {
            unsafe {
                let input =
                    CRYPT_INTEGER_BLOB { cbData: blob.len() as u32, pbData: blob.as_ptr() as *mut u8 };
                let mut output = CRYPT_INTEGER_BLOB::default();
                CryptUnprotectData(&input, None, None, None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut output)
                    .map_err(|_| BiometricError::StorageFailed)?;
                Ok(take_blob(output))
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
impl BiometricProvider for WindowsHelloProvider {
    fn is_supported(&self) -> bool {
        false
    }
    fn verify(&self) -> Result<bool, BiometricError> {
        Err(BiometricError::Unsupported)
    }
    fn protect(&self, _data: &[u8]) -> Result<Vec<u8>, BiometricError> {
        Err(BiometricError::Unsupported)
    }
    fn unprotect(&self, _blob: &[u8]) -> Result<Vec<u8>, BiometricError> {
        Err(BiometricError::Unsupported)
    }
}
