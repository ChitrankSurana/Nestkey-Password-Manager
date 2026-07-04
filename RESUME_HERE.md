# Resume Notes — Password Manager M1 Secure Core

**Written:** 2026-07-04, mid-session, because the session was freezing.
**Read this first in a new session before doing anything else.**

## Where everything lives

- **Project root:** `E:\Coding_Projects-CS\Account_1\Password_Manager` — its own standalone git repo
  (NOT part of the parent `Account_1` repo one level up — that was a deliberate choice; the parent
  repo has other unrelated projects and a Windows file-ownership quirk).
- **`main` branch:** design docs only (`docs/PRD.md`, `TRD.md`, `ARCHITECTURE.md`, `THREAT_MODEL.md`,
  `USAGE_GUIDE.md`, `README.md`).
- **Active work branch:** `m1-secure-core`, checked out in a git worktree at
  `E:\Coding_Projects-CS\Account_1\Password_Manager\.worktrees\m1-secure-core`. **All the Rust code
  lives here, not on `main`.** `cd` into this worktree path to continue.
- **The plan being executed:** `docs/superpowers/plans/2026-07-01-m1-secure-core.md` (present in both
  `main` and the worktree). This is a 13-task TDD plan for `pwmgr-core`, a standalone Rust library
  crate (at `core/` inside the worktree) that will later be consumed by a Tauri app (a separate,
  not-yet-written "M1 Phase B" plan).
- **No GitHub remote exists.** Nothing has been pushed anywhere. Everything is local commits only.

## Toolchain (already installed, but NOT on PATH for fresh shells)

Rust (rustup), MSVC C++ Build Tools, and Strawberry Perl (needed for SQLCipher's vendored-OpenSSL
build) are all installed on this machine, but new shell processes in this environment don't pick them
up automatically. **Every PowerShell command that runs `cargo`/`rustc` must start with:**

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;C:\Strawberry\perl\bin;C:\Strawberry\c\bin;" + $env:PATH
Set-Location "E:\Coding_Projects-CS\Account_1\Password_Manager\.worktrees\m1-secure-core\core"
```

Bash cannot find `cargo` at all in this environment — use PowerShell for all cargo/rustc commands,
Bash only for `git`.

## How this has been executed so far

Using the `superpowers:subagent-driven-development` workflow: for each of the plan's 13 tasks, I
(the controller) dispatch a fresh implementer subagent with the full task text (TDD: write failing
test, watch it fail, implement, watch it pass, commit), then a spec-compliance reviewer subagent
(verifies the code actually matches the plan, doesn't trust the implementer's self-report), then a
code-quality reviewer subagent (looks for real bugs/security issues beyond spec compliance). When a
reviewer finds a real issue, I update the plan doc with the fix, then resume the SAME implementer
agent (via the `SendMessage` tool, using its agent ID) with instructions to apply it, then re-verify.
Every task and every fix is its own git commit, so `git log --oneline` in the worktree is a reliable,
detailed history of exactly what happened and why.

## Progress: Tasks 1-10 are DONE (fully implemented, reviewed, fixed, re-verified)

1. Crate scaffold (`pwmgr-core`, deps)
2. `crypto.rs` — Argon2id key derivation (found+fixed: `VaultKey`'s byte field was `pub`, made private with an `as_bytes()` accessor to protect zeroization)
3. `crypto.rs` — AES-256-GCM seal/open
4. `crypto.rs` — constant-time verifier compare
5. `registry.rs` — unencrypted username/salt lookup table
6. `vault.rs` — SQLCipher open + wrong-key rejection (found+fixed: `PRAGMA foreign_keys` was never enabled; `credentials.strength_score` column from the TRD's data model was missing; key hex string wasn't zeroized)
7. `vault.rs` — site CRUD + soft delete
8. `vault.rs` — credential CRUD, AES-GCM-sealed secrets (found+fixed: a DB-corrupted nonce could panic the whole read path instead of returning an error)
9. `vault.rs` — case-insensitive search (found+fixed: user's raw search query wasn't escaped for SQL `LIKE` metacharacters `%`/`_`, so literal percent signs or underscores in a search term behaved as wildcards instead of literal characters)
10. `auth.rs` — `register_user` (found+fixed: registry row was committed BEFORE the vault file was created, so a vault-creation failure left a permanently orphaned, un-loginable username; reordered so vault creation happens first)

**Current confirmed state at Task 10's completion: 30/30 tests passing** (9 crypto + 4 registry +
12 vault + 5 auth). Last commit at that point: `ba69c81` "feat(auth): add login with verifier check
and generic-error rejection" — wait, that's actually Task 11's commit message; Task 10's own final
commit was `2e648e4`. Check `git log --oneline` to see the exact current HEAD; more commits may have
landed after this note was written if the session recovered on its own.

## Task 11 (`login`) — IN PROGRESS, this is where the freeze happened

- **Implementation: DONE**, committed as `ba69c81` ("feat(auth): add login with verifier check and
  generic-error rejection"). All 5 auth tests + full 30-test suite passed at that point.
- **Spec compliance review: PASSED** — independently confirmed every failure path in `login` maps to
  the same generic `AuthError::InvalidCredentials` (the anti-timing-oracle security property from
  `docs/THREAT_MODEL.md` §4), no `?`-operator leak to a more specific error type.
- **Code quality review: WAS RUNNING, DID NOT FINISH.** This is the actual work in flight when the
  freeze happened. I had specifically asked this reviewer to trace through a suspected real bug:

### The suspected bug (independently confirmed by two separate fact-check subagents, NOT yet fixed)

**If a user's vault file is deleted, moved, or otherwise missing, `login()` with the CORRECT password
will silently succeed into a brand-new, EMPTY vault instead of failing — masking data loss.**

Why: `vault::open_vault` (in `core/src/vault.rs`) calls `rusqlite::Connection::open(path)`, which
(per SQLite's default `SQLITE_OPEN_CREATE` behavior) silently creates a new empty file if none exists
at that path — it never returns an error for "file not found." Then it applies `PRAGMA key` and runs
`SELECT count(*) FROM sqlite_master` as its "did the key work" check — but on a brand-new empty file
there is no existing encrypted content to validate the key against, so this check trivially succeeds
for ANY key, correct or not. `open_vault` never calls anything like `path.exists()` to distinguish
"first-time creation" from "reopening a real existing vault." Both two fact-check subagents (one
researching rusqlite/SQLite/SQLCipher behavior in general, one reading `vault.rs` directly) confirmed
this reasoning independently.

In `login()`'s actual flow: the password is verified against the REGISTRY's stored `verifier_hash`
(not the vault file), so a correct password passes that check regardless of whether the vault file
exists. The sentinel decrypt check is also against registry-stored ciphertext, not the vault file, so
it also passes. Only THEN does `login()` call `vault::open_vault(vault_path, &vault_key)` — and if that
path doesn't exist, it silently creates a fresh empty vault and returns `Ok`, rather than the intended
`Err(AuthError::InvalidCredentials)`.

### The fix (not yet applied — do this first when resuming)

In `login()`, add a path-existence check before calling `vault::open_vault`, mapping a missing file to
the same generic error used everywhere else in this function:

```rust
if !vault_path.exists() {
    return Err(AuthError::InvalidCredentials);
}
vault::open_vault(vault_path, &vault_key).map_err(|_| AuthError::InvalidCredentials)?;
```

This preserves the "every failure path returns the same generic error" security property (per
THREAT_MODEL.md §4) while closing the data-loss-masking gap. Add a test proving it: register a user,
delete/rename the vault file on disk, then attempt `login` with the CORRECT password and assert it
returns `Err(AuthError::InvalidCredentials)` rather than `Ok`.

**Consider also**: this same underlying issue (`open_vault` can't tell "just created" from "genuinely
reopened") might be worth a note in `docs/THREAT_MODEL.md` or a `vault.rs` doc comment even after the
`login`-level fix, since any FUTURE caller of `vault::open_vault` directly (not through `login`) could
hit the same footgun. Use judgement on whether that's worth doing now vs. deferring — the `login`-level
fix is the one that actually matters for now since it's the only current caller exposed to an
untrusted/wrong path.

## What's left after Task 11 is closed out

- **Task 12:** `session.rs` — in-memory `VaultKey` holder with idle-timeout auto-lock (injectable
  `Clock` trait for testability). New file, needs `pub mod session;` added to `lib.rs`.
- **Task 13:** `core/tests/end_to_end.rs` — the integration test that mirrors the M1 milestone's
  "Verify" scenario (register → add → search → soft-delete/restore → lock → wrong-password-rejected →
  correct-password-reopen → data intact), plus a test proving the on-disk file contains no plaintext.
- **Final step:** dispatch a full-crate code reviewer (not per-task, the whole `pwmgr-core` crate), then
  use the `superpowers:finishing-a-development-branch` skill to decide how `m1-secure-core` merges back
  into `main` (the user hasn't been asked this yet — don't assume, ask when you get there).
- **After that:** M1 Phase A (this secure core) is done. M1 Phase B (Tauri scaffold + IPC commands +
  React login/vault UI + dark theme + tooltips) is a SEPARATE plan that still needs to be written via
  the `writing-plans` skill — it was deliberately deferred so Phase A could be its own small,
  independently-testable unit. See the "Post-Approval Next Steps" section of the plan doc.

## Deferred/tracked items (intentional, not bugs)

- `Credential.secret` (in `vault.rs`) isn't wrapped in `zeroize::Zeroizing` yet — deliberately deferred
  to the M1 Phase B UI/IPC layer (there's a `TODO(M1 Phase B)` comment on it already).
- Argon2id params (64 MiB / 3 iterations / 4 lanes) are the OWASP-baseline starting point, not yet
  benchmarked/tuned on real hardware — flagged in the TRD as a known follow-up.

## Task tracker state (as of freeze)

Use `TaskList` to see current status; as of this note: Tasks 1-10 completed, Task 11 in_progress
(implementation+spec review done, code-quality review was running when the freeze hit), Tasks 12-13
and the final-review task still pending.
