# Threat Model

**Last updated:** 2026-07-01

## 1. Assets to Protect

1. The master password (never stored, ever).
2. Derived vault keys (in-memory only while unlocked).
3. Stored credentials — website, username, password, remarks — at rest in the `.vault` file.
4. Backup files.
5. Plaintext CSV exports (transient, user-initiated).

## 2. Adversaries Considered (in scope)

- **A1 — Stolen/lost device, powered off:** attacker has the disk/files but not a live session.
- **A2 — Another local user on a shared machine, no admin rights:** can read files they have OS
  permission to, cannot read another OS user's home directory.
- **A3 — Casual file inspection:** someone opens the `.vault` file in a hex editor or plain
  `sqlite3` CLI without the master password.
- **A4 — Network eavesdropper (future web mode, M6):** someone on the same LAN trying to intercept
  traffic to the localhost-bound web server.
- **A5 — Tampering at rest:** a file is modified (corrupted or maliciously altered) between writes.

## 3. Adversaries Explicitly Out of Scope

- **O1 — Malware/keylogger running as the same OS user while the vault is unlocked.** If something
  can read process memory or keystrokes on your own logged-in session, no local password manager can
  fully defend against it. Mitigation is limited to minimizing the unlock window (auto-lock) and
  zeroizing key material promptly — not eliminating the risk.
- **O2 — Physical coercion / shoulder-surfing.**
- **O3 — A compromised OS or firmware (rootkit).**
- **O4 — Supply-chain compromise of a dependency.** Mitigated only informally (dependency checker,
  pinned versions, minimal dependency set) — not a full SBOM/reproducible-build program in v1.

## 4. Mitigations by Threat

| Threat | Mitigation |
|---|---|
| A1/A3 file theft or inspection | SQLCipher full-DB encryption + per-field AES-256-GCM; Argon2id makes offline brute-force expensive; master password never on disk so there's nothing to steal but ciphertext |
| A1 weak master password | Real-time strength/entropy feedback at registration nudges toward strong passwords (cannot be enforced without also being annoying — a minimum-entropy warning is shown, not a hard block, to avoid locking the user out of their own choice) |
| A2 shared machine | Per-user vault files with OS-level file permissions; one user's vault is unreadable without both filesystem access AND their master password |
| A4 web mode eavesdropping | Server binds to `127.0.0.1` only (never `0.0.0.0`), so LAN traffic never occurs for this feature; session token required per request |
| A5 tampering | AES-GCM is authenticated — any bit-flip in ciphertext or nonce causes decryption to fail loudly rather than returning corrupted plaintext |
| Backup file theft | Backups are encrypted with the same scheme as the live vault, never plaintext |
| Plaintext CSV export | Gated behind an explicit typed confirmation (not just a checkbox) warning the file will be unencrypted; export path is logged in-app (locally) so the user has a record they did it |
| HIBP breach check leaking passwords | k-anonymity: only first 5 hex chars of SHA-1(password) sent; HIBP cannot reconstruct the password from that; feature is off by default and requires explicit opt-in |
| Wrong-password oracle | Login failure returns one generic message regardless of whether the username, password, or vault file itself is the problem — no distinguishing signal |
| Key lingering in memory | `zeroize` on lock/logout/exit; auto-lock after configurable idle period (default 5 min) bounds the exposure window from O1 |

## 5. Residual Risks (accepted)

- If O1 (malware with memory access) occurs during the unlocked window, credentials in active use are
  exposed — this is true of every password manager and is documented rather than hidden.
- Argon2id parameters trade off unlock speed vs. brute-force cost; v1 targets OWASP-baseline params
  tuned to ≤1s unlock on typical hardware — a determined attacker with the ciphertext and unlimited
  offline compute is slowed, not stopped forever.
- The read-only viewer and web mode both reuse the same crypto core, so a vulnerability found in
  `crypto`/`vault` affects all surfaces equally — isolation limits blast radius of *other* modules'
  bugs, not this one.

## 6. Verification of This Model

Each mitigation above should have a corresponding automated test or manual check listed in the TRD
§7 testing requirements before the relevant milestone is considered done (e.g., "tamper detection"
mitigation ⇒ a test that flips a ciphertext bit and asserts decryption fails).
