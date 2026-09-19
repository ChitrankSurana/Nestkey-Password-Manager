# Phase 09: Web Mode Security Hardening - Research

**Researched:** 2026-09-18
**Domain:** axum/Rust web-server hardening (file-transfer redesign, login throttling, KDF concurrency
capping, CSP/security headers) for a localhost-only password manager
**Confidence:** MEDIUM — architecture and code patterns are grounded in this repo's own source and
official docs; a handful of discretionary numeric parameters are proposed (not user-confirmed), and
one CONTEXT.md premise (D-17's "existing dev/prod signal") did not verify against the actual codebase.

## Summary

Phase 9 closes four findings from `SECURITY_CONCERNS.md` (H1, M1, M2, M3) entirely inside
`web-src-tauri` (plus one line of `src-tauri/tauri.conf.json`). Three independent hardening tracks
are required and can be built largely in parallel: (1) a full upload/download redesign for
`export_csv`/`read_csv_headers`/`import_csv_preview`/`restore_backup` so no server-side path is ever
named by a web client (D-01 supersedes the ROADMAP's path-confinement wording — see the flagged
mismatch below); (2) a new in-process login/register throttle store plus a bounded `tokio::sync::
Semaphore` around the three Argon2-driving handlers; (3) a CSP + security-header middleware layer
applied to every response, mirrored into `tauri.conf.json`.

None of the three tracks requires a new *required* dependency — `tokio::sync::Semaphore`,
`axum::extract::Bytes` (raw-body upload), `std::collections::HashMap` + `RwLock` (throttle store, same
shape as the existing `SessionStore`), and a hand-written `axum::middleware::from_fn` header layer
(same shape as the existing `csrf_middleware`) are all already available in this workspace's current
dependency tree. The only *optional* new dependency is `multer` (transitively, via axum's
`"multipart"` feature) if the planner chooses `axum::extract::Multipart` over the simpler raw-bytes
upload — both patterns are documented below; **raw-bytes upload is the recommendation** because it
needs zero Cargo.toml changes and matches this codebase's stated "no new dependency" preference
(D-06 makes that preference explicit for the throttle store, and the same ethos fits the upload
redesign).

A verified, load-bearing finding that changes the shape of the frontend work: **all three of
`CsvImport.tsx`, `CsvExport.tsx`, and `Backup.tsx`'s restore flow call `@tauri-apps/plugin-dialog`
(`open()`/`save()`) completely unconditionally today — none of them branch on `isTauri`.** That
plugin only functions inside a real Tauri webview (it calls Tauri IPC internally); in a plain browser
tab (web mode) these calls have no working browser-native equivalent today. This means the web-mode
upload/download UI for these three screens does not currently exist in any form — it is new frontend
work, not a small modification of existing web-mode code paths.

**Primary recommendation:** Redesign the four H1 handlers to move bytes over the HTTP body directly
(raw upload via `axum::extract::Bytes` cached on `UserSession`, raw download via a response with
`Content-Disposition`), add a same-shaped throttle store next to `SessionStore` with the exact D-08
exponential-lockout-window shape, wrap `login`/`register_user`/password-mode `export_csv` in a shared
`tokio::sync::Semaphore` (recommend 3 permits), and add a `security_headers_middleware` layered the
same way `csrf_middleware` already is — then mirror the resulting CSP string into
`tauri.conf.json`'s `app.security.csp`, using Tauri's documented `--config` merge-patch mechanism
(not an existing repo convention — see Open Questions) to keep a relaxed dev-mode CSP for
`tauri dev`.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| File upload/download (export/import/restore) | API / Backend (`web-src-tauri`) | Browser / Client (new `<input type="file">` + Blob-download UI) | H1's fix must live entirely server-side (no path ever trusted from the client); the browser only ever sends/receives bytes, never a path |
| Login/register throttle-lockout | API / Backend (`web-src-tauri`, new store) | — | Attempt/lockout state is request-scoped server state; no client involvement beyond receiving a 429 |
| Argon2 concurrency cap | API / Backend (`web-src-tauri`, shared `Semaphore`) | — | Memory-exhaustion defense is purely a server resource-management concern |
| CSP / security headers | API / Backend (`web-src-tauri` middleware) + Desktop Shell (`tauri.conf.json`) | Browser / Client (CSP is *enforced* by the browser/webview, but *declared* by the server/shell) | Two distinct enforcement points for the same policy string — web mode's HTTP layer and the desktop webview's static config |
| 429 auto-retry (D-14) | Browser / Client (`src/lib/api.ts`) | — | Purely a frontend transport-layer concern; the backend only needs to return 429 with a distinguishable error code |

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| SEC-01 | Web-mode export/import/restore endpoints reject any client-supplied path outside the app's own backups/exports directory — no arbitrary file read or write is possible. | **Superseded by D-01's stronger fix** (see mismatch flag below): full upload/download redesign — see "Upload/Download Redesign" pattern, Code Examples, and the frontend-gap finding above. |
| SEC-02 | Web-mode login/register attempts are throttled (backoff/lockout), and concurrent Argon2 derivations are bounded so a request burst cannot exhaust server memory. | See "Login/Register Throttle Store" and "Argon2 Concurrency Semaphore" patterns, Code Examples, and proposed W/cap/permit-count values. |
| SEC-03 | The web server sends a strict Content-Security-Policy and `X-Content-Type-Options: nosniff` on every response, and the desktop Tauri shell no longer ships with `csp: null`. | See "Security Headers Middleware" pattern, concrete CSP directive proposal, and the Tauri dev/prod CSP split discussion. |

</phase_requirements>

## Wording Mismatch: ROADMAP vs. CONTEXT.md (flagged per orchestrator instruction)

`.planning/ROADMAP.md` Phase 9 success criterion #1 and `REQUIREMENTS.md` SEC-01's locked wording
both describe **path-confinement**: "reject any client-supplied path that isn't a filename inside the
app's own backups/exports directory." **`09-CONTEXT.md` D-01 explicitly overrides this** with a full
upload/download redesign — no server-side path is ever named by a web client for any of the four
handlers, full stop (no "filename-only" allowance either). D-01's rationale (closing the
attacker-controlled-input surface entirely rather than validating it) is authoritative per
CONTEXT.md's own text: *"the full upload/download redesign, not path-confinement."* **The planner and
plan-checker must implement D-01's redesign, not a path-confinement/canonicalization fix, even though
that is what the ROADMAP/REQUIREMENTS wording literally says.** Do not let a plan-checker pass that
diffs the plan against ROADMAP/REQUIREMENTS wording alone silently revert this to path-confinement.

## Standard Stack

### Core (no new dependency required)

| Component | Source | Purpose | Why no new crate |
|-----------|--------|---------|-------------------|
| `tokio::sync::Semaphore` | Already in `tokio = { version = "1", features = ["full"] }` (web-src-tauri Cargo.toml) | Bounded Argon2 concurrency (D-11/D-12/D-13) | `full` includes `sync`; `Semaphore::const_new`/`try_acquire` need no extra feature `[CITED: docs.rs/tokio/latest/tokio/sync/struct.Semaphore.html]` |
| `axum::extract::Bytes` (re-export of `bytes::Bytes`) | axum default features | Raw-body file upload (recommended over Multipart) | Built-in extractor, no feature flag; 2 MiB default limit raised via `DefaultBodyLimit` `[CITED: docs.rs — RUSTSEC-2022-0055 / DefaultBodyLimit docs]` — **verify exact re-export path (`axum::body::Bytes` vs adding `bytes = "1"` as a direct dependency to name `bytes::Bytes`) against the installed axum 0.7.9 docs at implementation time; my doc fetch for this specific re-export 404'd, so treat the import line as unverified until checked** |
| `std::collections::HashMap` + `std::sync::RwLock` | stdlib | Throttle store (D-06), same shape as `SessionStore` | Matches existing pattern exactly; D-06 explicitly rules out a new dependency |
| `axum::middleware::from_fn` | Already used (`csrf_middleware`, `auth_middleware`) | New `security_headers_middleware` layer (D-18) | Same layering convention already established in `web-src-tauri/src/middleware.rs` |

### Supporting (optional — only if Multipart chosen over raw-bytes)

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `multer` (transitive via `axum = { features = ["multipart"] }`) | `^3.0.0` `[CITED: docs.rs/axum Multipart page]` | Multipart form-data parsing | Only if the planner wants a single request to carry both the file AND extra form fields together — this phase's upload flows don't need that (format/confirmed are separate follow-up calls per D-03) |

**Version verification:** `multer` checked via the package-legitimacy seam — see audit below. No
other new crate needs registry verification; every other component listed is already a dependency of
this workspace (`tokio`, `axum`, `bytes` transitively) at the versions already pinned in `Cargo.lock`
(`axum 0.7.9`, confirmed via `Cargo.lock` `[VERIFIED: Cargo.lock:298]`).

**Installation (only if Multipart path chosen):**
```toml
# web-src-tauri/Cargo.toml
axum = { version = "0.7", features = ["multipart"] }
# dev-dependencies, only needed if integration tests build multipart bodies:
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json", "cookies", "multipart"] }
```

## Package Legitimacy Audit

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|--------------|---------|-------------|
| `multer` | crates.io | published 2020-05-10 (~6 yrs) | ~1.74M/week | github.com/rwf2/multer | **OK** | Approved (only needed if Multipart path chosen) `[VERIFIED: gsd_run query package-legitimacy check --ecosystem crates multer]` |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none
**Note:** no other new package is proposed by this research; the primary recommendation (raw-bytes
upload) needs zero new dependencies.

## Architecture Patterns

### System Architecture Diagram — Upload/Download Redesign (D-01/D-03/D-04/D-05)

```text
BROWSER (web mode only — desktop keeps native dialogs, D-02)
  <input type="file"> (CSV import / backup restore)
       │  file.arrayBuffer() / raw Blob body
       ▼
  fetch POST /api/upload_csv | /api/upload_backup
       │  headers: X-PWMGR-CSRF, Content-Type: application/octet-stream
       │  body: raw file bytes  (NO path field anywhere in the request)
       ▼
┌───────────────────────────────────────────────────────────────────┐
│ axum: csrf_middleware → auth_middleware → upload handler          │
│   extracts Extension<Arc<UserSession>> + Bytes body                 │
│   session.upload_cache.lock().unwrap() = Some(UploadCache::Csv(b))  │  ← D-04: lives on
└───────────────────────────────────────────────────────────────────┘     UserSession, not
       │  200 OK, no path returned                                        a new global store
       ▼
  frontend calls /api/read_csv_headers (bodyless POST)
       │
       ▼
┌───────────────────────────────────────────────────────────────────┐
│ handler reads session.upload_cache (no path param at all)          │
│   pwmgr_core::commands::do_read_csv_headers_from_bytes(&bytes)     │
└───────────────────────────────────────────────────────────────────┘
       │  headers[] JSON
       ▼
  ... import_csv_preview(format) → import_csv_commit(records) (unchanged) ...
       │  on commit success OR user cancel OR logout:
       ▼
  session.upload_cache cleared explicitly (D-04)

DOWNLOAD (export_csv):
  fetch POST /api/export_csv  { mode, confirmed }
       ▼
┌───────────────────────────────────────────────────────────────────┐
│ handler calls do_export_csv_bytes(...) -> Vec<u8> (no path write)  │
│ Response: Content-Disposition: attachment; filename="..."          │
│           body = the CSV/ciphertext bytes directly                 │
└───────────────────────────────────────────────────────────────────┘
       ▼
  frontend: res.blob() → URL.createObjectURL → programmatic <a download> click
```

### Recommended Project Structure (additions only)

```
web-src-tauri/src/
├── handlers.rs          # existing — export/import/restore handlers rewritten per below
├── middleware.rs         # existing — add security_headers_middleware here (same file, same
│                          #   convention as csrf_middleware/auth_middleware)
├── session_store.rs      # existing — UserSession gets a new `upload_cache: Mutex<Option<UploadCache>>`
│                          #   field; add the UploadCache enum here (co-located with the struct it hangs off)
├── throttle_store.rs      # NEW — ThrottleStore, same Arc<RwLock<HashMap<...>>> shape as SessionStore,
│                          #   its own sweep-compatible struct; D-06 says "alongside session_store.rs"
└── kdf_guard.rs OR handlers.rs  # NEW (or inline) — the shared Semaphore + a small wrapper fn;
                            #   D-12 leaves "same module or separate" to Claude's discretion —
                            #   recommend a tiny dedicated module (kdf_guard.rs) since it's reused by
                            #   3 handlers and unrelated to session/throttle concerns
core/src/
├── commands.rs            # do_export_csv/do_read_csv_headers/do_import_csv_preview/do_restore_backup
│                          #   each split into a `_bytes`-suffixed core fn (web calls this) plus the
│                          #   existing path-based fn becomes a thin wrapper (desktop still calls this)
└── error.rs                # add throttled_error()/rate_limited_error() constructors (see pitfalls)
web-src-tauri/src/error_map.rs  # add "throttled" and "rate_limited" arms to status_for() -> 429
src/screens/
├── CsvImport.tsx           # add isTauri branch: web branch uses <input type="file"> + upload_csv
├── CsvExport.tsx           # add isTauri branch: web branch triggers Blob-download instead of save()
└── Backup.tsx               # add isTauri branch to handleRestore (handleCreate already has one)
src/lib/
└── api.ts                  # httpPost gets 429-aware retry (D-14); new raw-body upload helper
```

### Pattern 1: Raw-bytes upload + session-cached bytes (D-01/D-03/D-04/D-05)

**What:** A new pre-existing-shaped pair of endpoints (`/api/upload_csv`, `/api/upload_backup`) that
accept the raw file body via axum's built-in `Bytes` extractor, and cache it on the requesting
session. Every downstream step (`read_csv_headers`, `import_csv_preview`, `restore_backup`) drops its
`path: String` field entirely and reads from the cached bytes instead.

**When to use:** Any of the four H1 handlers, and any future web-mode feature that would otherwise
want a server-side path from the client.

**Example (server, illustrative — exact types depend on final `UploadCache` shape):**
```rust
// web-src-tauri/src/session_store.rs — new field + enum
pub enum UploadCache {
    Csv(Vec<u8>),
    Backup(Vec<u8>),
}

pub struct UserSession {
    pub username: String,
    pub app_state: AppState,
    pub last_seen: Mutex<Instant>,
    pub upload_cache: Mutex<Option<UploadCache>>,   // NEW — D-04
}
```
```rust
// web-src-tauri/src/handlers.rs
use axum::extract::DefaultBodyLimit;

async fn upload_csv(
    Extension(s): Extension<Arc<UserSession>>,
    body: axum::body::Bytes,   // verify exact import path at implementation time — see Standard Stack note
) -> StatusCode {
    *s.upload_cache.lock().unwrap() = Some(UploadCache::Csv(body.to_vec()));
    StatusCode::OK
}

async fn read_csv_headers(
    Extension(s): Extension<Arc<UserSession>>,
) -> Result<Json<Vec<String>>, ApiError> {
    let guard = s.upload_cache.lock().unwrap();
    let Some(UploadCache::Csv(bytes)) = guard.as_ref() else {
        return Err(pwmgr_core::error::invalid_input_error("No CSV file uploaded yet.").into());
    };
    Ok(Json(do_read_csv_headers_from_bytes(bytes)?))
}

// Router — raise the 2MiB default so real vault CSVs/backups aren't silently rejected:
.route("/api/upload_csv", post(upload_csv))
.route("/api/upload_backup", post(upload_backup))
.layer(DefaultBodyLimit::max(25 * 1024 * 1024))  // 25 MiB — generous for any realistic vault export
```

**Example (frontend, illustrative):**
```typescript
// src/lib/api.ts — new helper, web-mode only
async function httpUpload(command: string, file: File): Promise<void> {
  const res = await fetch(`/api/${command}`, {
    method: "POST",
    headers: { "Content-Type": "application/octet-stream", "X-PWMGR-CSRF": "1" },
    credentials: "same-origin",
    body: file, // File implements Blob — fetch accepts it directly, no manual arrayBuffer() needed
  });
  if (!res.ok) { /* same error handling as httpPost */ }
}
```

### Pattern 2: Blob download for export (D-01)

**What:** `export_csv` returns the bytes directly in the HTTP response body with
`Content-Disposition: attachment`, instead of writing to a server path and returning 200/void.

**Example (server):**
```rust
async fn export_csv(
    Extension(s): Extension<Arc<UserSession>>,
    Json(b): Json<ExportCsvRequest>,   // now just { mode, confirmed } — no `path` field
) -> Result<Response, ApiError> {
    let bytes = do_export_csv_bytes(&s.app_state, &b.mode, b.confirmed)?;
    let filename = if matches!(b.mode, ExportMode::Plaintext) { "vault-export.csv" } else { "vault-export.pwenc" };
    Ok((
        [
            (header::CONTENT_TYPE, "application/octet-stream"),
            (header::CONTENT_DISPOSITION, &format!("attachment; filename=\"{filename}\"")),
        ],
        bytes,
    ).into_response())
}
```

**Example (frontend):**
```typescript
async function downloadExport(mode: ExportMode, confirmed: boolean) {
  const res = await fetch("/api/export_csv", {
    method: "POST",
    headers: { "Content-Type": "application/json", "X-PWMGR-CSRF": "1" },
    credentials: "same-origin",
    body: JSON.stringify({ mode, confirmed }),
  });
  if (!res.ok) { /* parse CommandError as in httpPost */ }
  const blob = await res.blob();
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = mode.mode === "plaintext" ? "vault-export.csv" : "vault-export.pwenc";
  a.click();
  URL.revokeObjectURL(url);
}
```

### Pattern 3: Login/register throttle store (D-06 through D-10)

**What:** A `ThrottleStore` shaped exactly like `SessionStore` (`Arc<RwLock<HashMap<...>>>`, own
sweep function called from the same idle-sweep loop in `main.rs`), tracking **both** a
`username`-keyed and an `ip`-keyed map independently (D-09).

**The exact D-08 shape (user-specified, do not substitute a different curve):**
- 5 failed attempts for a key → lock that key out for base window `W`.
- If, after that lockout expires, the same key fails 5 more times → the *next* lockout window doubles
  (`W → 2W → 4W → ...`). This is a per-lockout-cycle doubling of the **window itself**, not a
  per-attempt delay curve.
- D-10: both the fail counter and the window multiplier reset to base `W` on the **next successful
  login** for that key — not on a timer.

**Proposed concrete values (Claude's Discretion, D-08's "not discussed" gap — not user-confirmed,
recommend a checkpoint or explicit confirmation before locking in):**
- Base window `W = 30 seconds`. Rationale: SECURITY_CONCERNS.md M1 is about *online* brute force on a
  local single-user server, not a distributed attacker — 30s meaningfully slows automated guessing
  while not punishing a legitimate user who mistypes 5 times in a row.
- Doubling cap: cap the **window value** (not the cycle count) at **1 hour**, so the sequence is
  `30s, 1m, 2m, 4m, 8m, 16m, 32m, 60m (capped), 60m, ...`. Rationale: bounds the worst case for a
  legitimate user who keeps forgetting their password to "never locked out more than an hour," while
  still imposing serious friction on a sustained attacker.

**Example (illustrative sketch):**
```rust
// web-src-tauri/src/throttle_store.rs
pub struct ThrottleEntry {
    fail_count: u32,
    current_window: Duration,      // starts at BASE_WINDOW, doubles per lockout cycle, capped
    locked_until: Option<Instant>,
}

const BASE_WINDOW: Duration = Duration::from_secs(30);
const MAX_WINDOW: Duration = Duration::from_secs(60 * 60);
const FAIL_THRESHOLD: u32 = 5;

impl ThrottleStore {
    /// Returns Err(locked_until) if this key is currently locked out.
    pub fn check(&self, key: &str) -> Result<(), Instant> { /* ... */ }
    pub fn record_failure(&self, key: &str, clock: &dyn Clock) { /* increments; locks + doubles at threshold */ }
    pub fn record_success(&self, key: &str) { /* resets fail_count=0, current_window=BASE_WINDOW, locked_until=None */ }
}
```
The `login`/`register` handlers check **both** `throttle.check(&format!("user:{username}"))` and
`throttle.check(&format!("ip:{ip}"))` before calling `do_login`/`do_register` — whichever is locked
blocks the request (D-09). Extracting the client IP requires axum's `ConnectInfo<SocketAddr>`
extractor (built into axum core, no new dependency) on the `TcpListener`-bound router.

### Pattern 4: Argon2 concurrency semaphore (D-11/D-12/D-13)

**What:** One shared `tokio::sync::Semaphore` on `AppContext`, `try_acquire`'d (never queued —
D-11's DoS-resistance rationale) around all three Argon2-driving handlers: `login`, `register_user`,
and password-mode `export_csv`.

**Proposed permit count (Claude's Discretion, D-12's 2–4 range): `3`.** Rationale: matches
`SECURITY_CONCERNS.md` M2's own suggested fix verbatim — *"a bounded semaphore around KDF-bearing
handlers so concurrent derivations can't exhaust RAM... (e.g., max 3 in-flight)"*
`[VERIFIED: SECURITY_CONCERNS.md:70-71 — "same throttling as M1, plus a bounded semaphore around KDF-bearing handlers so concurrent derivations can't exhaust RAM." / core/src/crypto.rs:48 — "Params::new(65536, 3, 4, Some(32))"]`.
Worst-case memory with 3 permits: `3 × 64 MiB = 192 MiB`, confirmed against the actual Argon2 params
in `core/src/crypto.rs:48` (`m=65536` KiB = 64 MiB).

**Example:**
```rust
// AppContext gets a new field:
pub struct AppContext {
    pub store: SessionStore,
    pub paths: Arc<AppPaths>,
    pub kdf_semaphore: Arc<tokio::sync::Semaphore>,   // Semaphore::new(3)
}

async fn login(State(ctx): State<AppContext>, Json(body): Json<AuthRequest>) -> Result<Response, ApiError> {
    let _permit = ctx.kdf_semaphore.try_acquire()
        .map_err(|_| ApiError::from(pwmgr_core::error::rate_limited_error(
            "Server is busy processing other login attempts. Try again in a moment.")))?;
    issue_session(&ctx, &body, false)   // permit held for the whole derive_key call, dropped on return
}
```
**Ordering matters:** the throttle-lockout check (Pattern 3) should run **before** acquiring the
semaphore permit, so an already-locked-out key never competes for a KDF permit at all — that reserves
permits for keys that are still within their legitimate attempt budget.

### Pattern 5: Security headers middleware (D-15 through D-18)

**What:** A new `security_headers_middleware` in `web-src-tauri/src/middleware.rs`, layered the same
way `csrf_middleware` already is (`axum::middleware::from_fn`), applied to **every** response
including the static `ServeDir`/`ServeFile` fallback.

**Locked CSP core (D-15, verbatim):**
```
default-src 'self'; script-src 'self'; style-src 'self'
```
Confirmed non-blocking for the repo's one inline-style call site — verified by reading the file
directly: `src/components/PasswordGeneratorPanel.tsx:186` —
`` style={{ width: `${(meter.score / 4) * 100}%` }} `` `[VERIFIED: src/components/PasswordGeneratorPanel.tsx:186]`.
This is a React DOM-property assignment, not parsed inline CSS text, so it does not require
`'unsafe-inline'` in `style-src`. No other inline `style=` usage was found in this session's scan of
the affected screens (`CsvImport.tsx`, `CsvExport.tsx`, `Backup.tsx`) — if the planner finds another
during implementation, re-verify this reasoning against that specific call site before assuming it
also doesn't need an exception.

**Proposed additional directives (Claude's Discretion, D-15's "beyond default-src/script-src/style-src"
gap):**
```
img-src 'self' data:; connect-src 'self'; font-src 'self'; object-src 'none';
base-uri 'self'; form-action 'self'; frame-ancestors 'none'
```
Rationale, each verified against actual repo content this session:
- `img-src 'self' data:` — no external image hosts found (`index.html` only references
  `/vite.svg`); `data:` allowed for any inline SVG/icon data URIs.
- `connect-src 'self'` — all API calls are same-origin `fetch("/api/...")` (`src/lib/api.ts:144`);
  the HIBP breach check runs server-side via `reqwest::blocking` in `core/src/commands.rs`, not
  client-side, so no external `connect-src` exception is needed.
- `font-src 'self'` — `bootstrap-icons` is bundled locally via `@import
  "bootstrap-icons/font/bootstrap-icons.css"` (`src/App.css:4`) and an npm dependency
  (`package.json:17`), not a CDN link — Vite bundles the font files as same-origin assets
  `[VERIFIED: src/App.css:4 — "@import "bootstrap-icons/font/bootstrap-icons.css";"]`.
- `object-src 'none'`, `base-uri 'self'`, `form-action 'self'`, `frame-ancestors 'none'` — standard
  hardening defaults with no known conflicting usage in this codebase; `frame-ancestors 'none'` is the
  CSP-native complement to D-18's `X-Frame-Options: DENY`.

**D-18's additional headers (verbatim requirement — cost nothing once the layer exists):**
```
X-Content-Type-Options: nosniff
Referrer-Policy: no-referrer   (or strict-origin-when-cross-origin — either is defensible on localhost;
                                  no-referrer is simplest since there's no legitimate cross-origin need)
X-Frame-Options: DENY
```

**Web-mode header layer needs NO dev/prod split.** `pwmgr-web` (the axum binary) only ever serves
`./dist` (the built bundle) — it has no "dev mode" of its own (confirmed: `web-src-tauri/src/main.rs`
has no Vite-proxying logic, only `ServeDir::new("dist")`). The strict D-15 policy can be sent
unconditionally by this middleware.

**`tauri.conf.json`'s CSP DOES need a dev/prod split, and — important gap — the codebase does NOT
already have the signal D-17 assumes exists.** See Open Questions below.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|--------------|-----|
| Rate limiting / leaky-bucket algorithm | A generic token-bucket/leaky-bucket rate limiter, or a crate like `governor`/`tower_governor` | The hand-rolled `ThrottleStore` (Pattern 3) | **Locked by D-06/D-08**: the exact exponential-lockout-window shape is user-specified and doesn't map onto a generic rate-limiter crate's API; D-06 also explicitly rules out a new dependency |
| Argon2/KDF implementation | A different KDF library or hand-rolled memory-hard function | `argon2` crate (already a dependency, `core/src/crypto.rs`) — only the *concurrency bound* is new | This phase caps concurrency, it does not touch the KDF itself; SECURITY_CONCERNS.md's "Notes on things that are done well" already calls the crypto core well-built |
| CSP nonce/hash generation for inline scripts | A nonce-generation/CSP-builder crate | A static policy string (D-15 has no inline scripts to accommodate) | The app has zero inline `<script>` tags (Vite bundles everything as external `<script type="module" src="...">`); a static `script-src 'self'` needs no nonce machinery |
| Multipart form parsing (if that path is chosen) | A hand-rolled multipart boundary parser | `axum::extract::Multipart` (wraps `multer`) | Multipart parsing has real security history (boundary-injection, unbounded part counts) — never hand-roll it |

**Key insight:** every piece of new logic in this phase (throttle windows, semaphore permits, CSP
headers) is either explicitly locked by CONTEXT.md's own precise specification (D-08) or trivially
achievable with stdlib/already-present dependencies — there is no legitimate case for a new crate in
the required path.

## Common Pitfalls

### Pitfall 1: `status_for()` silently 500s a new error code instead of returning 429
**What goes wrong:** `web-src-tauri/src/error_map.rs::status_for` is an exhaustive match with a
catch-all `_ => StatusCode::INTERNAL_SERVER_ERROR` `[VERIFIED: web-src-tauri/src/error_map.rs:19-29 — "match code { \"invalid_credentials\" | \"locked\" => StatusCode::UNAUTHORIZED, ... _ => StatusCode::INTERNAL_SERVER_ERROR, }"]`.
Any new `CommandError` code introduced for throttle-lockout or semaphore-saturation (e.g.
`"throttled"`, `"rate_limited"`) will silently map to 500 unless a new match arm is added.
**Why it happens:** the catch-all is deliberate ("so a future code can never silently masquerade as
success") but that same design means a *missing* arm fails loud in the wrong direction — 500 instead
of 429 — not a compile error.
**How to avoid:** add explicit arms for both new codes, e.g. `"throttled" | "rate_limited" =>
StatusCode::TOO_MANY_REQUESTS`.
**Warning signs:** integration test expecting 429 gets 500 instead.

### Pitfall 2: One 429 code conflated across two different retry semantics
**What goes wrong:** D-14 wants the frontend to auto-retry (1-2s, invisible) on 429. But this phase
produces 429s from **two different causes** with very different correct client behavior: the KDF
semaphore (Pattern 4) is a brief, transient contention — safe and expected to auto-retry — while
throttle lockout (Pattern 3) can be locked for up to an hour (per the proposed cap) — auto-retrying
that after 1-2s just re-triggers the same lockout and burns another network round-trip for nothing,
and hides a message the user actually needs to see ("try again in N minutes").
**Why it happens:** both conditions naturally map to HTTP 429, so it's tempting to give them one
error code.
**How to avoid:** use two distinct `CommandError` codes (`"rate_limited"` for the semaphore,
`"throttled"` for lockout), both mapped to 429 in `status_for()`, and have `api.ts`'s retry logic
(D-14) branch on the code — auto-retry only `"rate_limited"`, surface `"throttled"` to the user
immediately with its message.
**Warning signs:** UX complaint of "the app just spins silently for a while during a lockout."

### Pitfall 3: The frontend screens don't have a web-mode code path to modify — they need one added
**What goes wrong:** Planning this as "swap the `path` field for a file-body upload" on the frontend
undercounts the work. `CsvImport.tsx` (`handlePickFile`), `CsvExport.tsx` (`handleExport`), and
`Backup.tsx` (`handleRestore`) all call `@tauri-apps/plugin-dialog`'s `open()`/`save()` **completely
unconditionally** today, with no `isTauri` branch at all
`[VERIFIED: src/screens/CsvImport.tsx:32-34, src/screens/CsvExport.tsx:22-24, src/screens/Backup.tsx:36-38]`.
(`Backup.tsx`'s `handleCreate`, by contrast, *does* already branch on `isTauri` — `Backup.tsx:15`.)
**Why it happens:** these three flows were presumably built desktop-first and never revisited for web
mode, since (per H1) the whole path-based contract was already broken for web-mode security anyway.
**How to avoid:** treat this as new web-mode UI, not a modification — add an `isTauri` branch to each
of the three, matching the pattern `Backup.tsx`'s `handleCreate` already establishes.
**Warning signs:** a plan that only touches `web-src-tauri/src/handlers.rs` and `core/src/commands.rs`
and assumes the frontend "just needs the path param removed" will ship broken web-mode file features.

### Pitfall 4: `axum::extract::Bytes`'/`Multipart`'s 2 MiB default silently rejects real files
**What goes wrong:** both the raw-`Bytes` extractor and `Multipart` cap request bodies at 2 MiB by
default (a fix for RUSTSEC-2022-0055) `[CITED: rustsec.org/advisories/RUSTSEC-2022-0055.html]`. A
vault backup or CSV export for a large vault could plausibly exceed that.
**Why it happens:** the limit is invisible until a real-world upload/download exceeds it — small test
vaults during development won't trip it.
**How to avoid:** apply `DefaultBodyLimit::max(N)` (e.g. 25 MiB) as a layer on the upload routes
specifically (not the whole router, to avoid loosening the limit on JSON command routes too).
**Warning signs:** "413 Payload Too Large" (or a generic error) on import/restore only for larger
files, working fine in dev with a tiny test vault.

### Pitfall 5: Desktop's existing `create_backup` for web mode already ships an undownloadable file — don't conflate with this phase's scope
**What goes wrong:** `web-src-tauri/src/handlers.rs::create_backup` (line ~577) already writes the
backup into `paths.backups_dir()` server-side (with `destination_path: None`, deliberately, per its
own comment) and returns the **server-side path string** to the browser
`[VERIFIED: web-src-tauri/src/handlers.rs:577-582]`. The browser has no way to fetch that file's bytes
— `Backup.tsx`'s success message ("Backup written to `<path>`") is misleading in web mode today. This
predates Phase 9 and is not one of H1's four named handlers (H1 lists `restore_backup`, `export_csv`,
`read_csv_headers`, `import_csv_preview` only) — `create_backup` never accepted an attacker-controlled
path, so it isn't an H1 vulnerability, just a pre-existing usability gap.
**Why it happens:** Phase 8 Plan 10 already closed a gate (`G-08-1`) that deliberately kept web-mode
`create_backup` passing `None` for the destination — this was an intentional decision, not an oversight.
**How to avoid:** don't silently fold "make create_backup downloadable" into this phase's scope; it's
adjacent but not named by D-01 or H1. Flag it (see Open Questions) rather than expanding scope
unilaterally.

### Pitfall 6: Mutex-poisoning risk (L1) applies to the new throttle store too if built the same way
**What goes wrong:** `SECURITY_CONCERNS.md` L1 already flags pervasive `.lock().unwrap()` as a
poisoning risk. A new `ThrottleStore` built with the same `Mutex`/`RwLock` + `.unwrap()` idiom
inherits the same risk — a panic while holding the throttle map's write lock (e.g. during
`record_failure`) would poison it and brick login/register for every user until restart.
**Why it happens:** copying `SessionStore`'s exact shape (as D-06 directs) copies its exact risk
profile too.
**How to avoid:** this phase doesn't need to fix L1 project-wide, but keep the new throttle-store code
path free of any `.unwrap()`/`.expect()`/`panic!` on the request path (arithmetic on `Duration`
doubling, in particular, must not overflow-panic — use `saturating_mul`/checked arithmetic capped at
`MAX_WINDOW`).
**Warning signs:** a crafted rapid-fail sequence causing a `Duration` overflow panic.

## Code Examples

See the five patterns above (Architecture Patterns section) for the load-bearing, verified-against-
this-repo code sketches: raw-bytes upload + session cache, Blob download, throttle store shape,
Argon2 semaphore, and security headers middleware. All are illustrative sketches to compose during
planning, not drop-in final code.

## State of the Art

| Old Approach (current repo state) | New Approach (this phase) | When Changed | Impact |
|--------------------------------|---------------------------|---------------|--------|
| `export_csv`/`import_csv_preview`/`read_csv_headers`/`restore_backup` take `path: String` from an untrusted HTTP client, pass straight to `std::fs::read`/`write` | Client never sends a path; server streams bytes over the HTTP body directly | This phase (SEC-01, D-01) | Eliminates H1 entirely — there is no path validation to get subtly wrong, because there is no path |
| No throttle/lockout on `/api/login` or `/api/register_user`; `/api/register_user` fully open pre-auth | Per-username AND per-IP exponential lockout (D-08/D-09/D-10) on both routes | This phase (SEC-02, D-06/D-07) | Closes M1 |
| Every login/register/password-export runs an unbounded-concurrency 64 MiB Argon2 derivation | Shared `Semaphore(3)`, `try_acquire` + immediate 429 on saturation | This phase (SEC-02, D-11/D-12/D-13) | Caps worst-case memory at ~192 MiB regardless of burst size, closes M2 |
| `web-src-tauri` sends no CSP/`X-Content-Type-Options`/`Referrer-Policy`/`X-Frame-Options`; `tauri.conf.json` has `"csp": null` | Strict CSP + all four headers on every response; real CSP in `tauri.conf.json` | This phase (SEC-03, D-15–D-18) | Closes M3 |

**Deprecated/outdated:** the four handlers' current `path: String` request DTOs
(`RestoreBackupRequest.path`, `ExportCsvRequest.path`, `ReadCsvHeadersRequest.path`,
`ImportCsvPreviewRequest.path` — `web-src-tauri/src/handlers.rs:198-220`) are removed entirely for web
mode, not deprecated-but-kept; desktop's Tauri command layer (`src-tauri/src/commands.rs`) keeps its
own separate `Path`-taking signatures per D-02 and is untouched by this phase.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|----------------|
| A1 | Base lockout window `W = 30s`, doubling cap at 1 hour | Pattern 3 (throttle store) | Too short: brute force still practical; too long: legitimate forgetful user locked out for up to an hour. CONTEXT.md explicitly delegates this value to research/planning — recommend confirming with the user via a checkpoint before finalizing, since it's a product/UX tradeoff, not a pure engineering fact. |
| A2 | Argon2 semaphore permit count = 3 | Pattern 4 | Grounded directly in SECURITY_CONCERNS.md's own suggested value, so low risk — but still an unconfirmed discretionary choice within the locked 2–4 range. |
| A3 | Referrer-Policy value proposed as `no-referrer` (either `no-referrer` or `strict-origin-when-cross-origin` is defensible; D-18 only names the header, not its value) | Pattern 5 | Low risk — both values are safe on a localhost-only server with no legitimate cross-origin referrer need. |
| A4 | `axum::body::Bytes` is the correct import path for the raw-body extractor (vs. adding `bytes` as a direct dependency and using `bytes::Bytes`) | Pattern 1 / Standard Stack | If wrong, a one-line import fix at implementation time — not architecturally significant, but flagging since my doc lookup for this specific re-export path returned a 404 rather than a confirmation. |
| A5 | D-17's premise that the codebase "already uses" a dev/prod signal to distinguish Vite dev server from built bundle, reusable for gating `tauri.conf.json`'s CSP | Pattern 5 / Open Questions | **This did not verify** — see Open Questions. If the planner assumes an existing mechanism exists and just "gates on it," they will find nothing to gate on and stall mid-implementation. |

## Open Questions

1. **D-17's assumed existing dev/prod signal was not found in this codebase.**
   - What we know: Tauri v2 CSP genuinely does apply during `tauri dev` (not just production builds)
     `[CITED: v2.tauri.app/security/csp — via WebFetch this session]`, so D-15's strict policy really
     would break Vite's HMR websocket + eval-based sourcemaps in dev if applied unconditionally, and a
     split is genuinely necessary, not optional. Tauri v2 officially supports config overlays via the
     `--config` CLI flag (JSON Merge Patch, RFC 7396) — e.g. `tauri dev --config
     src-tauri/tauri.dev.conf.json` `[CITED: v2.tauri.app/develop/configuration-files]`.
   - What's unclear: this repo's `package.json` `"tauri": "tauri"` script and `vite.config.ts` contain
     no existing dev/prod config-split mechanism today (confirmed by reading both files this session)
     — there is no `tauri.dev.conf.json`, no `--config` flag anywhere in `package.json` scripts, and no
     `NODE_ENV`/`TAURI_ENV_DEBUG`-gated Rust code path found in `src-tauri/`.
   - Recommendation: the planner should introduce this mechanism fresh (it doesn't need to "reuse"
     something that doesn't exist) — add `src-tauri/tauri.dev.conf.json` containing just a relaxed CSP
     override (e.g. `connect-src 'self' ws://localhost:1421; script-src 'self' 'unsafe-eval'`), and
     update the dev workflow to pass `--config src-tauri/tauri.dev.conf.json` when running `tauri dev`.
     Flag this gap explicitly to the user/discuss-phase rather than silently treating D-17 as
     "already solved."

2. **Is making web-mode `create_backup` downloadable in scope for this phase?**
   - What we know: it's a real, verified gap (Pitfall 5) but not one of H1's four named handlers, and
     Phase 8 Plan 10 already made a deliberate decision to keep it server-side-only for web mode.
   - What's unclear: whether the user considers this "close enough to H1's spirit" to fold in, or a
     separate follow-up.
   - Recommendation: raise this explicitly during discuss-phase/planning rather than assuming either
     answer.

3. **Raw-bytes body vs. `axum::extract::Multipart` for the upload endpoints — final call left to planner.**
   - What we know: raw-bytes needs zero new dependencies and is sufficient for this phase's exact
     flows (format/confirmed are always separate follow-up JSON calls, never bundled with the file in
     one request). Multipart is the more conventional "file upload" extractor and was the pattern
     explicitly named in the phase's research brief.
   - Recommendation: raw-bytes (Pattern 1) unless the planner has a concrete reason to want
     multi-field-in-one-request semantics.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|--------------|-----------|---------|----------|
| Rust/Cargo toolchain | All backend work (`core`, `web-src-tauri`) | ✓ (workspace already builds — `Cargo.toml`/`Cargo.lock` present and consistent) | pinned via `Cargo.lock` (axum 0.7.9 confirmed) | — |
| Node/npm | Frontend work (`vitest`, `vite`) | ✓ (`package.json`/`vitest.config.ts` present, existing scripts run) | not independently reprobed this session (no environment change needed) | — |
| No external service (DB, cache, network dependency) is introduced by this phase | — | n/a | — | Everything is in-process (throttle store, semaphore, session cache) per D-06's explicit "no external contract" |

**Missing dependencies with no fallback:** none.
**Missing dependencies with fallback:** none — this phase adds no new external dependency in its
required path.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | `cargo test` (Rust workspace: `core`, `web-src-tauri`) + `vitest` (frontend TS) |
| Config file | root `Cargo.toml` (`[workspace] members = [...]`); `vitest.config.ts` |
| Quick run command | `cargo test -p pwmgr-web` (fast, this phase's surface); `npm run test` |
| Full suite command | `cargo test --workspace`; `npm run test` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|---------------------|-------------|
| SEC-01 | Upload/download redesign accepts no `path` field; a crafted `../../etc/passwd`-style value in any remaining string field has nowhere to reach `std::fs` | integration | `cargo test -p pwmgr-web --test http_integration` | ✅ file exists (`web-src-tauri/tests/http_integration.rs`), ❌ new test cases needed (Wave 0 gap) |
| SEC-02 | 5 failed logins for one key → locked; locked key gets 429 with `"throttled"`; 6th concurrent login while 3 permits held → 429 with `"rate_limited"` | unit (throttle-window math) + integration (real HTTP 429s) | `cargo test -p pwmgr-web` (unit) + `cargo test -p pwmgr-web --test http_integration` | ❌ Wave 0 gap — no throttle-store test module exists yet |
| SEC-03 | Every response (including the static SPA fallback) carries CSP + `X-Content-Type-Options: nosniff` + `Referrer-Policy` + `X-Frame-Options` | integration | `cargo test -p pwmgr-web --test http_integration` | ❌ Wave 0 gap — new test cases needed |

### Sampling Rate
- **Per task commit:** `cargo test -p pwmgr-web`
- **Per wave merge:** `cargo test --workspace && npm run test`
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] New throttle-store unit test module (proposed `web-src-tauri/src/throttle_store.rs`'s own
      `#[cfg(test)] mod tests` block, mirroring `session_store.rs`'s existing `FakeClock` pattern) —
      covers SEC-02's exact D-08 window-doubling/reset sequence.
- [ ] New cases appended to `web-src-tauri/tests/http_integration.rs` — covers SEC-01 (no path
      accepted, upload→read→preview→commit round-trip works over real HTTP) and SEC-03 (headers
      present on both an API route and the static fallback route).
- [ ] Frontend: no existing test file covers `CsvImport.tsx`/`CsvExport.tsx`/`Backup.tsx` at all
      (`.planning/codebase/CONCERNS.md` already flags "React Screens Not Tested" as a pre-existing
      gap) — this phase doesn't need to fully close that gap, but any new `isTauri`-branching logic
      added to these screens should get at minimum a focused unit test if the planner wants
      Nyquist-level coverage of the new branch, not just the backend.

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|----------------|---------|---------------------|
| V2 Authentication | yes | Existing Argon2id + constant-time compare (unchanged); this phase adds throttle/lockout (V2.2-class control) |
| V3 Session Management | no change | `SessionStore`/cookie handling already exists and is out of this phase's scope (only extended with the new `upload_cache` field) |
| V4 Access Control | no change | `auth_middleware` already gates all protected routes; this phase doesn't alter authorization logic |
| V5 Input Validation | yes | Upload bytes are opaque payloads validated by existing CSV/backup parsers (`portio.rs`, `backup.rs`) — no new parsing surface introduced beyond what already exists, just a new transport for the same bytes |
| V6 Cryptography | no change (concurrency-only) | Argon2 params (`m=65536, t=3, p=4`) unchanged; this phase only bounds *concurrent* derivations, never modifies KDF cost |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|------------------------|
| Path traversal / arbitrary file read-write (H1) | Tampering, Information Disclosure | Eliminated structurally — no path is ever accepted from the client (D-01), not validated-and-confined |
| Online credential brute force (M1) | Spoofing | Per-username + per-IP exponential lockout (Pattern 3) |
| Memory-exhaustion DoS via KDF flood (M2) | Denial of Service | Bounded `Semaphore`, immediate 429 on saturation, no queueing (Pattern 4) |
| Reflected/stored XSS escalating to full vault compromise via DOM/IPC access (M3) | Tampering, Information Disclosure, Elevation of Privilege | Strict CSP (no `'unsafe-inline'`/`'unsafe-eval'` in prod), `X-Content-Type-Options: nosniff` (Pattern 5) |
| Clickjacking (framing the app in a malicious page) | Spoofing, Tampering | `X-Frame-Options: DENY` + `frame-ancestors 'none'` (D-18 + proposed CSP directive) |

## Sources

### Primary (HIGH confidence — read directly this session)
- `SECURITY_CONCERNS.md` — H1/M1/M2/M3 findings, verbatim fix text quoted above.
- `09-CONTEXT.md` — all 18 locked decisions (D-01–D-18), Claude's Discretion, Deferred Ideas.
- `web-src-tauri/src/{handlers.rs, middleware.rs, session_store.rs, error_map.rs, main.rs, lib.rs}` —
  exact current router/middleware/session shape.
- `core/src/{commands.rs, crypto.rs, error.rs, paths.rs}` — exact current handler signatures, Argon2
  params (`m=65536, t=3, p=4`), error-code catalog, `AppPaths` (no `exports_dir()` exists, only
  `backups_dir()`).
- `src/screens/{CsvImport.tsx, CsvExport.tsx, Backup.tsx}`, `src/lib/api.ts`,
  `src/components/PasswordGeneratorPanel.tsx`, `index.html`, `src/App.css`, `package.json`,
  `vite.config.ts` — verified the frontend `isTauri`-branching gap and CSP directive rationale.
- `Cargo.lock` — axum 0.7.9 confirmed pinned; no `multer` currently in the tree (confirms Multipart
  feature is not currently enabled).
- `gsd_run query package-legitimacy check --ecosystem crates multer` — OK verdict, verified.

### Secondary (MEDIUM confidence — official docs, fetched this session)
- `docs.rs/axum/latest/axum/extract/struct.Multipart.html` — Multipart feature flag, `multer`
  dependency, default 2 MiB body limit, `DefaultBodyLimit` usage.
- `docs.rs/tokio/latest/tokio/sync/struct.Semaphore.html` — `const_new`, `try_acquire` semantics, no
  extra feature required.
- `v2.tauri.app/security/csp` — CSP applies in `tauri dev`, not just production builds.
- `v2.tauri.app/develop/configuration-files` — `--config` CLI flag / JSON Merge Patch overlay
  mechanism for platform/environment-specific config.
- `rustsec.org/advisories/RUSTSEC-2022-0055.html` — origin of axum's 2 MiB default body limit for the
  `Bytes` extractor family.

### Tertiary (LOW confidence — flagged where used)
- The exact re-export path for `axum::body::Bytes` vs. requiring a direct `bytes` dependency (A4 in
  Assumptions Log) — my doc.rs lookups for this specific page 404'd; not confirmed this session.

## Metadata

**Confidence breakdown:**
- Standard stack (no new required dependency): HIGH — grounded directly in this repo's `Cargo.toml`/
  `Cargo.lock` plus official docs for the one optional dependency.
- Architecture (upload/download redesign, throttle store, semaphore, CSP layer): HIGH for the shape
  (all patterns mirror existing, already-reviewed code in this repo) — MEDIUM for a few exact
  numeric/syntax details (import path for `Bytes`, exact throttle values) flagged individually above.
- Pitfalls: HIGH — five of six are verified directly against this repo's actual source (not general
  axum/Rust folklore); the sixth (mutex poisoning) is a documented, pre-existing repo finding
  (SECURITY_CONCERNS.md L1) applied to new code.

**Research date:** 2026-09-18
**Valid until:** 30 days (stable stack; no fast-moving dependency introduced) — but the D-17 dev/prod
CSP gap (Open Question 1) should be resolved with the user before planning locks in a specific
mechanism, since it's a genuine unknown rather than a stale-research-window concern.
