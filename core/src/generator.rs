use rand::seq::SliceRandom;
use thiserror::Error;
use zeroize::Zeroizing;

// Validation bounds live here (single source of truth); the IPC layer maps GenError -> invalid_input.
pub const MIN_LENGTH: usize = 4;
pub const MAX_LENGTH: usize = 128;
pub const MIN_WORDS: usize = 2;
pub const MAX_WORDS: usize = 12;

const UPPERCASE: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const LOWERCASE: &str = "abcdefghijklmnopqrstuvwxyz";
const DIGITS: &str = "0123456789";
// SYMBOLS intentionally overlaps with a couple of Task 3's pattern-DSL reserved tokens (# and ?) -
// those are template-input tokens, this is output-alphabet content; a generated password
// containing a literal '#' is never re-parsed as a template, so the overlap is harmless.
const SYMBOLS: &str = "!@#$%^&*()-_=+[]{};:,.?";
const VOWELS: &str = "aeiou";
const CONSONANTS: &str = "bcdfghjklmnpqrstvwxyz";

/// All messages here are safe, user-facing validation text (no internal detail) — the IPC layer
/// surfaces `e.to_string()` directly as an `invalid_input` message (see error.rs From impl).
#[derive(Error, Debug, PartialEq)]
pub enum GenError {
    #[error("Length must be between 4 and 128.")]
    InvalidLength,
    #[error("Word count must be between 2 and 12.")]
    InvalidWordCount,
    #[error("Select at least one character type.")]
    NoCharClasses,
    #[error("Pattern must contain at least one token.")]
    EmptyPattern,
}

pub struct RandomOptions {
    pub length: usize,
    pub uppercase: bool,
    pub lowercase: bool,
    pub digits: bool,
    pub symbols: bool,
}

pub fn generate_random(opts: &RandomOptions) -> Result<Zeroizing<String>, GenError> {
    if !(MIN_LENGTH..=MAX_LENGTH).contains(&opts.length) {
        return Err(GenError::InvalidLength);
    }
    let mut pool: Vec<char> = Vec::new();
    if opts.uppercase { pool.extend(UPPERCASE.chars()); }
    if opts.lowercase { pool.extend(LOWERCASE.chars()); }
    if opts.digits { pool.extend(DIGITS.chars()); }
    if opts.symbols { pool.extend(SYMBOLS.chars()); }
    if pool.is_empty() {
        return Err(GenError::NoCharClasses);
    }
    let mut rng = rand::thread_rng();
    // `choose` is uniform/bias-free over the pool (same primitive family as crypto.rs's CSPRNG use).
    let mut s = String::with_capacity(opts.length); // ASCII-only alphabet => exact capacity, zero reallocation
    for _ in 0..opts.length {
        s.push(*pool.choose(&mut rng).unwrap());
    }
    Ok(Zeroizing::new(s))
}

// Note (deliberate, not a bug): `generate_random` does not guarantee "at least one char from
// every selected class." That guarantee slightly lowers entropy and adds branching; at length >= 4
// over a multi-class pool the omission is negligible.
// ponytail: no per-class guarantee; add a post-generation class-presence retry only if a spec
// later requires it.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_respects_requested_length() {
        let opts = RandomOptions { length: 24, uppercase: true, lowercase: true, digits: true, symbols: true };
        assert_eq!(generate_random(&opts).unwrap().chars().count(), 24);
    }

    #[test]
    fn random_two_draws_differ() {
        let opts = RandomOptions { length: 32, uppercase: true, lowercase: true, digits: true, symbols: true };
        assert_ne!(*generate_random(&opts).unwrap(), *generate_random(&opts).unwrap());
    }

    #[test]
    fn random_respects_excluded_classes() {
        // digits-only: output must be all ASCII digits, nothing else.
        let opts = RandomOptions { length: 40, uppercase: false, lowercase: false, digits: true, symbols: false };
        let pw = generate_random(&opts).unwrap();
        assert!(pw.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn random_rejects_no_classes_selected() {
        let opts = RandomOptions { length: 16, uppercase: false, lowercase: false, digits: false, symbols: false };
        assert!(matches!(generate_random(&opts), Err(GenError::NoCharClasses)));
    }

    #[test]
    fn random_rejects_out_of_range_length() {
        let opts = RandomOptions { length: 0, uppercase: true, lowercase: false, digits: false, symbols: false };
        assert!(matches!(generate_random(&opts), Err(GenError::InvalidLength)));
    }

    #[test]
    fn random_output_allocates_exact_capacity_no_reallocation() {
        // A String built via with_capacity(n) + n pushes of 1-byte chars has capacity exactly n,
        // never more - collect()'s doubling growth would almost certainly overshoot n instead.
        let opts = RandomOptions { length: 50, uppercase: true, lowercase: true, digits: true, symbols: true };
        let pw = generate_random(&opts).unwrap();
        assert_eq!(pw.capacity(), 50);
    }

    #[test]
    fn random_accepts_min_and_max_length() {
        let mk = |length| RandomOptions { length, uppercase: true, lowercase: true, digits: true, symbols: true };
        assert_eq!(generate_random(&mk(MIN_LENGTH)).unwrap().chars().count(), MIN_LENGTH);
        assert_eq!(generate_random(&mk(MAX_LENGTH)).unwrap().chars().count(), MAX_LENGTH);
    }

    #[test]
    fn random_rejects_one_past_min_and_max_length() {
        let mk = |length| RandomOptions { length, uppercase: true, lowercase: true, digits: true, symbols: true };
        assert!(matches!(generate_random(&mk(MIN_LENGTH - 1)), Err(GenError::InvalidLength)));
        assert!(matches!(generate_random(&mk(MAX_LENGTH + 1)), Err(GenError::InvalidLength)));
    }
}
