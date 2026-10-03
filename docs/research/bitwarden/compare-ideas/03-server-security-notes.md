# Bitwarden Security Model vs. Our Threat Model

Sources: [Bitwarden Security Whitepaper](https://bitwarden.com/help/bitwarden-security-white-paper/), [ETH Zurich cryptography audit writeup](https://bitwarden.com/blog/security-through-transparency-eth-zurich-audits-bitwarden-cryptography/), [Compliance/Audits page](https://bitwarden.com/help/is-bitwarden-audited/). Our ground truth: `docs/design/THREAT_MODEL.md`.

## 1. What Bitwarden publicly documents

**Key hierarchy (account creation/login):**
- Master password + email salt → PBKDF2 (default 600,000 iterations, user-configurable; Argon2id offered as an opt-in alternative post-account-creation) → 256-bit **Master Key**.
- Master Key → HKDF stretch → 512-bit **Stretched Master Key**. Neither Master Key nor Stretched Master Key ever leaves the client or touches Bitwarden servers.
- A separate **Master Password Hash** (PBKDF2-SHA256 over the Master Key, salted with the master password) *is* sent to the server, purely for login authentication — this is the "hash of a hash" split that keeps the vault-decryption secret and the server-auth secret cryptographically distinct.
- Client generates a random 512-bit **Generated Symmetric Key** + 128-bit IV, encrypts it with AES-256 under the Stretched Master Key → **Protected Symmetric Key**, which *is* stored server-side (ciphertext only).
- Each vault item ("Cipher") gets its own random 64-byte **Cipher Key**, encrypted under the user's (or organization's) Symmetric Key. This is per-item envelope encryption — a compromised single Cipher Key only exposes that one item.
- Organizations: a CSPRNG-generated Organization Symmetric Key is wrapped with each member's RSA public key (RSA-OAEP), so sharing doesn't require re-deriving anything from a password.

**Threat model / scope statements:**
- Explicitly zero-knowledge: Bitwarden claims it cannot decrypt vault data server-side even under compromise, because Master Key / Stretched Master Key never transmit.
- Explicitly **out of scope**, stated directly in the whitepaper: compromised client device/OS, keyloggers, malicious browser extensions — "You, as the end-user and/or device owner, are responsible for ensuring devices are secured." This is functionally identical to our O1 (malware/keylogger on the same OS user).
- Browser/web-specific risks called out: phishing, malicious extensions, clickjacking/XSS/CSRF — mitigated with CSP, X-Frame-Options, HSTS.
- No public documentation found of specific server-side brute-force throttling/lockout parameters (attempt counts, backoff curves) — the whitepaper's "Security tools for users" section instead pushes 2FA/lockout-avoidance as a user responsibility, not a documented algorithm. Nothing concrete to adopt here beyond what we already do.
- Self-hosting: mentioned only in passing (compliance page references SOC2/Cure53 audits of the *cloud* offering); no local-only mode comparable to ours — every Bitwarden deployment, including self-hosted, still runs the client/server sync architecture, just against your own server instead of theirs. Not the same category as "no server at all."

**Third-party audits (2022–2025):** Cure53 (source + pentest, no major unresolved issues), IOActive (clients/SDKs), Unit 42/Palo Alto (mobile), Fracture Labs (web app/network), SOC 2 Type 2 + SOC 3, and the 2025 **ETH Zurich Applied Cryptography Group** audit — notable because it explicitly modeled a *fully malicious server* (server actively colluding with attacker, not just breached). Found 10 attacks (12 after publication split), all medium/low severity requiring server-infrastructure control as a precondition. 7 fixed/in remediation, 3 accepted as intentional design tradeoffs necessary for product functionality (details of which 3 aren't in Bitwarden's summary — only in the linked PDF report, not fetched here).

## 2. How our threat model compares

Our `THREAT_MODEL.md` is structurally similar in spirit but the attack surface is entirely different because there is no server to be malicious:

| Bitwarden concern | Our equivalent |
|---|---|
| Malicious/breached server sees only ciphertext | N/A — no server exists in Main/Viewer apps; Web Portal binds `127.0.0.1` only (A4) |
| Client device compromise / keylogger (O1-equivalent, explicitly out of scope for them too) | Same: our O1, same reasoning, same "auto-lock + zeroize" mitigation, not elimination |
| PBKDF2 600k iterations / optional Argon2id | We go straight to Argon2id (OWASP-baseline params) — no legacy PBKDF2 path, no config theater between two algorithms |
| Master Key vs. Stretched Master Key vs. Symmetric Key split (three derivation stages, because two different secrets — server auth vs. vault decryption — must stay independent) | We only need one derived key path: no server to authenticate against separately, so no need for the Master-Key/Master-Password-Hash split. Our A3 (file inspection) + wrong-password-oracle mitigation covers the analogous "don't let anything double as an oracle" goal with less machinery |
| Per-item Cipher Key envelope encryption | We use SQLCipher full-DB encryption + per-field AES-256-GCM — coarser granularity (no per-item key), but also no key-management surface to get wrong. Reasonable tradeoff given no sharing/multi-tenant model to isolate |
| Browser-extension/XSS/CSRF (web client attack surface) | Our A4 CSRF mitigation (double-submit: `SameSite=Strict` cookie + custom header) is arguably *more* concrete/testable than Bitwarden's whitepaper description, which just names CSP/X-Frame-Options generically |
| No documented brute-force/lockout algorithm specifics | We don't have one either (see gaps below) |

## 3. Ideas worth adopting (ranked)

1. **Explicit written statement that device/OS compromise is out of scope, phrased for the user, not just the security doc.** Bitwarden puts this directly in user-facing docs ("you are responsible for..."). We already have O1 in `THREAT_MODEL.md`, but consider surfacing a one-line version in `USAGE_GUIDE.md` or the app's Settings/About screen so end users see it, not just contributors reading the threat model. *Low effort, real value.*
2. **Consider documenting failed-unlock-attempt behavior explicitly**, even though it's a genuine gap on both sides. Neither we nor Bitwarden document a specific throttling curve. Given we're local-only, the honest options are: (a) do nothing extra (Argon2id cost already rate-limits offline attempts; there's no network attacker to lock out locally) or (b) add a small in-memory delay/backoff after N consecutive wrong-password unlock attempts to blunt a *local* automated-guessing script running as the same OS user. This is a genuinely open question, not a clear miss — **recommend deciding explicitly and writing the decision (even "not needed, here's why") into THREAT_MODEL.md** rather than leaving it implicit.
3. **Third-party audit transparency pattern** (not code, a practice): Bitwarden publishes every audit PDF verbatim, including one that found real issues, framed as "here's what was found and fixed." If/when this project ever gets an external review (even a single competent friend doing a focused crypto review), publishing findings + resolution the same way builds the same kind of trust signal cheaply. *Not urgent — no audit has happened yet — but worth keeping in mind for the project's public posture.*

Not adopting: PBKDF2 fallback/dual-KDF support (pure legacy-compat complexity we don't need — we have no existing user base migrating off an older KDF), Cipher-level per-item key wrapping (solves a sharing/multi-tenant problem we don't have), RSA org-key wrapping (no organizations/sharing feature exists or is planned per README).

## 4. Where we already do as well or better

- **Simpler, more auditable key derivation.** One KDF (Argon2id), one derived key, no PBKDF2-legacy branch, no three-stage Master Key/Stretched Master Key split needed — because we don't have Bitwarden's problem of keeping "prove who I am to a server" and "decrypt my vault" cryptographically separate. Fewer moving parts is a real security property here, not just simplicity for its own sake.
- **CSRF mitigation is more concretely specified.** Our double-submit (`SameSite=Strict` + required custom header, A4 in THREAT_MODEL.md) is a specific, testable mechanism; the whitepaper's browser-attack section is descriptive, not prescriptive, about *how* CSRF is actually blocked beyond naming security headers generically.
- **Authenticated encryption by default at the item level** (AES-256-GCM, A5 tamper detection) is equivalent to what Bitwarden gets from AES-CBC+HMAC or GCM in their ciphers — we didn't find a documented gap here, it's parity.
- **No server attack surface at all for Main/Viewer apps**, and a hard-coded (non-configurable) `127.0.0.1` bind for Web Portal — this sidesteps the entire "malicious/breached server" threat class ETH Zurich spent an audit on. Not a design win we should take much credit for (different problem entirely), but worth noting explicitly: our zero-knowledge property is structural (no server exists) rather than cryptographically-enforced-against-an-adversarial-server the way theirs is.

## Not fetched / limits of this research

The full 2025 ETH Zurich PDF report (linked from the blog post) was not fetched — only the summary blog post. If the specific "3 accepted-as-intentional-design" findings matter for future work, that PDF is the next source to pull.
