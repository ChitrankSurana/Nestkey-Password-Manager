---
phase: 09-web-mode-security-hardening
plan: 02
subsystem: api
tags: [axum, tauri, rust, typescript, csv-import, upload-cache, session-store, sec-01]

# Dependency graph
requires:
  - "httpDownload/isTauri-branched transport dispatch pattern established in 09-01 (src/lib/api.ts)"
provides:
  - "UploadCache enum + per-session upload cache (upload_cache field, set_upload/csv_upload/backup_upload/clear_upload) on UserSession (web-src-tauri/src/session_store.rs) — the D-04 cache 09-03's backup-restore upload can reuse for its own Backup variant"
  - "Path-free do_read_csv_headers_from_str / do_import_csv_preview_from_str core parse functions (core/src/commands.rs)"
  - "httpUpload web transport helper + api.uploadCsv/clearUpload (src/lib/api.ts) for 09-03's upload_backup to model"
affects: [09-03]

# Actuals (#2632)
actuals:
  tokens: 9610
  tasks: 3
  commits: 3

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Per-session single-slot upload cache (UploadCache enum on UserSession, Zeroizing<Vec<u8>> payload) replacing wholesale under one mutex critical section - no append API, so concurrent uploads never splice"
    - "Path-free byte-producer core function + thin path-reading wrapper (do_read_csv_headers_from_str/do_import_csv_preview_from_str -> do_read_csv_headers/do_import_csv_preview), mirroring 09-01's do_export_csv_bytes pattern"
    - "Raw-bytes upload route (axum::body::Bytes extractor) with a route-scoped DefaultBodyLimit layer, applied only to the upload route, never globally"

key-files:
  created: []
  modified:
    - web-src-tauri/src/session_store.rs
    - web-src-tauri/src/middleware.rs
    - core/src/commands.rs
    - web-src-tauri/src/handlers.rs
    - web-src-tauri/tests/http_integration.rs
    - src/lib/api.ts
    - src/screens/CsvImport.tsx

key-decisions:
  - "set_upload's single-critical-section replacement guarantee doc comment was written in Task 1 (when the method was first created) rather than deferred to Task 3 as the plan's task split literally proposed - the guarantee is a property of the method's own body, so documenting it at first-write time avoids a same-file two-commit churn on one doc comment with no functional difference in outcome"
  - "handleSourceChange (web mode) clears the server-side upload cache and resets the path label whenever the user switches source with an upload already cached, rather than preserving the cache to allow multi-format reuse from the dropdown - the backend's D-03 reuse capability is still proven directly at the HTTP layer (one_upload_serves_two_previews), but the UI treats a source switch as abandoning the current upload per D-04's cancel-clears-the-cache intent, avoiding a state where a stale filename label is shown with no bytes behind it"

patterns-established:
  - "isTauri-branched file-pick UI (hidden <input type=\"file\"> + useRef, following Backup.tsx::handleCreate's isTauri-branch template) for any future web-mode file-upload screen"

requirements-completed: []

coverage:
  - id: D1
    description: "A web-mode user can choose a CSV file in the browser, see its column headers, preview the parsed records, and commit the import - without the client ever naming a server-side path"
    requirement: "SEC-01"
    verification:
      - kind: integration
        ref: "web-src-tauri/tests/http_integration.rs#upload_csv_then_read_csv_headers_round_trips_over_http"
        status: pass
      - kind: integration
        ref: "web-src-tauri/tests/http_integration.rs#csv_import_uploads_once_then_previews_and_commits"
        status: pass
    human_judgment: false
  - id: D2
    description: "The uploaded CSV bytes are cached on the requesting browser's own UserSession in process memory only (Zeroizing<Vec<u8>>); no temp file is written to disk"
    requirement: "SEC-01"
    verification:
      - kind: unit
        ref: "web-src-tauri/src/session_store.rs::UploadCache (Zeroizing<Vec<u8>> payload, no fs::write anywhere in the upload path)"
        status: pass
    human_judgment: false
  - id: D3
    description: "The file is uploaded once and reused across the headers, preview, and commit steps - the client does not re-send the bytes at each step"
    requirement: "SEC-01"
    verification:
      - kind: integration
        ref: "web-src-tauri/tests/http_integration.rs#one_upload_serves_two_previews"
        status: pass
      - kind: integration
        ref: "web-src-tauri/tests/http_integration.rs#csv_import_uploads_once_then_previews_and_commits"
        status: pass
    human_judgment: false
  - id: D4
    description: "Two concurrent uploads on the same session serialize on the session's own upload lock: a subsequent read always sees exactly one complete buffer, never spliced bytes"
    requirement: "SEC-01"
    verification:
      - kind: integration
        ref: "web-src-tauri/tests/http_integration.rs#two_concurrent_uploads_leave_one_complete_buffer"
        status: pass
    human_judgment: false
  - id: D5
    description: "Two different browser sessions never observe each other's cached upload"
    requirement: "SEC-01"
    verification:
      - kind: integration
        ref: "web-src-tauri/tests/http_integration.rs#uploads_are_isolated_between_two_browser_sessions"
        status: pass
      - kind: integration
        ref: "web-src-tauri/tests/http_integration.rs#a_second_session_never_sees_the_first_sessions_uploaded_csv"
        status: pass
    human_judgment: false
  - id: D6
    description: "read_csv_headers and import_csv_preview with no prior upload return invalid_input, not a server error and not a filesystem read"
    requirement: "SEC-01"
    verification:
      - kind: integration
        ref: "web-src-tauri/tests/http_integration.rs#read_csv_headers_without_an_upload_is_rejected"
        status: pass
      - kind: integration
        ref: "web-src-tauri/tests/http_integration.rs#import_csv_preview_without_an_upload_is_rejected"
        status: pass
    human_judgment: false
  - id: D7
    description: "The cached upload is cleared on import_csv_commit success, on an explicit clear_upload call, and on logout"
    requirement: "SEC-01"
    verification:
      - kind: integration
        ref: "web-src-tauri/tests/http_integration.rs#upload_cache_is_cleared_after_a_successful_commit"
        status: pass
      - kind: integration
        ref: "web-src-tauri/tests/http_integration.rs#clear_upload_route_drops_the_cached_bytes"
        status: pass
      - kind: unit
        ref: "web-src-tauri/src/handlers.rs::logout calls session.clear_upload() before ctx.store.remove"
        status: pass
    human_judgment: false
  - id: D8
    description: "An upload larger than the 25 MiB route limit is rejected by the upload route only; JSON command routes keep axum's default body limit"
    requirement: "SEC-01"
    verification:
      - kind: integration
        ref: "web-src-tauri/tests/http_integration.rs#an_oversized_upload_is_rejected"
        status: pass
    human_judgment: false
  - id: D9
    description: "Desktop CSV import is unchanged: src-tauri commands still take a path from the native open dialog"
    requirement: "SEC-01"
    verification:
      - kind: unit
        ref: "cargo test -p pwmgr-core (329 tests, no edits to any pre-existing CSV test)"
        status: pass
    human_judgment: false
  - id: D10
    description: "A web-mode user clicking Choose File in CsvImport.tsx uploads via the hidden file input, sees headers/preview, and commits - with no save/open dialog and no server-side path"
    verification: []
    human_judgment: true
    rationale: "Requires manually driving the CSV import screen in a running web-mode session - not exercised by the automated HTTP/unit suites."

duration: ~55min
completed: 2026-09-19
status: complete
---

# Phase 09 Plan 02: Web-Mode CSV Import Upload Redesign Summary

**Web-mode `read_csv_headers`/`import_csv_preview` now read from a per-session in-memory upload cache instead of a client-supplied path — the browser uploads the CSV once and every downstream step (headers, preview, commit) reuses it, cleared on commit success, explicit cancel, and logout.**

## Performance

- **Duration:** ~55 min
- **Started:** 2026-09-18T19:40:00Z (approx.)
- **Completed:** 2026-09-18T20:01:47Z
- **Tasks:** 3
- **Files modified:** 7

## Accomplishments

- Added `UploadCache` enum (`Csv`/`Backup` variants, `Zeroizing<Vec<u8>>` payload) and a single
  `upload_cache: Mutex<Option<UploadCache>>` slot on `UserSession`, with `set_upload`/`csv_upload`/
  `backup_upload`/`clear_upload` accessors — `set_upload` replaces the slot wholesale under one
  critical section, so concurrent uploads on the same session never splice.
- Added `POST /api/upload_csv` (raw `application/octet-stream` body, `DefaultBodyLimit::max(25 MiB)`
  applied to that route only) that caches the uploaded bytes on the requesting session; rewrote
  `read_csv_headers` and `import_csv_preview` to read from that cache instead of a client-supplied
  path, removing `ReadCsvHeadersRequest` entirely and reducing `ImportCsvPreviewRequest` to `{ format }`.
- Split `do_read_csv_headers`/`do_import_csv_preview` in `core/src/commands.rs` into path-free
  `_from_str` byte-producers plus thin `fs::read_to_string` wrappers, so desktop behavior (still
  path-based, per D-02) is provably unchanged — all 329 pre-existing `pwmgr-core` tests pass with no edits.
- Wired all three D-04 clearing points: `import_csv_commit`'s success path (a failed commit keeps
  the upload so the user can retry), a new `POST /api/clear_upload` route, and `logout` (resolves
  the session and clears its upload before removing it from the store).
- Added `httpUpload`/`api.uploadCsv`/`api.clearUpload` to `src/lib/api.ts` and branched
  `readCsvHeaders`/`importCsvPreview` on `isTauri` since desktop and web argument shapes now diverge.
- Branched `CsvImport.tsx`'s file-pick flow on `isTauri` (hidden `<input type="file">` + `useRef` in
  web mode, following `Backup.tsx::handleCreate`'s template), and wired `clearUpload()` into the Back
  button and into `handleSourceChange` when a web-mode upload is abandoned by switching source.
- Added 12 new HTTP integration tests covering the upload→headers→preview→commit round trip, no-upload
  rejection on both handlers, cross-session isolation (two variants), one-upload-serves-two-previews,
  cache clearing on commit/explicit-clear/logout, unauthenticated-upload rejection, concurrent-upload
  coherence (`tokio::join!`), and oversized-upload rejection.

## Task Commits

Each task was committed atomically:

1. **Task 1: Upload a CSV in the browser and get its column headers back — no path, bytes cached on the session** - `bfb6b32` (feat)
2. **Task 2: Preview and commit the cached upload — the full import flow with one upload (D-03)** - `ca019ad` (feat)
3. **Task 3: Upload-cache lifecycle and isolation — cleared on commit, cancel and logout; never shared or spliced** - `c0ffa79` (feat)

## Files Created/Modified

- `web-src-tauri/src/session_store.rs` - Added `UploadCache` enum and the `upload_cache` field +
  `set_upload`/`csv_upload`/`backup_upload`/`clear_upload` methods on `UserSession`; updated the
  test module's `test_session` helper for the new field.
- `web-src-tauri/src/middleware.rs` - Updated the test module's `store_with_session` helper's
  `UserSession` literal for the new `upload_cache` field.
- `core/src/commands.rs` - Added `do_read_csv_headers_from_str` and `do_import_csv_preview_from_str`
  (path-free); `do_read_csv_headers`/`do_import_csv_preview` reduced to thin path-reading wrappers.
- `web-src-tauri/src/handlers.rs` - Added `UPLOAD_BODY_LIMIT` const, `upload_csv` and `clear_upload`
  handlers; rewrote `read_csv_headers`/`import_csv_preview` to read the session cache; wired
  `clear_upload()` into `import_csv_commit`'s success path and `logout`; registered
  `/api/upload_csv` (with the route-scoped body limit) and `/api/clear_upload`.
- `web-src-tauri/tests/http_integration.rs` - 12 new integration tests (see Accomplishments).
- `src/lib/api.ts` - Added `httpUpload` helper, `api.uploadCsv`, `api.clearUpload`; branched
  `readCsvHeaders`/`importCsvPreview` off the shared `call()` dispatcher.
- `src/screens/CsvImport.tsx` - `isTauri`-branched file pick (hidden file input in web mode),
  `handlePreview`/`handleSourceChange` updated for the null-path web contract, `handleBack` and
  `handleSourceChange` now clear the upload cache when abandoning a web-mode upload.

## Decisions Made

- `set_upload`'s doc comment stating the single-critical-section replacement guarantee was written
  in Task 1, at the method's first creation, rather than added later in Task 3 as the plan's task
  split literally proposed — no functional difference, just avoids re-touching the same doc comment
  in two separate commits.
- Web-mode `handleSourceChange` clears the server-side upload cache (and resets the `path` display
  label) whenever the user switches the format dropdown after already uploading a file, rather than
  preserving the cache for cross-format reuse from the UI. The backend's D-03 reuse capability
  (same upload survives two different-format previews) is still proven directly at the HTTP layer by
  `one_upload_serves_two_previews` — this is a frontend UX choice, not a backend limitation: a
  format switch is treated as abandoning the current import attempt, consistent with D-04's
  cancel-clears-the-cache intent, and avoids showing a stale filename label with no cached bytes
  behind it.

## Deviations from Plan

None — plan executed exactly as written, aside from the two Claude's-discretion notes above (both
documented as key-decisions, neither changes behavior or test coverage from what the plan specified).

## Issues Encountered

None. All three tasks' verification commands (`cargo test -p pwmgr-core`, `cargo test -p pwmgr-web`
/`--test http_integration`, `cargo test --workspace`, `npm run build`, `npm test`) passed on the
first run with no auto-fix cycles needed.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- SEC-01's CSV-import half is closed: `read_csv_headers`/`import_csv_preview` accept no path and
  read only from the session's own upload cache, proven by 12 new integration tests covering the
  happy path, no-upload rejection, cross-session isolation, concurrency coherence, cache clearing,
  and oversized-upload rejection.
- `UploadCache`'s `Backup` variant is already defined and unused — 09-03 (backup-restore upload
  redesign) can extend `UserSession`'s existing cache with `backup_upload`/`set_upload(UploadCache::Backup(...))`
  rather than introducing a second store, per D-04.
- `httpUpload` in `src/lib/api.ts` is now available as the established pattern for 09-03's
  `upload_backup` route.
- SEC-01 requirement is NOT yet marked complete in REQUIREMENTS.md — it is shared with 09-03
  (backup-restore upload redesign), which must also land before the `requirements.mark-complete`
  gate resolves it.
- One item remains genuinely manual: actually driving the CSV import screen (choose file → preview
  → commit) in a running web-mode session to observe the browser file-picker UX end-to-end
  (coverage `D10`) — not exercised by the automated suites and flagged for human verification at
  phase UAT.

---
*Phase: 09-web-mode-security-hardening*
*Completed: 2026-09-19*

## Self-Check: PASSED

- FOUND: `.planning/phases/09-web-mode-security-hardening/09-02-SUMMARY.md`
- FOUND: commit `bfb6b32` (Task 1)
- FOUND: commit `ca019ad` (Task 2)
- FOUND: commit `c0ffa79` (Task 3)
- FOUND: `pub enum UploadCache` in `web-src-tauri/src/session_store.rs`
- FOUND: `pub fn do_read_csv_headers_from_str` in `core/src/commands.rs`
- FOUND: `pub fn do_import_csv_preview_from_str` in `core/src/commands.rs`
- FOUND: `httpUpload` in `src/lib/api.ts`
- FOUND: `clearUpload` referenced in `src/screens/CsvImport.tsx`
