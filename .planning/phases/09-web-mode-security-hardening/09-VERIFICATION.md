---
phase: 09-web-mode-security-hardening
verified: 2026-09-19T12:00:00Z
status: human_needed
score: 3/3 must-haves verified
behavior_unverified: 0
overrides_applied: 0
human_verification:
  - test: "Run `npm run tauri:dev` and `npm run tauri:build` (launch the binary); open every screen with DevTools console open"
    expected: "No CSP violation messages; app renders and works in both dev (overlay CSP) and production binary"
    why_human: "CSP violations are only observable in a real webview console; not automatable here"
  - test: "Open web mode in a browser, visit every screen with DevTools console open (production CSP)"
    expected: "No CSP violations; export downloads a file; import/restore upload works"
    why_human: "Visual/runtime browser behavior (09-05 planned human-check, not performed by executor)"
---

# Phase 9: Web Mode Security Hardening Verification Report

**Goal:** Localhost web server can no longer read/write arbitrary files, be brute-forced, or exhaust memory, and ships baseline security headers.
**Status:** human_needed (only the 09-05 DevTools/console check remains)
**Re-verification:** No

## Observable Truths

| # | Truth (ROADMAP SC) | Status | Evidence |
|---|---|---|---|
| 1 | Web export/import/restore accept no client-supplied path (SEC-01; D-01 stronger than filename-confinement) | VERIFIED | `web-src-tauri/src/handlers.rs`: request structs have no `path` field (grep `path:` returns nothing); export returns bytes + Content-Disposition; import/restore read bytes from per-session upload cache via `/api/upload_csv`, `/api/upload_backup` with `DefaultBodyLimit`; create_backup uses `None` path. Integration tests incl. `export_csv_writes_no_server_side_file_for_a_crafted_destination`, `uploads_are_isolated_between_two_browser_sessions` pass. Desktop path flow retained (D-02). |
| 2 | Login/register throttled; Argon2 bounded (SEC-02) | VERIFIED | `throttle_store.rs` + per-user and per-IP keys checked before KDF (`handlers.rs` 266-301); `kdf_guard.rs` semaphore (3 permits) acquired in login/register/unlock/withPassword export. Tests `five_failed_logins_lock_the_key_out_with_429`, `two_usernames_have_independent_throttle_counters`, `a_successful_login_resets_the_failure_counter` pass. |
| 3 | Strict CSP + nosniff on every web response; desktop no longer `csp: null` (SEC-03) | VERIFIED (code); runtime UI check human | `security_headers_middleware` is outermost layer (handlers.rs 845-848), sets CSP, nosniff, Referrer-Policy, X-Frame-Options, covering fallback static service and rejections. `src-tauri/tauri.conf.json` has the byte-identical strict CSP; `tauri.dev.conf.json` dev overlay. Test `security_headers_are_present_on_every_response_shape` passes. |

**Score:** 3/3 truths verified in code.

## Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|---|---|---|---|
| Rust suite | `cargo test --workspace` | all `test result` lines 0 failed (396 passed) | PASS |
| Frontend suite | `npm test` | 7 files, 40 tests passed | PASS |

## Requirements Coverage

| Req | Status | Note |
|---|---|---|
| SEC-01 | SATISFIED | see truth 1 |
| SEC-02 | SATISFIED | see truth 2 |
| SEC-03 | SATISFIED in code | REQUIREMENTS.md checkbox/traceability still "Pending" - update after human check |

## Anti-Patterns

None blocking. Only inline style usage is React `style={{width}}` in `PasswordGeneratorPanel.tsx:186` (CSSOM-set, permitted under `style-src 'self'`; confirm in the human check). Debt marker scan not exhaustive; none observed.

## Notes

- Pre-existing bug logged in `deferred-items.md`: registering a taken username returns 500 instead of 409 (out of SEC-02 scope).
- ROADMAP SC1 wording (filename confinement) is superseded by CONTEXT D-01 (no path at all) - stronger, documented in ROADMAP scope note.
- `covered_files`/`covered_digest` omitted: gsd-tools unavailable in this environment.

## Human Verification Required

1. **tauri:dev / tauri:build** - launch both, click through every screen with DevTools open. Expected: no CSP violations. Why human: webview console only.
2. **Web mode in browser** - same sweep on the production CSP; test export download and import/restore upload. Why human: runtime browser behavior.

---
_Verified: 2026-09-19_
_Verifier: Claude (gsd-verifier)_
