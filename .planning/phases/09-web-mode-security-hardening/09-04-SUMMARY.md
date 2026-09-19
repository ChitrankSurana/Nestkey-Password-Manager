---
phase: 09-web-mode-security-hardening
plan: 04
subsystem: api
tags: [axum, rust, typescript, throttle, argon2, semaphore, sec-02]
requires:
  - "Session/AppContext shape and integration harness (09-01..09-03)"
provides:
  - "ThrottleStore: 5 attempts then doubling lockout (30s..1h cap), begin_attempt linearizes count+lock, record_success resets"
  - "KdfGuard: KDF_PERMITS=3 non-blocking Argon2 concurrency cap"
  - "throttled_error / rate_limited_error mapped to HTTP 429"
  - "Client fetchOk retries only rate_limited, once"
affects: [09-05]
status: complete
commits: 3
plan_head_before: 6acd2a5cc2bd1af2887b92f91decf1e13e23c843
actuals:
  tokens: 14000
  tasks: 3
  commits: 3
key-files:
  created:
    - web-src-tauri/src/throttle_store.rs
    - web-src-tauri/src/kdf_guard.rs
    - .planning/phases/09-web-mode-security-hardening/deferred-items.md
  modified:
    - core/src/error.rs
    - web-src-tauri/src/error_map.rs
    - web-src-tauri/src/lib.rs
    - web-src-tauri/src/main.rs
    - web-src-tauri/src/handlers.rs
    - web-src-tauri/tests/http_integration.rs
    - src/lib/api.ts
decisions:
  - "W=30s, MAX_WINDOW=1h, KDF_PERMITS=3, threshold 5 (planner-locked)"
  - "unlock_with_pin intentionally left unthrottled (developer decision, T-09-19)"
  - "IP key is machine-wide on loopback (D-09 as written, T-09-22 accepted)"
metrics:
  completed: 2026-09-19
---

# Phase 9 Plan 04: Login/register throttle and bounded Argon2 Summary

`/api/login` and `/api/register_user` now lock a username key and an IP key after five attempts with a doubling window (30s to 1h cap, reset on success), and login/register/password-mode export share a 3-permit non-queueing Argon2 semaphore; both rejections are HTTP 429 with distinct codes (`throttled`, `rate_limited`) and the client retries only the latter.

## Tasks

1. Throttle store, error codes, 429 mapping, login wiring, connect-info serving (commit cfdee8d)
2. Register parity plus reset / independence / concurrent-burst tests (commit 205be4a)
3. KdfGuard, permits in three handlers, code-aware client retry, saturation test (commit ec8143c)

## Deviations from Plan

**1. [Rule 1 - Test assumption] Taken-username register returns 500, not 409**
- **Found during:** Task 2
- **Issue:** Re-registering an existing username fails in `vault::open_vault` (fresh key vs existing vault) before the registry insert, so the route returns 500 rather than 409. Pre-existing in core; out of scope.
- **Fix:** `register_user_route_is_throttled_like_login` asserts each of five attempts is a counted non-200/non-429 failure and the sixth is 429 `throttled`. Logged in `deferred-items.md`.

**2. [Rule 3 - Test harness] Saturation test uses a multi-thread runtime**
- Handlers run Argon2 synchronously, so on `#[tokio::test]`'s single thread requests serialize and the semaphore never saturates. The test uses `flavor = "multi_thread"`; production is already multi-thread.

## Known Stubs

None.

## Threat Flags

None beyond the plan's register (T-09-19 and T-09-22 remain accepted risks; `unlock_with_pin` is unthrottled and uncapped).

## Verification

`cargo test --workspace` green (web crate: 38 integration + 25 unit incl. throttle window math, KDF permit boundary); `npm run build` and `npm test` (40 passed) green.

## Self-Check: PASSED
