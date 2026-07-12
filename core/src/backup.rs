use crate::registry::UserRecord;
use thiserror::Error;

const BUNDLE_VERSION: u8 = 1;

#[derive(Error, Debug, PartialEq)]
pub enum BackupError {
    #[error("backup file is empty or truncated")]
    Truncated,
    #[error("unsupported backup version {0}")]
    UnsupportedVersion(u8),
    #[error("backup contains invalid UTF-8 in the username field")]
    InvalidUsername,
}

fn write_field(out: &mut Vec<u8>, field: &[u8]) {
    out.extend_from_slice(&(field.len() as u32).to_le_bytes());
    out.extend_from_slice(field);
}

/// Bundles a registry record and the raw bytes of its vault file into one self-contained buffer.
/// No new crypto: `vault_bytes` is already SQLCipher ciphertext, and every `UserRecord` field is
/// either non-secret (salts) or already-sealed (sentinel_ciphertext) - this just frames them
/// together so restore can pull both back out atomically.
pub fn encode_bundle(user: &UserRecord, vault_bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.push(BUNDLE_VERSION);
    write_field(&mut out, user.username.as_bytes());
    write_field(&mut out, &user.kdf_salt);
    write_field(&mut out, &user.verifier_salt);
    write_field(&mut out, &user.verifier_hash);
    write_field(&mut out, &user.sentinel_ciphertext);
    write_field(&mut out, &user.sentinel_nonce);
    write_field(&mut out, vault_bytes);
    out
}

fn read_field<'a>(buf: &'a [u8], cursor: &mut usize) -> Result<&'a [u8], BackupError> {
    if buf.len() < *cursor + 4 {
        return Err(BackupError::Truncated);
    }
    let len = u32::from_le_bytes(buf[*cursor..*cursor + 4].try_into().unwrap()) as usize;
    *cursor += 4;
    if buf.len() < *cursor + len {
        return Err(BackupError::Truncated);
    }
    let field = &buf[*cursor..*cursor + len];
    *cursor += len;
    Ok(field)
}

pub fn decode_bundle(bytes: &[u8]) -> Result<(UserRecord, Vec<u8>), BackupError> {
    if bytes.is_empty() {
        return Err(BackupError::Truncated);
    }
    let version = bytes[0];
    if version != BUNDLE_VERSION {
        return Err(BackupError::UnsupportedVersion(version));
    }
    let mut cursor = 1usize;
    let username = String::from_utf8(read_field(bytes, &mut cursor)?.to_vec())
        .map_err(|_| BackupError::InvalidUsername)?;
    let kdf_salt = read_field(bytes, &mut cursor)?.to_vec();
    let verifier_salt = read_field(bytes, &mut cursor)?.to_vec();
    let verifier_hash = read_field(bytes, &mut cursor)?.to_vec();
    let sentinel_ciphertext = read_field(bytes, &mut cursor)?.to_vec();
    let sentinel_nonce = read_field(bytes, &mut cursor)?.to_vec();
    let vault_bytes = read_field(bytes, &mut cursor)?.to_vec();
    Ok((
        UserRecord { username, kdf_salt, verifier_salt, verifier_hash, sentinel_ciphertext, sentinel_nonce },
        vault_bytes,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::UserRecord;

    fn sample_user() -> UserRecord {
        UserRecord {
            username: "alice".to_string(),
            kdf_salt: vec![1, 2, 3, 4],
            verifier_salt: vec![5, 6, 7, 8],
            verifier_hash: vec![9; 32],
            sentinel_ciphertext: vec![10, 11, 12],
            sentinel_nonce: vec![13; 12],
        }
    }

    #[test]
    fn encode_then_decode_roundtrips_every_field() {
        let user = sample_user();
        let vault_bytes = b"pretend-sqlcipher-ciphertext".to_vec();

        let bundle = encode_bundle(&user, &vault_bytes);
        let (decoded_user, decoded_vault) = decode_bundle(&bundle).unwrap();

        assert_eq!(decoded_user.username, "alice");
        assert_eq!(decoded_user.kdf_salt, vec![1, 2, 3, 4]);
        assert_eq!(decoded_user.verifier_hash, vec![9; 32]);
        assert_eq!(decoded_vault, vault_bytes);
    }

    #[test]
    fn decode_rejects_an_unsupported_version_byte() {
        let mut bundle = encode_bundle(&sample_user(), b"x");
        bundle[0] = 99;

        assert!(matches!(decode_bundle(&bundle), Err(BackupError::UnsupportedVersion(99))));
    }

    #[test]
    fn decode_rejects_an_empty_buffer() {
        assert!(matches!(decode_bundle(&[]), Err(BackupError::Truncated)));
    }

    #[test]
    fn decode_rejects_a_buffer_truncated_mid_field() {
        let bundle = encode_bundle(&sample_user(), b"vault-bytes-here");
        let cut = &bundle[..bundle.len() - 5]; // chop off the tail of the last field

        assert!(matches!(decode_bundle(cut), Err(BackupError::Truncated)));
    }

    #[test]
    fn decode_rejects_invalid_utf8_in_the_username_field() {
        // encode_bundle can't produce this itself (UserRecord.username is always a valid
        // String), so this hand-builds the byte layout directly - the only way to reach
        // BackupError::InvalidUsername's defined-but-otherwise-unreachable path.
        let mut bundle = vec![BUNDLE_VERSION];
        write_field(&mut bundle, &[0xFF, 0xFE]); // not valid UTF-8
        for _ in 0..6 {
            write_field(&mut bundle, b""); // remaining fields never get read - username fails first
        }

        assert!(matches!(decode_bundle(&bundle), Err(BackupError::InvalidUsername)));
    }
}
