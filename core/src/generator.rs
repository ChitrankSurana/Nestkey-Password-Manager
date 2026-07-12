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
    // Explicit with_capacity+push, not `.collect()`: functionally equivalent today (Range/Map report
    // an exact size_hint, and String's Extend<char> impl already reserves once from that hint before
    // pushing - verified empirically, zero reallocations either way), but this form doesn't depend on
    // that iterator-chain invariant holding. A later edit that inserts a `.filter()`/`.take_while()`
    // into the iterator would silently make the size_hint inexact and reopen a real gap (reallocation
    // during growth copies plaintext into a new heap block and frees the old one unzeroized); this
    // form can't regress that way since it reserves directly from `opts.length`, not a hint.
    let mut s = String::with_capacity(opts.length);
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

static EFF_SHORT: &str = include_str!("wordlists/eff_short.txt");

// Parses the last whitespace-delimited token per line (tolerates the "11213<TAB>word" EFF format
// or a bare word list). Cheap to rebuild per call (~1296 &str slices from a &'static str); no
// caching dep needed. // ponytail: rebuild each call, add once_cell only if profiling flags it.
fn wordlist() -> Vec<&'static str> {
    EFF_SHORT.lines().filter_map(|l| l.split_whitespace().last()).collect()
}

pub fn generate_memorable(words: usize, separator: &str) -> Result<Zeroizing<String>, GenError> {
    if !(MIN_WORDS..=MAX_WORDS).contains(&words) {
        return Err(GenError::InvalidWordCount);
    }
    let list = wordlist();
    let mut rng = rand::thread_rng();
    let picked: Vec<&str> = (0..words).map(|_| *list.choose(&mut rng).unwrap()).collect();
    Ok(Zeroizing::new(picked.join(separator)))
}

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
        // Regression guard, not a red->green bug demonstration: with_capacity(n) + n pushes of
        // 1-byte chars has capacity exactly n by construction, independent of any iterator's
        // size_hint. Checked at a representative length and at both boundaries (below) so a future
        // edit can't silently regress capacity-exactness at the edges of the valid range.
        let opts = RandomOptions { length: 50, uppercase: true, lowercase: true, digits: true, symbols: true };
        let pw = generate_random(&opts).unwrap();
        assert_eq!(pw.capacity(), 50);
    }

    #[test]
    fn random_output_allocates_exact_capacity_at_length_boundaries() {
        let mk = |length| RandomOptions { length, uppercase: true, lowercase: true, digits: true, symbols: true };
        assert_eq!(generate_random(&mk(MIN_LENGTH)).unwrap().capacity(), MIN_LENGTH);
        assert_eq!(generate_random(&mk(MAX_LENGTH)).unwrap().capacity(), MAX_LENGTH);
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

    #[test]
    fn memorable_produces_requested_word_count() {
        let pw = generate_memorable(4, "-").unwrap();
        assert_eq!(pw.split('-').count(), 4);
    }

    #[test]
    fn memorable_pulls_only_words_from_the_wordlist() {
        let words = wordlist(); // test-visible helper
        let pw = generate_memorable(6, " ").unwrap();
        assert!(pw.split(' ').all(|w| words.contains(&w)));
    }

    #[test]
    fn memorable_rejects_out_of_range_word_count() {
        assert!(matches!(generate_memorable(1, "-"), Err(GenError::InvalidWordCount)));
        assert!(matches!(generate_memorable(99, "-"), Err(GenError::InvalidWordCount)));
    }

    #[test]
    fn memorable_two_draws_differ() {
        assert_ne!(*generate_memorable(6, " ").unwrap(), *generate_memorable(6, " ").unwrap());
    }
}
