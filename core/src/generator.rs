use rand::seq::SliceRandom;
use thiserror::Error;
use zeroize::Zeroizing;

// Validation bounds live here (single source of truth); the IPC layer maps GenError -> invalid_input.
pub const MIN_LENGTH: usize = 4;
pub const MAX_LENGTH: usize = 128;
pub const MIN_WORDS: usize = 2;
pub const MAX_WORDS: usize = 12;
pub const MAX_SEPARATOR_LENGTH: usize = 8;

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
    #[error("Pattern template must be {MAX_LENGTH} characters or fewer.")]
    TemplateTooLong,
    #[error("Separator must be {MAX_SEPARATOR_LENGTH} characters or fewer.")]
    SeparatorTooLong,
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
    if separator.chars().count() > MAX_SEPARATOR_LENGTH {
        return Err(GenError::SeparatorTooLong);
    }
    let list = wordlist();
    let mut rng = rand::thread_rng();
    let picked: Vec<&str> = (0..words).map(|_| *list.choose(&mut rng).unwrap()).collect();
    Ok(Zeroizing::new(picked.join(separator)))
}

fn pick(set: &str, rng: &mut impl rand::Rng) -> char {
    set.chars().collect::<Vec<_>>().choose(rng).copied().unwrap()
}

pub fn generate_pattern(template: &str) -> Result<Zeroizing<String>, GenError> {
    if template.chars().count() > MAX_LENGTH {
        return Err(GenError::TemplateTooLong);
    }
    let mut rng = rand::thread_rng();
    // Pre-sized to the template's char count: each iteration consumes >=1 input char and produces
    // <=1 output char, so this is a safe upper bound - zero reallocations, matching Task 1.5's fix
    // (an unsized String::new()+push would leave unzeroized plaintext fragments in freed heap
    // memory on each reallocation).
    let mut out = String::with_capacity(template.chars().count());
    let mut chars = template.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => { if let Some(lit) = chars.next() { out.push(lit); } } // escape: next char literal
            'C' => out.push(pick(CONSONANTS, &mut rng).to_ascii_uppercase()),
            'c' => out.push(pick(CONSONANTS, &mut rng)),
            'V' => out.push(pick(VOWELS, &mut rng).to_ascii_uppercase()),
            'v' => out.push(pick(VOWELS, &mut rng)),
            '#' => out.push(pick(DIGITS, &mut rng)),
            '?' => out.push(pick(SYMBOLS, &mut rng)),
            other => out.push(other),
        }
    }
    if out.is_empty() {
        return Err(GenError::EmptyPattern);
    }
    Ok(Zeroizing::new(out))
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

    #[test]
    fn memorable_rejects_separator_longer_than_max() {
        let separator = "-".repeat(MAX_SEPARATOR_LENGTH + 1);
        assert!(matches!(generate_memorable(4, &separator), Err(GenError::SeparatorTooLong)));
    }

    #[test]
    fn memorable_accepts_separator_at_max_length() {
        let separator = "-".repeat(MAX_SEPARATOR_LENGTH);
        let pw = generate_memorable(4, &separator).unwrap();
        assert_eq!(pw.split(separator.as_str()).count(), 4);
    }

    #[test]
    fn pattern_expands_every_token_type() {
        let pw = generate_pattern("Cv#?").unwrap();
        let chars: Vec<char> = pw.chars().collect();
        assert_eq!(chars.len(), 4);
        assert!(CONSONANTS.to_uppercase().contains(chars[0]));
        assert!(VOWELS.contains(chars[1]));
        assert!(chars[2].is_ascii_digit());
        assert!(SYMBOLS.contains(chars[3]));
    }

    #[test]
    fn pattern_passes_literals_and_escapes_through() {
        // '-' is literal; '\C' is an escaped literal 'C'.
        let pw = generate_pattern(r"v-\C").unwrap();
        let chars: Vec<char> = pw.chars().collect();
        assert_eq!(chars.len(), 3);
        assert!(VOWELS.contains(chars[0]));
        assert_eq!(chars[1], '-');
        assert_eq!(chars[2], 'C');
    }

    #[test]
    fn pattern_rejects_empty_template() {
        assert!(matches!(generate_pattern(""), Err(GenError::EmptyPattern)));
    }

    #[test]
    fn pattern_rejects_template_longer_than_max_length() {
        let template = "c".repeat(MAX_LENGTH + 1);
        assert!(matches!(generate_pattern(&template), Err(GenError::TemplateTooLong)));
    }

    #[test]
    fn pattern_accepts_template_at_max_length() {
        let template = "c".repeat(MAX_LENGTH);
        assert_eq!(generate_pattern(&template).unwrap().chars().count(), MAX_LENGTH);
    }

    #[test]
    fn pattern_handles_consecutive_escapes() {
        // '\\\\' in the Rust source is the two-character template `\\`, which the DSL's escape rule
        // (`\x` -> literal x) expands to one literal backslash. Two of them back-to-back -> "\\\\".
        let pw = generate_pattern(r"\\\\").unwrap();
        assert_eq!(&*pw, r"\\");
    }
}
