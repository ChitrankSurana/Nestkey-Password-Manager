use std::collections::HashMap;

pub struct ReuseInput {
    pub credential_id: i64,
    pub secret: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReusedGroup {
    pub credential_ids: Vec<i64>,
}

/// Groups credentials that share an identical plaintext secret. Every returned group has 2+
/// members - a secret used exactly once produces no group at all. Pure function: takes
/// already-decrypted secrets, does no DB or crypto work itself.
pub fn classify_reused(inputs: &[ReuseInput]) -> Vec<ReusedGroup> {
    let mut by_secret: HashMap<&str, Vec<i64>> = HashMap::new();
    for i in inputs {
        by_secret.entry(i.secret.as_str()).or_default().push(i.credential_id);
    }
    by_secret
        .into_values()
        .filter(|ids| ids.len() >= 2)
        .map(|credential_ids| ReusedGroup { credential_ids })
        .collect()
}

/// Given a full SHA-1 hex digest and the (suffix, count) entries HIBP's range API returned for
/// that digest's first-5-char prefix, finds whether the digest's own suffix (chars 5..) is among
/// them. Case-insensitive since HIBP's casing isn't a contract this function should depend on.
pub fn hibp_match(full_hash_hex: &str, entries: &[(String, u32)]) -> Option<u32> {
    let suffix = full_hash_hex.get(5..)?;
    entries
        .iter()
        .find(|(s, _)| s.eq_ignore_ascii_case(suffix))
        .map(|(_, count)| *count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_reused_groups_credentials_sharing_an_identical_secret() {
        let inputs = vec![
            ReuseInput { credential_id: 1, secret: "shared-secret".to_string() },
            ReuseInput { credential_id: 2, secret: "shared-secret".to_string() },
            ReuseInput { credential_id: 3, secret: "unique-secret".to_string() },
        ];

        let groups = classify_reused(&inputs);

        assert_eq!(groups.len(), 1);
        let mut ids = groups[0].credential_ids.clone();
        ids.sort();
        assert_eq!(ids, vec![1, 2]);
    }

    #[test]
    fn classify_reused_ignores_secrets_used_only_once() {
        let inputs = vec![
            ReuseInput { credential_id: 1, secret: "a".to_string() },
            ReuseInput { credential_id: 2, secret: "b".to_string() },
        ];

        assert!(classify_reused(&inputs).is_empty());
    }

    #[test]
    fn classify_reused_handles_three_way_reuse_as_one_group() {
        let inputs = vec![
            ReuseInput { credential_id: 1, secret: "x".to_string() },
            ReuseInput { credential_id: 2, secret: "x".to_string() },
            ReuseInput { credential_id: 3, secret: "x".to_string() },
        ];

        let groups = classify_reused(&inputs);

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].credential_ids.len(), 3);
    }

    #[test]
    fn hibp_match_finds_a_matching_suffix() {
        let full_hash = "AAAAA1234567890ABCDEF1234567890ABCDEF12"; // 40 hex chars, fabricated for the test
        let suffix = &full_hash[5..];
        let entries = vec![(suffix.to_string(), 42u32), ("FFFF...".to_string(), 1)];

        assert_eq!(hibp_match(full_hash, &entries), Some(42));
    }

    #[test]
    fn hibp_match_returns_none_when_no_suffix_matches() {
        let full_hash = "AAAAA1234567890ABCDEF1234567890ABCDEF12";
        let entries = vec![("DEADBEEF00000000000000000000000000".to_string(), 5)];

        assert_eq!(hibp_match(full_hash, &entries), None);
    }

    #[test]
    fn hibp_match_is_case_insensitive() {
        // HIBP's own responses are uppercase, but don't assume every caller normalizes first.
        let full_hash = "AAAAA1234567890abcdef1234567890abcdef12";
        let suffix_upper = "1234567890ABCDEF1234567890ABCDEF12";
        let entries = vec![(suffix_upper.to_string(), 7u32)];

        assert_eq!(hibp_match(full_hash, &entries), Some(7));
    }
}
