---
phase: 09-web-mode-security-hardening
plan: 05
subsystem: security
tags: [axum, csp, security-headers, tauri, sec-03]
requires:
  - "build_router + CSRF layer + throttle (09-01..09-04)"
provides:
  - "security_headers_middleware + CSP_POLICY: CSP, nosniff, no-referrer, X-Frame-Options DENY on every web response"
  - "Real CSP in src-tauri/tauri.conf.json (byte-identical to CSP_POLICY)"
  - "src-tauri/tauri.dev.conf.json dev-only overlay + tauri:dev / tauri:build scripts"
affects: []
status: complete
commits: 2
plan_head_before: 35f9f48
actuals:
  tokens: 9000
  tasks: 2
  commits: 2
key-files:
  created:
    - src-tauri/tauri.dev.conf.json
  modified:
    - web-src-tauri/src/middleware.rs
    - web-src-tauri/src/handlers.rs
    - web-src-tauri/tests/http_integration.rs
    - src-tauri/tauri.conf.json
    - package.json
    - README.md
decisions:
  - "D-17 resolution (09-RESEARCH Open Question 1): no dev/prod signal existed, so a Tauri --config JSON Merge Patch overlay (tauri.dev.conf.json) applied only by npm run tauri:dev was introduced."
  - "Header layer added last in build_router chain (outermost), inserts unconditionally after next.run."
---

# Phase 09 Plan 05: Strict CSP and security headers Summary

Strict CSP plus nosniff, no-referrer and X-Frame-Options DENY are stamped on every web response by an outermost axum layer, and the desktop shell's null CSP is replaced by the same string with a dev-only overlay for `tauri dev`.

## Tasks

1. Web headers (`526f679`): `CSP_POLICY`, `security_headers_middleware`, attached outside CSRF; unit test (CSRF 403 carries CSP) and `security_headers_are_present_on_every_response_shape` covering authenticated 200, static fallback, CSRF 403, 401, throttled 429, bodyless OPTIONS.
2. Desktop config (`4f39497`): real CSP in `tauri.conf.json`, `tauri.dev.conf.json` overlay (unsafe-eval, unsafe-inline style, HMR websockets), `tauri:dev`/`tauri:build` scripts.

Verification: `cargo test --workspace` green, `npm run build` and `cargo build -p pwmgr-web` succeed, all node config gates pass (policy string identical in both places, no `unsafe-` in production, overlay has only `app`, only dev script references overlay).

## Deviations from Plan

**1. [Rule 2 - Missing critical] README dev command**
- README documented `npm run tauri dev`, which would now run the strict policy in development and bypass the overlay. Changed to `npm run tauri:dev`. Commit `4f39497`.

## Pending Human Verification

The plan's `<human-check>` (browser DevTools sweep of all screens for CSP violations, `npm run tauri:dev`, `npm run tauri:build` launch) was not performed by the executor; it needs a person. Any blocked resource is fixed in the production policy (both `CSP_POLICY` and `tauri.conf.json`) or, for dev-only breakage, in the overlay.

## Known Stubs

None.

## Self-Check: PASSED

Files and commits `526f679`, `4f39497` verified present.
