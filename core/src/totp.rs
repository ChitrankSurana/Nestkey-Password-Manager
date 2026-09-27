use hmac::{Hmac, Mac};
use sha1::Sha1;

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum TotpError {
    #[error("TOTP secret must be a valid base32 string")]
    InvalidSecret,
}

const STEP_SECONDS: u64 = 30;

/// RFC 6238 TOTP: HMAC-SHA1, 30s step, 6-digit code - the classic profile (Google Authenticator,
/// Bitwarden's default) matching the plan doc's chosen spec. `secret_base32` is the value a user
/// pastes from a site's "set up 2FA" screen (e.g. `JBSWY3DPEHPK3PXP`); this decodes with
/// `BASE32_NOPAD` rather than requiring `=` padding, so `=` is stripped alongside whitespace/dashes
/// below rather than passed through - a secret copied from a site that *does* pad it (some issuers
/// do) was previously rejected outright with the generic "must be a valid base32 string" error
/// (J2). `normalize_secret` is exposed so a caller can persist the cleaned form rather than the
/// raw pasted string (see `commands::do_set_totp_secret`), avoiding re-deriving it on every read.
pub fn normalize_secret(secret_base32: &str) -> String {
    secret_base32
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-' && *c != '=')
        .collect::<String>()
        .to_ascii_uppercase()
}

pub fn generate_code(secret_base32: &str, unix_time: u64) -> Result<String, TotpError> {
    let cleaned = normalize_secret(secret_base32);
    let key = data_encoding::BASE32_NOPAD
        .decode(cleaned.as_bytes())
        .map_err(|_| TotpError::InvalidSecret)?;
    Ok(code_for_key(&key, unix_time))
}

fn code_for_key(key: &[u8], unix_time: u64) -> String {
    let counter = unix_time / STEP_SECONDS;
    // HMAC over an 8-byte big-endian counter, then RFC 4226 dynamic truncation - the same
    // "HOTP at a time-derived counter" construction TOTP is defined as.
    let mut mac = <Hmac<Sha1> as Mac>::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(&counter.to_be_bytes());
    let hash = mac.finalize().into_bytes();
    let offset = (hash[hash.len() - 1] & 0x0f) as usize;
    let bin_code = ((u32::from(hash[offset]) & 0x7f) << 24)
        | (u32::from(hash[offset + 1]) << 16)
        | (u32::from(hash[offset + 2]) << 8)
        | u32::from(hash[offset + 3]);
    format!("{:06}", bin_code % 1_000_000)
}

/// Seconds left in the current 30s step - drives the UI's countdown ring.
pub fn seconds_remaining(unix_time: u64) -> u32 {
    (STEP_SECONDS - (unix_time % STEP_SECONDS)) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_code_matches_the_rfc_6238_sha1_test_vector_at_t59() {
        // RFC 6238 Appendix B: key = ASCII "12345678901234567890", T=59 -> HOTP truncation
        // 0x057... = 94287082 (the RFC's 8-digit example value). Our 6-digit output is that same
        // truncated value mod 10^6, i.e. its last 6 digits.
        let code = code_for_key(b"12345678901234567890", 59);
        assert_eq!(code, "287082");
    }

    #[test]
    fn generate_code_base32_decodes_before_computing_the_same_value_as_the_raw_key_path() {
        let key = b"12345678901234567890";
        let secret_base32 = data_encoding::BASE32_NOPAD.encode(key);

        let code = generate_code(&secret_base32, 59).unwrap();

        assert_eq!(code, code_for_key(key, 59));
    }

    #[test]
    fn generate_code_is_case_and_whitespace_insensitive_and_accepts_dashes() {
        let key = b"12345678901234567890";
        let secret_base32 = data_encoding::BASE32_NOPAD.encode(key).to_ascii_lowercase();
        let spaced = format!(" {}-{} ", &secret_base32[..8], &secret_base32[8..]);

        let code = generate_code(&spaced, 59).unwrap();

        assert_eq!(code, code_for_key(key, 59));
    }

    #[test]
    fn generate_code_rejects_invalid_base32() {
        assert_eq!(generate_code("not-valid-base32-!!!", 59), Err(TotpError::InvalidSecret));
    }

    #[test]
    fn generate_code_accepts_padded_base32_and_tabs_or_newlines() {
        // J2: a secret pasted from a site that pads it (some issuers do) was previously rejected
        // outright with "TOTP secret must be a valid base32 string" - BASE32_NOPAD's decode fails
        // on a trailing '=' unless it's stripped first, same as whitespace/dashes already were.
        // A 4-byte key is deliberately NOT a multiple of 5 bytes (unlike the 20-byte RFC key used
        // elsewhere in this file, which base32-encodes to an exact 32 chars with no padding at
        // all) - 32 bits needs padding out to a full 8-char base32 group.
        let key = b"1234";
        let secret_base32 = data_encoding::BASE32.encode(key); // padded, unlike BASE32_NOPAD
        assert!(secret_base32.contains('='), "test fixture must actually be padded");
        let messy = format!("\t{}\n", secret_base32);

        let code = generate_code(&messy, 59).unwrap();

        assert_eq!(code, code_for_key(key, 59));
    }

    #[test]
    fn normalize_secret_strips_whitespace_dashes_and_padding_then_uppercases() {
        assert_eq!(normalize_secret(" jbsw-y3dp ehpk3pxp=\t\n"), "JBSWY3DPEHPK3PXP");
    }

    #[test]
    fn generate_code_changes_across_a_30s_step_boundary() {
        let key = "JBSWY3DPEHPK3PXP";
        assert_ne!(generate_code(key, 29).unwrap(), generate_code(key, 30).unwrap());
        // Same step: 0 and 29 both fall in counter 0.
        assert_eq!(generate_code(key, 0).unwrap(), generate_code(key, 29).unwrap());
    }

    #[test]
    fn seconds_remaining_counts_down_within_a_step() {
        assert_eq!(seconds_remaining(0), 30);
        assert_eq!(seconds_remaining(29), 1);
        assert_eq!(seconds_remaining(30), 30);
    }
}
