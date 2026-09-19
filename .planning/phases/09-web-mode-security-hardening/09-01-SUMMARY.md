---
phase: 09-web-mode-security-hardening
plan: 01
subsystem: api
tags: [axum, tauri, rust, typescript, csv-export, content-disposition, sec-01]

# Dependency graph
requires: []
provides:
  - "do_export_csv_bytes / do_export_csv_download path-free export byte producers (core/src/commands.rs)"
  - "web-mode export_csv handler returning bytes with Content-Disposition, no destination field"
  - "httpDownload web transport helper (src/lib/api.ts) for the CSV-import/backup-restore plans (09-02, 09-03) to model"
affects: [09-02, 09-03]

# Actuals (#2632)
actuals:
  tokens: 4076
  tasks: 2
  commits: 2

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Path-free byte-producer core function + thin path-writing wrapper (do_export_csv_bytes -> do_export_csv_download / do_export_csv)"
    - "isTauri-branched transport dispatch for file-transfer screens (httpDownload alongside invoke)"

key-files:
  created: []
  modified:
    - core/src/commands.rs
    - web-src-tauri/src/handlers.rs
    - web-src-tauri/tests/http_integration.rs
    - src/lib/api.ts
    - src/screens/CsvExport.tsx

key-decisions:
  - "Plaintext web-mode exports log a fixed <web download> label instead of a filesystem destination, preserving the DATA-02 log-every-plaintext-export behavior without ever writing a path"
  - "export_csv HTTP response uses a raw two-header + bytes IntoResponse tuple rather than axum::body::Body::builder, matching the codebase's existing header-injection idiom (issue_session's Set-Cookie insert)"

patterns-established:
  - "Web-only file-download helper (httpDownload) sits beside httpPost in api.ts and is called directly, bypassing call()'s isTauri dispatch, since desktop's invoke() and web's fetch() now take different argument shapes for the same command (path vs no path)"

requirements-completed: [SEC-01]

coverage:
  - id: D1
    description: "Web-mode export_csv accepts no destination field and returns export bytes with a Content-Disposition attachment header for all three modes (thisDevice/withPassword/plaintext)"
    requirement: "SEC-01"
    verification:
      - kind: integration
        ref: "web-src-tauri/tests/http_integration.rs#export_csv_returns_bytes_for_every_mode"
        status: pass
      - kind: integration
        ref: "web-src-tauri/tests/http_integration.rs#export_csv_returns_the_file_bytes_in_the_response_body"
        status: pass
    human_judgment: false
  - id: D2
    description: "A crafted extra destination field in the export_csv request body creates no file on the server filesystem"
    requirement: "SEC-01"
    verification:
      - kind: integration
        ref: "web-src-tauri/tests/http_integration.rs#export_csv_writes_no_server_side_file_for_a_crafted_destination"
        status: pass
    human_judgment: false
  - id: D3
    description: "Plaintext export over the web transport still requires confirmed=true and still appends a plaintext-export log entry (with a <web download> label instead of a path)"
    requirement: "SEC-01"
    verification:
      - kind: integration
        ref: "web-src-tauri/tests/http_integration.rs#export_csv_plaintext_without_confirmation_is_rejected"
        status: pass
      - kind: integration
        ref: "web-src-tauri/tests/http_integration.rs#plaintext_web_export_logs_the_download_label"
        status: pass
    human_judgment: false
  - id: D4
    description: "Desktop export_csv Tauri command is unchanged: still takes a Path from the native save dialog and do_export_csv still writes that file"
    requirement: "SEC-01"
    verification:
      - kind: unit
        ref: "core/src/commands.rs#export_csv_this_device_mode_produces_a_sealable_file_decryptable_with_the_session_key (and 5 other pre-existing do_export_csv tests, unedited)"
        status: pass
    human_judgment: false
  - id: D5
    description: "A web-mode user clicking Export in CsvExport.tsx gets a browser download with no save dialog, naming no server-side path"
    verification: []
    human_judgment: true
    rationale: "Requires manually clicking Export in a running web-mode session and observing the browser's native download prompt/file - not exercised by the automated HTTP/unit suites."

duration: ~35min
completed: 2026-09-19
status: complete
---

# Phase 09 Plan 01: Web-Mode CSV Export Redesign Summary

**Web-mode `export_csv` now returns bytes with a `Content-Disposition` header instead of writing a server-side file — the client can no longer name any export destination.**

## Performance

- **Duration:** ~35 min
- **Started:** 2026-09-18T19:00:00Z (approx.)
- **Completed:** 2026-09-18T19:38:19Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments
- Extracted `do_export_csv_bytes` (path-free byte producer) and `do_export_csv_download` (web-mode wrapper that logs plaintext exports under a fixed `<web download>` label) out of the former path-writing `do_export_csv` in `core/src/commands.rs`; `do_export_csv` itself becomes a thin wrapper so desktop behavior is provably unchanged.
- Rewrote the web `export_csv` handler to drop its `path` request field entirely and return the export bytes with a `Content-Disposition: attachment; filename="..."` header — the handler performs zero filesystem access.
- Added a web-only `httpDownload` transport helper in `src/lib/api.ts` (blob + programmatic `<a download>` click) and branched `api.exportCsv` on `isTauri` since desktop and web now legitimately take different argument shapes.
- Branched `CsvExport.tsx::handleExport` on `isTauri`, following `Backup.tsx::handleCreate`'s established shape: desktop keeps the native save dialog, web mode skips it and shows "Export downloaded."
- Added 5 new HTTP integration tests proving: bytes come back for all three modes with a content-disposition header, withPassword's ciphertext differs from thisDevice's (fresh salt), unconfirmed plaintext is rejected with `invalid_input`, a crafted extra destination field in the request body creates no file anywhere on the server filesystem, and a confirmed plaintext export's log entry records the `<web download>` label rather than a path.

## Task Commits

Each task was committed atomically:

1. **Task 1: Export downloads end to end — browser click to saved file, no server path anywhere** - `9314271` (feat)
2. **Task 2: Prove the export path is gone — all three modes, confirmation gate, and a no-file-written negative test** - `a78c81b` (test)

_Note: Task 1's commit already included the plan's core/handler/frontend rewrite plus one basic HTTP proof test (`export_csv_returns_the_file_bytes_in_the_response_body`), matching its own acceptance criteria; Task 2 added the four deeper mode/negative-test-coverage tests on top._

## Files Created/Modified
- `core/src/commands.rs` - Added `do_export_csv_bytes` (no filesystem access) and `do_export_csv_download` (web-mode wrapper); `do_export_csv` reduced to a thin path-writing wrapper over `do_export_csv_bytes`.
- `web-src-tauri/src/handlers.rs` - `ExportCsvRequest` drops its `path` field; `export_csv` handler returns `Response` with bytes + `Content-Disposition` instead of `StatusCode`.
- `web-src-tauri/tests/http_integration.rs` - 5 new integration tests covering the positive (bytes returned, all modes) and negative (no server-side file for a crafted destination) SEC-01 evidence for export.
- `src/lib/api.ts` - New `httpDownload` helper; `exportCsv` wrapper branches on `isTauri` with divergent desktop/web argument shapes.
- `src/screens/CsvExport.tsx` - `handleExport` branches on `isTauri`; web mode skips the save dialog and reports "Export downloaded."

## Decisions Made
- Plaintext web-mode exports log a fixed `<web download>` label in place of a filesystem destination, preserving the existing plaintext-export-log behavior (DATA-02) without ever recording or requiring a path.
- The HTTP response for `export_csv` is built via a `([(HeaderName, String); 2], Vec<u8>).into_response()` tuple rather than manually constructing an `axum::body::Body` — smaller diff, and axum's own `IntoResponse` impl for header-array + body tuples already covers this shape.
- Task 1's own acceptance criteria required one HTTP integration test as evidence (`export_csv_returns_the_file_bytes_in_the_response_body`); Task 2 then added the four deeper tests (every mode, confirmation gate, no-file-written negative, plaintext log label) as its own commit, keeping the split between "the feature works" (Task 1) and "the feature is proven path-free under adversarial input" (Task 2) visible in git history.

## Deviations from Plan

None — plan executed exactly as written. All artifacts, acceptance criteria, and verification commands from `09-01-PLAN.md` were satisfied without needing Rule 1/2/3 auto-fixes.

## Issues Encountered

The first `cargo test -p pwmgr-web` run was kicked off in the background before the four Task 2 tests had been appended to `http_integration.rs` (a race between editing the test file and an already-running background compile), so that run only reported 10 of the eventual 14 tests. Caught before committing by noticing the test count didn't match the file's actual content, and resolved by re-running `cargo test -p pwmgr-web --test http_integration` (and later `cargo test --workspace`) after all edits landed — both reran clean with all 14 web integration tests and the full 329+2 pwmgr-core/end_to_end tests passing. No code was affected; this was purely a verification-sequencing issue, not a deviation from the plan's implementation.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- SEC-01's export half is closed: `web-src-tauri/src/handlers.rs::export_csv` accepts no destination and writes no server-side file, proven by `export_csv_writes_no_server_side_file_for_a_crafted_destination`.
- `httpDownload` in `src/lib/api.ts` is now available as the established pattern for any other web-mode file-download flow.
- Plans 09-02 (CSV import upload/preview redesign) and 09-03 (backup restore upload redesign) can proceed independently — this plan's `depends_on: []` and no file overlap with either.
- One item remains genuinely manual: actually clicking Export in a running web-mode session to observe the browser's native download UX end-to-end (coverage `D5`) — not exercised by the automated suites and flagged for human verification at phase UAT.

---
*Phase: 09-web-mode-security-hardening*
*Completed: 2026-09-19*

## Self-Check: PASSED

- FOUND: `.planning/phases/09-web-mode-security-hardening/09-01-SUMMARY.md`
- FOUND: commit `9314271` (Task 1)
- FOUND: commit `a78c81b` (Task 2)
- FOUND: `pub fn do_export_csv_bytes` in `core/src/commands.rs`
- FOUND: `do_export_csv_download` referenced in `web-src-tauri/src/handlers.rs`
- FOUND: `httpDownload` in `src/lib/api.ts`
