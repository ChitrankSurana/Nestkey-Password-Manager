# M2 — Password Generator + Strength/Entropy Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking. Implementers run on Sonnet, one task at a time; a
> `Task N.5` self-review addendum may be inserted after any task by the reviewer — do not
> pre-author those.

**Goal:** Deliver M2 from `docs/PRD.md` — a password generator with 4 methods (random, memorable,
pattern, pronounceable) plus a real-time strength/entropy meter — embedded directly in the existing
`CredentialEditor` modal, with a user-selectable simple/zxcvbn strength estimator persisted in
Settings, and copy-to-clipboard with auto-clear.

**Architecture:** A new serde-free `pwmgr-core::generator` module owns all four generation
algorithms and their input validation, reusing the crate's existing CSPRNG (`rand::thread_rng()`,
exactly as `crypto.rs` does). The `src-tauri` app layer adds three commands (`generate_password`,
`analyze_strength`, `copy_to_clipboard`) on the established `do_*` + `#[tauri::command]` wrapper
split, and one Settings field (`strength_algorithm`) persisted/validated exactly like
`auto_lock_minutes`. The **simple** entropy estimator lives entirely in the frontend (TypeScript,
`length × log2(poolSize)`, per keystroke, zero deps). The **advanced** estimator is the `zxcvbn`
Rust crate behind `analyze_strength`, called on debounce. `pwmgr-core` stays serde-free; `zxcvbn`
and the clipboard plugin are added to the **app** crate only, keeping the core lean.

**Tech Stack:** Rust (`pwmgr-core`, Tauri 2.x app), React 19 + TypeScript + Vite, Tailwind. Two new
Rust dependencies (`zxcvbn`, `tauri-plugin-clipboard-manager`) — justified in-line.

**Deliberately NOT in core:** no `generator`-side entropy helper. Simple entropy is a 2-line TS
function; advanced is zxcvbn in the app crate. Adding a core entropy fn nobody calls would be dead
code (YAGNI).

---

## Before you start (every task)

- Every `cargo`/`rustc`/`npm` PowerShell command **must** be prefixed:
  ```powershell
  $env:PATH = "$env:USERPROFILE\.cargo\bin;C:\Strawberry\perl\bin;C:\Strawberry\c\bin;" + $env:PATH
  ```
- Use PowerShell for cargo/npm/rustc. **Bash is for `git` only** (it cannot find `cargo` in this
  environment).
- Work in the `m2-password-generator` worktree at `.worktrees/m2-password-generator` (Task 0),
  **not** the main working copy. Confirm `git status` / `pwd` before every commit — the worktree's
  copy and `main`'s copy at the same relative path are two different files on disk.
- Repo root: `E:\Coding_Projects-CS\Account_1\Password_Manager`.
- Every task ends with a `git add` / `git commit` step using conventional-commit prefixes (`feat:`,
  `test:`, `build:`, `fix:`, `docs:`).

---

## Task 0: Create the isolated worktree

**Files:** none (git operations only)

- [ ] **Step 1: Create branch + worktree from `main`**

```bash
git worktree add -b m2-password-generator .worktrees/m2-password-generator main
```

- [ ] **Step 2: Verify the worktree is clean and on `main`'s tip**

Run: `git -C .worktrees/m2-password-generator status`
Expected: `On branch m2-password-generator` and `nothing to commit, working tree clean`.

Run: `git -C .worktrees/m2-password-generator log --oneline -1`
Expected: the same commit as `main`'s tip (the shipped M1 merge + this plan doc's own commit).

All subsequent tasks operate inside
`E:\Coding_Projects-CS\Account_1\Password_Manager\.worktrees\m2-password-generator`.

---

# Slice 1 — Core `generator` module (all 4 methods, unit-tested)

New file `core/src/generator.rs`, declared in `core/src/lib.rs`. Matches the per-concern file split
of `crypto`/`vault`/`auth`/`session`. **All randomness reuses `rand::thread_rng()`** (OS-seeded
CSPRNG, same as `crypto::random_salt`/`seal`) via `rand::seq::SliceRandom::choose`, which is
bias-free for slice selection. No new RNG dependency. Every generation function returns
`Result<Zeroizing<String>, GenError>` — the plaintext is wrapped in `zeroize::Zeroizing` at birth
(core already depends on `zeroize`). Validation lives here (the root cause place); the IPC layer
just maps `GenError` → `invalid_input`.

## Task 1: `generator` module skeleton, `GenError`, and the `random` method

**Files:**
- Modify: `core/src/lib.rs` (add `pub mod generator;`)
- Create: `core/src/generator.rs`

- [ ] **Step 1: Declare the module**

```rust
// core/src/lib.rs
pub mod crypto;
pub mod registry;
pub mod vault;
pub mod auth;
pub mod session;
pub mod generator;
```

- [ ] **Step 2: Write the failing tests** (mirrors `crypto.rs`'s `random_salt_is_not_all_zero_and_varies` template)

```rust
// core/src/generator.rs, inside `#[cfg(test)] mod tests { use super::*; ... }`

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
```

- [ ] **Step 3: Run to confirm they fail**

Run: `cargo test -p pwmgr-core generator`
Expected: FAIL — `cannot find type RandomOptions` / `generate_random`.

- [ ] **Step 4: Implement**

```rust
// core/src/generator.rs
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
    let s: String = (0..opts.length).map(|_| *pool.choose(&mut rng).unwrap()).collect();
    Ok(Zeroizing::new(s))
}
```

**Note (deliberate, not a bug):** `generate_random` does **not** guarantee "at least one char from
every selected class." That guarantee slightly lowers entropy and adds branching; at length ≥ 4
over a multi-class pool the omission is negligible.
`// ponytail:` no per-class guarantee; add a post-generation class-presence retry only if a spec
later requires it.

- [ ] **Step 5: Run to confirm pass**

Run: `cargo test -p pwmgr-core generator`
Expected: all 5 pass.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat(core): add generator module with CSPRNG random-password method"
```

---

## Task 2: `memorable` (diceware) method + bundled EFF short wordlist

**Files:**
- Create: `core/src/wordlists/eff_short.txt` (**manual asset — see Step 1**)
- Modify: `core/src/generator.rs`

**Wordlist embedding decision:** `include_str!("wordlists/eff_short.txt")` at compile time. It is a
tiny (~13 KB) immutable public-domain asset; compiling it into the binary removes any runtime
file-path/IO/permissions failure mode and needs no `AppPaths` plumbing. Runtime file reads would be
strictly more code for zero benefit on a static asset.

- [ ] **Step 1: Obtain and commit the real wordlist (manual)**

Obtain the EFF short wordlist (`short_wordlist.txt`, ~1296 words, public domain) from
https://www.eff.org/dice and save it verbatim to `core/src/wordlists/eff_short.txt`. **Do not
fabricate word content.** The file's canonical format is `<dice-digits><TAB><word>` per line; the
parser in Step 4 takes the **last whitespace-delimited token** per line, so it works whether the
committed file keeps the dice-number prefixes or is a bare one-word-per-line list.

Run (after saving): count the lines to sanity-check.
Run: `Get-Content core/src/wordlists/eff_short.txt | Measure-Object -Line`
Expected: `Lines : 1296` (the EFF short list). If the count is wildly off, you have the wrong file —
stop and re-download.

- [ ] **Step 2: Write the failing tests**

```rust
// core/src/generator.rs, mod tests

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
```

- [ ] **Step 3: Run to confirm fail** — `cargo test -p pwmgr-core memorable` → FAIL (no
  `generate_memorable`).

- [ ] **Step 4: Implement**

```rust
// core/src/generator.rs
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
```

- [ ] **Step 5: Run to confirm pass** — `cargo test -p pwmgr-core memorable` → all pass.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat(core): add diceware memorable method with bundled EFF short wordlist"
```

---

## Task 3: `pattern` method + mini-language

**Files:** Modify `core/src/generator.rs`

### Pattern mini-language spec (minimal)

Each character of the template expands as follows:

| Token | Expands to |
|---|---|
| `C` | random **uppercase** consonant |
| `c` | random **lowercase** consonant |
| `V` | random **uppercase** vowel |
| `v` | random **lowercase** vowel |
| `#` | random digit `0`–`9` |
| `?` | random symbol (from `SYMBOLS`) |
| `\x` | literal `x` (escape hatch — `\C` → literal `C`, `\\` → literal `\`) |
| any other char | passes through literally |

Upper/lower variants for letters are included so a pattern can produce mixed-case output (a
2-token `C`/`v`-only set could never yield an uppercase letter). That is the whole DSL — no
repetition counts, no character ranges, no groups (YAGNI; add only if a real request appears).
Empty template, or a template that expands to an empty string, is `GenError::EmptyPattern`.

- [ ] **Step 1: Write the failing tests**

```rust
// mod tests
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
```

- [ ] **Step 2: Run to confirm fail.**

- [ ] **Step 3: Implement**

```rust
// core/src/generator.rs
fn pick(set: &str, rng: &mut impl rand::Rng) -> char {
    set.chars().collect::<Vec<_>>().choose(rng).copied().unwrap()
}

pub fn generate_pattern(template: &str) -> Result<Zeroizing<String>, GenError> {
    let mut rng = rand::thread_rng();
    let mut out = String::new();
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
```

- [ ] **Step 4: Run to confirm pass.**

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(core): add pattern mini-language generation method"
```

---

## Task 4: `pronounceable` method

**Files:** Modify `core/src/generator.rs`

Algorithmic consonant-vowel alternation (fake syllables), no wordlist — distinct from `memorable`
(real words) and `pattern` (user template). Starts with a consonant, alternates to the requested
length; lowercase output.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn pronounceable_respects_length_and_uses_only_syllable_chars() {
    let pw = generate_pronounceable(12).unwrap();
    assert_eq!(pw.chars().count(), 12);
    assert!(pw.chars().all(|c| CONSONANTS.contains(c) || VOWELS.contains(c)));
}

#[test]
fn pronounceable_alternates_consonant_vowel() {
    let pw = generate_pronounceable(10).unwrap();
    for (i, c) in pw.chars().enumerate() {
        if i % 2 == 0 { assert!(CONSONANTS.contains(c)); } else { assert!(VOWELS.contains(c)); }
    }
}

#[test]
fn pronounceable_rejects_out_of_range_length() {
    assert!(matches!(generate_pronounceable(2), Err(GenError::InvalidLength)));
}
```

- [ ] **Step 2: Confirm fail.**

- [ ] **Step 3: Implement**

```rust
pub fn generate_pronounceable(length: usize) -> Result<Zeroizing<String>, GenError> {
    if !(MIN_LENGTH..=MAX_LENGTH).contains(&length) {
        return Err(GenError::InvalidLength);
    }
    let mut rng = rand::thread_rng();
    let s: String = (0..length)
        .map(|i| if i % 2 == 0 { pick(CONSONANTS, &mut rng) } else { pick(VOWELS, &mut rng) })
        .collect();
    Ok(Zeroizing::new(s))
}
```

- [ ] **Step 4: Confirm pass, then full core regression** — `cargo test -p pwmgr-core` → all pass.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(core): add pronounceable syllable generation method"
```

---

# Slice 2 — IPC commands (`generate_password`, `analyze_strength`)

## Task 5: `generate_password` command

**Files:**
- Modify: `src-tauri/src/dto.rs` (add `GenerateRequest` input DTO)
- Modify: `src-tauri/src/error.rs` (add `From<generator::GenError>`)
- Modify: `src-tauri/src/commands.rs` (`do_generate_password` + wrapper)
- Modify: `src-tauri/src/lib.rs` (register)

**Return type:** bare `Zeroizing<String>` (like `add_site` returns a bare `i64`). No new
secret-carrying DTO, so **no manual `Debug`-redaction surface to maintain** — this is the preferred
shape from the reuse map. `zeroize`'s `serde` feature is already enabled in
`src-tauri/Cargo.toml` (verified), so `Zeroizing<String>` serializes to the plaintext string across
IPC. The command is **stateless** (no vault/session needed) — it takes no `State`, but keeps the
`do_*`/wrapper split for testability.

- [ ] **Step 1: Add the input DTO** (`GenerateRequest` — a serde-tagged enum, so one command
  dispatches all four methods)

```rust
// src-tauri/src/dto.rs  (add `use serde::Deserialize;`)
#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", tag = "method")]
pub enum GenerateRequest {
    Random { length: usize, uppercase: bool, lowercase: bool, digits: bool, symbols: bool },
    Memorable { words: usize, separator: String },
    Pattern { template: String },
    Pronounceable { length: usize },
}
```

- [ ] **Step 2: Map `GenError` → `invalid_input`** (in `error.rs`, mirroring the existing
  `From<AuthError>`/`From<VaultError>` impls)

```rust
// src-tauri/src/error.rs
impl From<pwmgr_core::generator::GenError> for CommandError {
    fn from(e: pwmgr_core::generator::GenError) -> Self {
        // GenError's Display strings are already safe, user-facing validation text — surface directly.
        invalid_input_error(&e.to_string())
    }
}
```

- [ ] **Step 3: Write the failing tests** (in `commands.rs mod tests` — no `State` needed)

```rust
#[test]
fn generate_password_random_returns_requested_length() {
    let req = GenerateRequest::Random { length: 20, uppercase: true, lowercase: true, digits: true, symbols: false };
    let pw = do_generate_password(&req).unwrap();
    assert_eq!(pw.chars().count(), 20);
}

#[test]
fn generate_password_memorable_uses_separator() {
    let req = GenerateRequest::Memorable { words: 3, separator: "-".into() };
    let pw = do_generate_password(&req).unwrap();
    assert_eq!(pw.split('-').count(), 3);
}

#[test]
fn generate_password_maps_gen_error_to_invalid_input() {
    let req = GenerateRequest::Random { length: 0, uppercase: true, lowercase: false, digits: false, symbols: false };
    assert_eq!(do_generate_password(&req).unwrap_err().code(), "invalid_input");
}
```

- [ ] **Step 4: Confirm fail.**

- [ ] **Step 5: Implement**

```rust
// src-tauri/src/commands.rs  (add `use crate::dto::GenerateRequest; use zeroize::Zeroizing;`)
pub fn do_generate_password(req: &GenerateRequest) -> Result<Zeroizing<String>, CommandError> {
    use pwmgr_core::generator as gen;
    let pw = match req {
        GenerateRequest::Random { length, uppercase, lowercase, digits, symbols } =>
            gen::generate_random(&gen::RandomOptions {
                length: *length, uppercase: *uppercase, lowercase: *lowercase, digits: *digits, symbols: *symbols,
            })?,
        GenerateRequest::Memorable { words, separator } => gen::generate_memorable(*words, separator)?,
        GenerateRequest::Pattern { template } => gen::generate_pattern(template)?,
        GenerateRequest::Pronounceable { length } => gen::generate_pronounceable(*length)?,
    };
    Ok(pw)
}

#[tauri::command]
pub fn generate_password(request: GenerateRequest) -> Result<Zeroizing<String>, CommandError> {
    do_generate_password(&request)
}
```

- [ ] **Step 6: Register in `lib.rs`'s `generate_handler!`** — add `commands::generate_password,`.

- [ ] **Step 7: Confirm pass** — `cargo test -p pwmgr-app generate_password` then
  `cargo test --workspace`.

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "feat: add generate_password IPC command dispatching all four methods"
```

---

## Task 6: `analyze_strength` command (new `zxcvbn` dependency)

**Files:**
- Modify: `src-tauri/Cargo.toml` (add `zxcvbn`)
- Modify: `src-tauri/src/dto.rs` (add `StrengthDto`)
- Modify: `src-tauri/src/commands.rs` (`do_analyze_strength` + wrapper)
- Modify: `src-tauri/src/lib.rs` (register)

**New dependency justification (ponytail ladder):** the advanced estimator must detect dictionary
words, keyboard walks, l33t substitutions, and repeats — pattern-matching that is genuinely hard to
hand-roll correctly and a security-sensitive thing to get wrong. This is rung 5 ("already-installed
dependency" fails — nothing here does this) → a maintained, pure-Rust `zxcvbn` port is the right
reuse versus reinventing an entire heuristic engine. It lives in the **app** crate only (not
`pwmgr-core`), keeping the core serde-free and dependency-light. The **simple** estimator (Slice 4,
frontend) stays dependency-free — we add `zxcvbn` for the one thing a formula can't do, not for
both.

**Secret boundary note:** `analyze_strength` takes the password as **input** — the plaintext
crosses the IPC boundary in the request (same exposure class as the existing `add_credential`
secret arg; covered by THREAT_MODEL residual risk on credentials-in-use). The **return**
(`StrengthDto`) is a non-secret score/label, so it derives `Debug` normally.

- [ ] **Step 1: Add the dependency**

```toml
# src-tauri/Cargo.toml, [dependencies]
zxcvbn = "3"
```
Run: `cargo build -p pwmgr-app`
Expected: resolves and compiles `zxcvbn`. **Confirm the installed 3.x API** before Step 4:
`zxcvbn::zxcvbn(pw, &[])` returns an `Entropy` value directly (not a `Result`); `.score()` returns a
`Score` enum convertible to `u8`; `.guesses_log10()` returns `f64`. If the resolved version differs,
adapt the two accessor lines in Step 4 accordingly.

- [ ] **Step 2: Add `StrengthDto`** (non-secret → normal `Debug`)

```rust
// src-tauri/src/dto.rs
#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StrengthDto {
    pub score: u8,           // zxcvbn 0..=4
    pub guesses_log10: f64,  // log10(estimated guesses)
    pub label: String,       // "Very weak" .. "Very strong"
}
```

- [ ] **Step 3: Write the failing tests** (pure fn, no `State`)

```rust
#[test]
fn analyze_strength_scores_a_weak_password_low() {
    assert!(do_analyze_strength("password").score <= 1);
}

#[test]
fn analyze_strength_scores_a_strong_password_high() {
    assert!(do_analyze_strength("7#kLp!qZ2r@Vx9W").score >= 3);
}

#[test]
fn analyze_strength_handles_empty_without_panicking() {
    let s = do_analyze_strength("");
    assert_eq!(s.score, 0);
}
```

- [ ] **Step 4: Implement** (guard empty first — some zxcvbn versions dislike an empty input)

```rust
// src-tauri/src/commands.rs  (add `use crate::dto::StrengthDto;`)
pub fn do_analyze_strength(password: &str) -> StrengthDto {
    if password.is_empty() {
        return StrengthDto { score: 0, guesses_log10: 0.0, label: "Very weak".into() };
    }
    let est = zxcvbn::zxcvbn(password, &[]);
    let score = u8::from(est.score()); // adapt to `est.score() as u8` if From isn't impl'd (Step 1)
    let label = match score {
        0 => "Very weak", 1 => "Weak", 2 => "Fair", 3 => "Strong", _ => "Very strong",
    }.to_string();
    StrengthDto { score, guesses_log10: est.guesses_log10(), label }
}

#[tauri::command]
pub fn analyze_strength(password: String) -> StrengthDto {
    do_analyze_strength(&password)
}
```

- [ ] **Step 5: Register** in `lib.rs` — add `commands::analyze_strength,`.

- [ ] **Step 6: Confirm pass** — `cargo test -p pwmgr-app analyze_strength`, then
  `cargo test --workspace`.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "feat: add analyze_strength IPC command backed by zxcvbn"
```

---

# Slice 3 — Settings: `strength_algorithm` toggle (end-to-end)

Persisted/validated **exactly** like `auto_lock_minutes`: a `VALID_*`-contains check + sanitize-on-load
in `Config`, a `ConfigDto` field, a validated update command, and a Settings control following the
`editedRef`/`submitting`/`disabled`-while-saving conventions (M1 Task 25.5 template).

## Task 7: `Config` field + validation

**Files:** Modify `src-tauri/src/config.rs`

- [ ] **Step 1: Write the failing tests** (mirror the existing
  `load_rejects_invalid_auto_lock_minutes...` test)

```rust
#[test]
fn load_defaults_strength_algorithm_to_simple() {
    let dir = tempdir().unwrap();
    let config = Config::load(&dir.path().join("config.json")).unwrap();
    assert_eq!(config.strength_algorithm, "simple");
}

#[test]
fn load_rejects_invalid_strength_algorithm_by_falling_back_to_default() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.json");
    std::fs::write(&path, r#"{"theme":"dark","auto_lock_minutes":5,"strength_algorithm":"bogus"}"#).unwrap();
    assert_eq!(Config::load(&path).unwrap().strength_algorithm, "simple");
}
```

- [ ] **Step 2: Confirm fail.**

- [ ] **Step 3: Implement** (add the constant, the default fn, the field, extend the sanitize in
  `load`)

```rust
// src-tauri/src/config.rs
pub(crate) const VALID_STRENGTH_ALGORITHMS: [&str; 2] = ["simple", "zxcvbn"];

fn default_strength_algorithm() -> String { "simple".to_string() }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_auto_lock_minutes")]
    pub auto_lock_minutes: u32,
    #[serde(default = "default_strength_algorithm")]
    pub strength_algorithm: String,
}
// Update `impl Default` to include strength_algorithm: default_strength_algorithm().
```

Extend `Config::load`'s sanitize block (after the existing `auto_lock_minutes` guard), preserving
the sanitized value from the previous step:

```rust
let parsed = if !VALID_AUTO_LOCK_MINUTES.contains(&parsed.auto_lock_minutes) {
    Config { auto_lock_minutes: default_auto_lock_minutes(), ..parsed }
} else { parsed };
if !VALID_STRENGTH_ALGORITHMS.contains(&parsed.strength_algorithm.as_str()) {
    return Ok(Config { strength_algorithm: default_strength_algorithm(), ..parsed });
}
Ok(parsed)
```

- [ ] **Step 4: Confirm pass** — `cargo test -p pwmgr-app config` (existing config tests must
  still pass).

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat: add validated strength_algorithm field to Config"
```

---

## Task 8: `ConfigDto` field + `update_strength_algorithm` command

**Files:** Modify `src-tauri/src/dto.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`

- [ ] **Step 1: Add the field to `ConfigDto`**

```rust
// src-tauri/src/dto.rs — ConfigDto
pub struct ConfigDto {
    pub theme: String,
    pub auto_lock_minutes: u32,
    pub strength_algorithm: String,
}
```

- [ ] **Step 2: Write the failing tests** (mirror `update_auto_lock_minutes_persists...` and
  `..._rejects_an_out_of_range_value`)

```rust
#[test]
fn get_config_includes_strength_algorithm() {
    let (state, _paths, _dir) = test_state_and_paths();
    assert_eq!(do_get_config(&state).strength_algorithm, "simple");
}

#[test]
fn update_strength_algorithm_persists() {
    let (state, paths, _dir) = test_state_and_paths();
    do_update_strength_algorithm(&state, &paths, "zxcvbn").unwrap();
    assert_eq!(do_get_config(&state).strength_algorithm, "zxcvbn");
    assert_eq!(Config::load(&paths.config_path()).unwrap().strength_algorithm, "zxcvbn");
}

#[test]
fn update_strength_algorithm_rejects_invalid_value() {
    let (state, paths, _dir) = test_state_and_paths();
    assert_eq!(do_update_strength_algorithm(&state, &paths, "bogus").unwrap_err().code(), "invalid_input");
}
```

- [ ] **Step 3: Confirm fail.**

- [ ] **Step 4: Implement** — update `do_get_config` to include the field, and add the update
  command following `do_update_auto_lock_minutes`'s **fixed** (post-Task-24.5) mutate-a-clone-then-
  commit shape:

```rust
// src-tauri/src/commands.rs
use crate::config::VALID_STRENGTH_ALGORITHMS;

// in do_get_config: ConfigDto { ..., strength_algorithm: config.strength_algorithm.clone() }

pub fn do_update_strength_algorithm(
    state: &AppState,
    paths: &AppPaths,
    algorithm: &str,
) -> Result<(), CommandError> {
    if !VALID_STRENGTH_ALGORITHMS.contains(&algorithm) {
        return Err(crate::error::invalid_input_error("Unknown strength algorithm."));
    }
    let mut config = state.config.lock().unwrap();
    let mut updated = config.clone();
    updated.strength_algorithm = algorithm.to_string();
    updated.save(&paths.config_path()).map_err(|_| crate::error::internal_error())?;
    *config = updated; // commit in-memory only after the save succeeds (Task 24.5 precedent)
    Ok(())
}

#[tauri::command]
pub fn update_strength_algorithm(
    state: State<AppState>,
    paths: State<AppPaths>,
    algorithm: String,
) -> Result<(), CommandError> {
    do_update_strength_algorithm(&state, &paths, &algorithm)
}
```

- [ ] **Step 5: Register** `commands::update_strength_algorithm,` in `lib.rs`.

- [ ] **Step 6: Confirm pass** — `cargo test -p pwmgr-app`.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "feat: expose and persist strength_algorithm via config commands"
```

---

## Task 9: Settings UI toggle

**Files:** Modify `src/lib/api.ts`, `src/screens/Settings.tsx`

- [ ] **Step 1: Extend the API surface**

```ts
// src/lib/api.ts — ConfigDto interface: add
strengthAlgorithm: string;

// api object: add
updateStrengthAlgorithm: (algorithm: string) =>
  invoke<void>("update_strength_algorithm", { algorithm }),
```

- [ ] **Step 2: Add the control to `Settings.tsx`** following the existing
  `editedRef`/`submitting`/`disabled` idiom. Load both fields under the single `editedRef` guard;
  add a second state + handler.

```tsx
const [strengthAlgorithm, setStrengthAlgorithm] = useState<string | null>(null);

// in the getConfig().then(c => { ... }) block, under the same `if (!editedRef.current)` guard:
setStrengthAlgorithm(c.strengthAlgorithm);

async function handleAlgorithmChange(algo: string) {
  editedRef.current = true;
  setSubmitting(true);
  try {
    setError(null);
    await api.updateStrengthAlgorithm(algo);
    setStrengthAlgorithm(algo);
  } catch (err) {
    setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
  } finally {
    setSubmitting(false);
  }
}
```

Render a second labelled `<select>` (mirroring the auto-lock select's classes,
`disabled={submitting}`, wrapped in
`<Tooltip text="Which algorithm scores password strength" className="w-full">`), options `simple` →
"Simple (entropy)" and `zxcvbn` → "Advanced (zxcvbn)".

- [ ] **Step 3: Manual verification** (no frontend test harness in this project)

Run: `npm run tauri dev`
Expected: Settings shows the new dropdown; switching it and reopening Settings shows the choice
persisted; `~/.pwmgr/config.json` contains the new `strength_algorithm` value. Close the window
when confirmed.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat: add strength-algorithm toggle to Settings screen"
```

---

# Slice 4 — CredentialEditor: generate, meter, copy-with-auto-clear

## Task 10: Clipboard plugin + `copy_to_clipboard` command (auto-clear)

**Files:**
- Modify: `src-tauri/Cargo.toml` (add `tauri-plugin-clipboard-manager`)
- Modify: `src-tauri/src/lib.rs` (register plugin + command)
- Modify: `src-tauri/src/commands.rs` (`copy_to_clipboard`)

**New dependency justification (ponytail ladder):** clipboard access is a native OS capability with
no stdlib/existing-dep path in this project (verified: neither `src-tauri/Cargo.toml` nor
`package.json` currently has any clipboard package). The official `tauri-plugin-clipboard-manager`
is the maintained platform-blessed way to read/write/clear the OS clipboard — rung 4/5. We add
**only the Rust plugin** (not the `@tauri-apps/plugin-clipboard-manager` JS package): the auto-clear
timer lives in the backend (below), so the frontend routes through our own `copy_to_clipboard`
command and never needs the JS binding. Fewer moving parts.

**Timer-location decision (backend):** the auto-clear timer runs in a `tauri::async_runtime::spawn`
task in the backend, not a frontend `setTimeout`. Rationale: the copy happens from the
`CredentialEditor` modal, which **unmounts on save/close** — a `setTimeout` owned by that component
would be torn down on unmount, so the clipboard would *never* be cleared in the common flow (copy →
close modal). A component-independent module-level JS timer would survive unmount but not a webview
reload; the backend task survives both and only dies on full process exit — and process exit is a
wash (no in-process timer of any kind survives it; the OS clipboard just retains whatever was last
written regardless of approach). The backend task reuses the exact `async_runtime::spawn` +
`tokio::time::sleep` pattern already in `lib.rs`'s auto-lock loop, so it is not new machinery.

**Check-before-clear (race handling):** after sleeping, the task reads the current clipboard and
clears **only if it still equals the value we wrote** — so if the user copied something else in the
interim, we don't clobber their clipboard. The held copy of the secret is wrapped in
`Zeroizing<String>` for the task's lifetime, matching the project's zeroization bar.

- [ ] **Step 1: Add the dependency**

```toml
# src-tauri/Cargo.toml, [dependencies]
tauri-plugin-clipboard-manager = "2"
```

- [ ] **Step 2: Register the plugin** in `lib.rs` (before `.invoke_handler`):

```rust
.plugin(tauri_plugin_clipboard_manager::init())
```

- [ ] **Step 3: Implement the command** (thin wrapper only — the effect is an OS clipboard side
  effect requiring a real `AppHandle`, so it is not unit-testable in the headless harness; this is
  an intentional, justified deviation from the `do_*` split, verified manually in Step 5)

```rust
// src-tauri/src/commands.rs
use tauri_plugin_clipboard_manager::ClipboardExt;
use zeroize::Zeroizing;
use std::time::Duration;

const CLIPBOARD_CLEAR_SECS: u64 = 30; // hardcoded constant, NOT a user setting (ponytail)

#[tauri::command]
pub fn copy_to_clipboard(app: tauri::AppHandle, text: String) -> Result<(), CommandError> {
    let secret = Zeroizing::new(text);
    app.clipboard().write_text((*secret).clone()).map_err(|_| crate::error::internal_error())?;
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(CLIPBOARD_CLEAR_SECS)).await;
        // check-before-clear: only wipe if the clipboard still holds exactly what we wrote.
        if let Ok(current) = handle.clipboard().read_text() {
            if current == *secret {
                let _ = handle.clipboard().clear();
            }
        }
    });
    Ok(())
}
```

- [ ] **Step 4: Register** `commands::copy_to_clipboard,` in `lib.rs`. Run
  `cargo build -p pwmgr-app` (Expected: compiles). If a runtime permissions error appears when the
  command is exercised in Step 5, add the clipboard-manager permission to
  `src-tauri/capabilities/default.json` (`clipboard-manager:allow-write-text`,
  `:allow-read-text`, `:allow-clear`) and rebuild — direct Rust `ClipboardExt` calls generally don't
  require it, but grant it if the platform demands.

- [ ] **Step 5: Manual verification** (deferred to Task 12's end-to-end check, where the Copy
  button exists). Note here that copy must place the value on the OS clipboard and it must be
  cleared ~30s later only if unchanged.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat: add copy_to_clipboard command with backend auto-clear timer"
```

---

## Task 11: Frontend API + simple-entropy helper

**Files:** Modify `src/lib/api.ts`; create `src/lib/strength.ts`

- [ ] **Step 1: Add API bindings** (`api.ts`)

```ts
export type GenerateRequest =
  | { method: "random"; length: number; uppercase: boolean; lowercase: boolean; digits: boolean; symbols: boolean }
  | { method: "memorable"; words: number; separator: string }
  | { method: "pattern"; template: string }
  | { method: "pronounceable"; length: number };

export interface StrengthDto { score: number; guessesLog10: number; label: string; }

// in the api object:
generatePassword: (request: GenerateRequest) => invoke<string>("generate_password", { request }),
analyzeStrength: (password: string) => invoke<StrengthDto>("analyze_strength", { password }),
copyToClipboard: (text: string) => invoke<void>("copy_to_clipboard", { text }),
```

- [ ] **Step 2: Add the simple estimator** (`src/lib/strength.ts`) — the zero-dependency
  `length × log2(poolSize)` formula, computed from which character classes actually appear in the
  string:

```ts
// src/lib/strength.ts
export function simpleEntropyBits(pw: string): number {
  if (pw.length === 0) return 0;
  let pool = 0;
  if (/[a-z]/.test(pw)) pool += 26;
  if (/[A-Z]/.test(pw)) pool += 26;
  if (/[0-9]/.test(pw)) pool += 10;
  if (/[^a-zA-Z0-9]/.test(pw)) pool += 32; // symbols/other
  return pool === 0 ? 0 : Math.round(pw.length * Math.log2(pool));
}

// Map bits -> a 0..4 band + label so the simple and zxcvbn meters render identically.
export function entropyBand(bits: number): { score: number; label: string } {
  const score = bits < 28 ? 0 : bits < 36 ? 1 : bits < 60 ? 2 : bits < 128 ? 3 : 4;
  return { score, label: ["Very weak", "Weak", "Fair", "Strong", "Very strong"][score] };
}
```

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat: add frontend generator API bindings and simple-entropy helper"
```

---

## Task 12: CredentialEditor — generate controls, meter, copy button

**Files:** Modify `src/screens/CredentialEditor.tsx`

Embed the generator **inside the existing modal**, attached to the `secret` field. Keep every
existing convention: per-field `useState`, the `submitting`/`error` pair, the
try/catch/`isCommandError`/`finally` submit idiom, `<Tooltip>`-wrapped buttons,
`disabled={submitting}` + `disabled:opacity-50`.

- [ ] **Step 1: Add generator state and the generate handler**

```tsx
type GenMethod = "random" | "memorable" | "pattern" | "pronounceable";
const [genMethod, setGenMethod] = useState<GenMethod>("random");
const [length, setLength] = useState(20);
const [classes, setClasses] = useState({ uppercase: true, lowercase: true, digits: true, symbols: true });
const [words, setWords] = useState(4);
const [separator, setSeparator] = useState("-");
const [template, setTemplate] = useState("Cvcv-####");
const [genError, setGenError] = useState<string | null>(null);

async function handleGenerate() {
  setGenError(null);
  try {
    const req: GenerateRequest =
      genMethod === "random" ? { method: "random", length, ...classes } :
      genMethod === "memorable" ? { method: "memorable", words, separator } :
      genMethod === "pattern" ? { method: "pattern", template } :
      { method: "pronounceable", length };
    setSecret(await api.generatePassword(req));
  } catch (err) {
    setGenError(isCommandError(err) ? err.message : "Could not generate a password.");
  }
}
```

- [ ] **Step 2: Load the configured algorithm; wire both meters**

On mount, fetch `api.getConfig()` to learn `strengthAlgorithm`. The **simple** meter is derived
synchronously from `secret` (per keystroke) via `simpleEntropyBits`/`entropyBand`. The **zxcvbn**
meter is fetched on a **debounce** (do NOT call per keystroke):

```tsx
const [algorithm, setAlgorithm] = useState("simple");
const [zxcvbn, setZxcvbn] = useState<StrengthDto | null>(null);

useEffect(() => { api.getConfig().then((c) => setAlgorithm(c.strengthAlgorithm)).catch(() => {}); }, []);

useEffect(() => {
  if (algorithm !== "zxcvbn" || secret === "") { setZxcvbn(null); return; }
  const id = setTimeout(() => { api.analyzeStrength(secret).then(setZxcvbn).catch(() => {}); }, 250);
  return () => clearTimeout(id); // debounce: cancel the pending call on each keystroke
}, [secret, algorithm]);

const meter = algorithm === "zxcvbn"
  ? (zxcvbn ?? { score: 0, label: "…" })
  : (() => { const bits = simpleEntropyBits(secret); return { ...entropyBand(bits), bits }; })();
```

- [ ] **Step 3: Render the controls, meter bar, Generate + Copy buttons** near the `secret` input.
  Method selector (`<select>`); the method-specific params (length + 4 class checkboxes for random;
  words + separator for memorable; a template text input for pattern with a one-line legend
  `C/c V/v # ? \literal`; length for pronounceable). A strength bar coloured by `meter.score` with
  `meter.label` (and the entropy bits when in simple mode).
  `{genError && <p className="text-sm text-red-400">{genError}</p>}`. Buttons wrapped in
  `<Tooltip>`, `disabled={submitting}`:

```tsx
<Tooltip text="Generate a password with the selected method">
  <button type="button" onClick={handleGenerate} disabled={submitting}
    className="rounded bg-blue-600 px-3 py-1 disabled:opacity-50">Generate</button>
</Tooltip>
<Tooltip text="Copy to clipboard (auto-clears after 30s)">
  <button type="button" onClick={() => api.copyToClipboard(secret)}
    disabled={submitting || secret === ""} className="px-3 py-1 disabled:opacity-50">Copy</button>
</Tooltip>
```

- [ ] **Step 4: Manual end-to-end verification** (covers Task 10 Step 5 too)

Run: `npm run tauri dev`
Expected, in the Add/Edit Credential modal:
1. Each of the 4 methods generates a password into the field; invalid inputs (length 0, no classes,
   empty pattern) surface a friendly inline error, not a crash.
2. With Settings = **Simple**, the meter updates on every keystroke with an entropy label; with
   Settings = **Advanced (zxcvbn)**, the meter updates shortly after typing stops (debounced) and
   rates known-weak strings (`password`) low.
3. Copy places the password on the OS clipboard; pasting elsewhere within 30s works; ~30s after
   copy the clipboard is cleared **only if** it still holds that password (copy something else in
   between → that value is preserved).
Close the window when confirmed.

- [ ] **Step 5: Full regression**

Run: `cargo test --workspace` (Expected: all pass) and `npm run build` (Expected: `tsc` clean, Vite
build succeeds).

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat: embed password generator, strength meter, and copy-with-auto-clear in CredentialEditor"
```

---

## Out of scope for M2 (do NOT build here)

- **Standalone Generator screen / nav rail** — deferred to **M4**. M2 embeds the generator in the
  existing `CredentialEditor` modal only.
- **Persisted `strength_score` column + reuse/age detection** for the health dashboard — **M4**.
  `analyze_strength` is computed on demand and never stored.
- **HIBP breach check (`check_breach`, k-anonymity client)** — **M4**.
- **Per-class-presence guarantee** in `random`, **caching** of the wordlist, **repeat/range tokens**
  in the pattern DSL, and a **user-configurable clipboard timeout** — all intentionally omitted
  (YAGNI); add only against a concrete future request.

## What we deliberately kept rigorous (not simplified away)

- CSPRNG for all generation (`rand::thread_rng()`, reused from `crypto.rs`) — never a non-crypto
  RNG.
- Generated passwords are `Zeroizing<String>` from birth in core; the clipboard task holds the
  secret in `Zeroizing` and clears with a check-before-clear guard.
- All generation inputs are validated at the trust boundary (`GenError` → `invalid_input`),
  single-sourced in `pwmgr-core::generator`.
- `pwmgr-core` stays serde-free and free of the two new deps; `zxcvbn` and the clipboard plugin
  live in the app crate only.
