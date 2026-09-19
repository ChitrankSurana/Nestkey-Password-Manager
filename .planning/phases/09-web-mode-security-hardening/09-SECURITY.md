---
phase: "09"
slug: web-mode-security-hardening
status: verified
threats_open: 0
asvs_level: 1
created: "2026-09-19"
---

# Phase 09 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| Browser → `pwmgr-web` HTTP API | Untrusted request bodies (JSON and raw file bytes) from anything reaching 127.0.0.1:7373 | Credentials, vault exports, backups |
| Session A → Session B | Two browser sessions share one server process | Cached uploads |
| `pwmgr-core` → local filesystem | Former H1 arbitrary read/write primitive, severed for the web branch | Vault files, exports |
| Pre-auth routes → Argon2 KDF | Each crossing allocates 64 MiB | Master password, PIN |
| Injected script → DOM / IPC | XSS reaching decrypted secrets; CSP is the boundary | Decrypted secrets |

---

## Threat Register

Severity/disposition per the 09-01..09-05 PLAN threat models. All 34 register rows plus five T-09-SC rows verified 2026-09-19.

| Threat ID | Category | Component | Severity | Disposition | Mitigation (evidence) | Status |
|-----------|----------|-----------|----------|-------------|-----------------------|--------|
| T-09-01 | Tampering | `export_csv` web handler (H1) | critical | mitigate | Request has no destination; handler returns bytes; `export_csv_writes_no_server_side_file_for_a_crafted_destination` | closed |
| T-09-02 | Info Disclosure | Plaintext export to chosen path | critical | mitigate | Plaintext gate in `do_export_csv_bytes`; rejection test | closed |
| T-09-03 | Info Disclosure | Export bytes persisted server-side | high | mitigate | No fs access in handlers; fixed log label | closed |
| T-09-04 | Elevation | Desktop `export_csv` reshaped | medium | mitigate | Thin wrapper retained, command still registered | closed |
| T-09-05 | DoS | Oversized export response | low | accept | Bounded by user's own vault | closed |
| T-09-06 | Info Disclosure | CSV read primitive (H1) | critical | mitigate | Path field removed; reads session cache | closed |
| T-09-07 | Info Disclosure | Cross-session upload read | high | mitigate | Cache on `UserSession`; isolation test | closed |
| T-09-08 | Info Disclosure | Upload persisted or echoed | high | mitigate | `Zeroizing` in-memory cache, fixed error messages | closed |
| T-09-09 | DoS | Unbounded upload | medium | mitigate | 25 MiB per-route body limit; test | closed |
| T-09-10 | Tampering | Concurrent upload splice | medium | mitigate | Single-slot replace under one mutex; test | closed |
| T-09-11 | DoS | Upload cache accumulation | low | accept | One slot per session, 24h prune | closed |
| T-09-12 | Tampering | `restore_backup` arbitrary path (H1) | critical | mitigate | Path field removed; no-client-destination gate test | closed |
| T-09-13 | Tampering | Hostile bundle corrupts vault | high | mitigate | `decode_bundle` before write; atomic ordering kept | closed |
| T-09-14 | Tampering | Overwrite without consent | high | mitigate | `confirm_required` 409 before write; frontend confirm | closed |
| T-09-15 | Info Disclosure | Backup bytes persisted/echoed | high | mitigate | Zeroizing cache, sanitized errors, cleared on success | closed |
| T-09-16 | DoS | Oversized backup upload | medium | mitigate | Same per-route body limit | closed |
| T-09-17 | Spoofing | Restore for other username | medium | accept | Pre-existing, same as desktop; own master password still required | closed |
| T-09-18 | Spoofing | Online brute force of login (M1) | high | mitigate | Username + IP throttle, 5 attempts then doubling lockout | closed |
| T-09-19 | Spoofing | PIN brute force via `unlock_with_pin` | high | accept | Developer declined throttling; known gap, see Accepted Risks | closed |
| T-09-20 | DoS | Argon2 memory flood (M2) | high | mitigate | `KdfGuard` 3 permits, non-blocking | closed |
| T-09-21 | Spoofing | Concurrent burst past threshold | medium | mitigate | Check+increment in one write lock; test | closed |
| T-09-22 | DoS | Loopback IP key machine-wide lockout | medium | accept | Locked decision D-09; bounded to 1h, cleared on success | closed |
| T-09-23 | Info Disclosure | Username enumeration via throttle | medium | mitigate | Identical messages/keys regardless of account existence | closed |
| T-09-24 | DoS | Panic poisons throttle map | medium | mitigate | `saturating_mul`, no panicking arithmetic | closed |
| T-09-25 | Info Disclosure | Credentials in logs/429 body | medium | mitigate | No logging on credential path; fixed strings | closed |
| T-09-26 | DoS | Unbounded throttle map | low | accept | Swept every 2s, pruned after 24h | closed |
| T-09-27 | Elevation | Desktop null CSP (M3) | high | mitigate | Strict CSP in `tauri.conf.json`, byte-identical to web policy | closed |
| T-09-28 | Tampering | XSS in web SPA (M3) | high | mitigate | Strict CSP on every response, no unsafe-* | closed |
| T-09-29 | Tampering | Rejected response skips headers | high | mitigate | Outermost layer; six-shape test | closed |
| T-09-30 | Spoofing | Clickjacking | medium | mitigate | `X-Frame-Options: DENY` + `frame-ancestors 'none'` | closed |
| T-09-31 | Tampering | MIME sniffing | medium | mitigate | `X-Content-Type-Options: nosniff` | closed |
| T-09-32 | Tampering | Dev CSP in release build | medium | mitigate | Relaxation only in `tauri.dev.conf.json`, used by `tauri:dev` only | closed |
| T-09-33 | Info Disclosure | Referrer leakage | low | mitigate | `Referrer-Policy: no-referrer` | closed |
| T-09-34 | DoS | Strict CSP breaks an asset | low | accept | Covered by UAT sweep (Tests 1 and 4 passed) | closed |
| T-09-SC (x5) | Tampering | Dependency supply chain | low | accept | No new cargo/npm dependencies; lockfile changes are version bumps only | closed |

*Status: open · closed · open — below high threshold (non-blocking)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-09-01 | T-09-19 | `unlock_with_pin` left unthrottled and uncapped; scope kept to the three routes in D-07/D-13. Follow-up recommended. | Developer (plan 09-04) | 2026-09-19 |
| AR-09-02 | T-09-22 | Loopback IP key is machine-wide by locked decision D-09; bounded by 1h max window | Developer (plan 09-04) | 2026-09-19 |
| AR-09-03 | T-09-05, -11, -17, -26, -34, -SC | Low/medium risks accepted in PLAN threat models | Developer (plans 09-01..09-05) | 2026-09-19 |

---

## Unregistered Flags (auditor warnings, not counted in threats_open)

- **W1 — username used as a path component.** `do_restore_backup_from_bytes` builds `vault_path(username)` from the uploaded bundle's username, which is only UTF-8 validated. A crafted bundle with separators or `..` in the name could write a `.vault`-suffixed file outside the vaults dir. Requires an authenticated session and `confirmed: true`. Same pattern exists in `do_register` and `do_login` and pre-dates phase 09. Recommended follow-up: restrict usernames to a safe charset or reject separators and `..` in `vault_path`.
- **W2 — plaintext copies on the heap.** `csv_upload()`/`backup_upload()` return plain `Vec<u8>` clones and handlers hold non-zeroized strings for the request lifetime. Declared T-09-08 mitigations are met.
- **W3 — no integration test for `/api/create_backup_download`.** Added after the plans; it sits in the protected router, takes no client input and writes no server file. Recommended: add a test.
- Post-plan additions reviewed with no new unmitigated threat: `/api/create_backup_download`, frontend `pwmgr-session-locked` event.

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-09-19 | 39 | 39 | 0 | gsd-security-auditor (ASVS L1, block_on high) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-09-19
