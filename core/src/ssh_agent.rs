use signature::Signer;
use ssh_key::{PrivateKey, Signature};

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum SshAgentError {
    #[error("could not parse this as an OpenSSH private key")]
    InvalidKey,
    #[error("passphrase-encrypted keys aren't supported yet - decrypt it first (e.g. `ssh-keygen -p`)")]
    Encrypted,
    #[error("signing failed")]
    SignFailed,
}

fn parse(private_key_openssh: &str) -> Result<PrivateKey, SshAgentError> {
    let key = PrivateKey::from_openssh(private_key_openssh).map_err(|_| SshAgentError::InvalidKey)?;
    if key.is_encrypted() {
        return Err(SshAgentError::Encrypted);
    }
    Ok(key)
}

/// Parses and validates an OpenSSH-format private key, returning its public-key line (e.g.
/// `ssh-ed25519 AAAA... comment`) - used both to validate a key on import (reject anything that
/// doesn't parse before it's ever encrypted and stored) and to answer the agent's "list
/// identities" request without a separate stored public-key column: it's always re-derived from
/// the private key.
pub fn public_key_line(private_key_openssh: &str) -> Result<String, SshAgentError> {
    parse(private_key_openssh)?.public_key().to_openssh().map_err(|_| SshAgentError::InvalidKey)
}

/// Signs `data` with the given OpenSSH-format private key. Returns `ssh_key`'s own `Signature`
/// type (not raw bytes) - the ssh-agent-lib `Session::sign` trait method this backs expects
/// exactly that, and lets it handle the wire encoding.
pub fn sign(private_key_openssh: &str, data: &[u8]) -> Result<Signature, SshAgentError> {
    parse(private_key_openssh)?.try_sign(data).map_err(|_| SshAgentError::SignFailed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ssh_key::Algorithm;

    fn generate_ed25519_key() -> String {
        let key = PrivateKey::random(&mut rand::rngs::OsRng, Algorithm::Ed25519).unwrap();
        key.to_openssh(ssh_key::LineEnding::LF).unwrap().to_string()
    }

    #[test]
    fn public_key_line_returns_an_ssh_ed25519_line() {
        let openssh = generate_ed25519_key();

        let line = public_key_line(&openssh).unwrap();

        assert!(line.starts_with("ssh-ed25519 "));
    }

    #[test]
    fn public_key_line_rejects_garbage() {
        let result = public_key_line("not an ssh key");
        assert_eq!(result, Err(SshAgentError::InvalidKey));
    }

    #[test]
    fn sign_produces_a_signature_the_public_key_verifies() {
        use signature::Verifier;
        let openssh = generate_ed25519_key();
        let key = PrivateKey::from_openssh(&openssh).unwrap();
        let data = b"session-id || SSH2_MSG_USERAUTH_REQUEST || ...";

        let sig = sign(&openssh, data).unwrap();

        assert!(key.public_key().key_data().verify(data, &sig).is_ok());
    }

    #[test]
    fn sign_produces_a_different_signature_for_different_data() {
        let openssh = generate_ed25519_key();
        let sig_a = sign(&openssh, b"data one").unwrap();
        let sig_b = sign(&openssh, b"data two").unwrap();
        assert_ne!(sig_a, sig_b);
    }
}
