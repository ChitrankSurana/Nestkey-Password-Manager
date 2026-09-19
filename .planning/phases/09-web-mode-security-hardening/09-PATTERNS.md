# Phase 09: Web Mode Security Hardening - Pattern Map

**Mapped:** 2026-09-18
**Files analyzed:** 13
**Analogs found:** 11 / 13

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|-----------------|---------------|
| `web-src-tauri/src/throttle_store.rs` (NEW) | store/service | event-driven (in-process, swept) | `web-src-tauri/src/session_store.rs` | exact (explicitly named analog in CONTEXT D-06) |
| `web-src-tauri/src/kdf_guard.rs` (NEW, or inline) | utility/middleware | request-response (concurrency gate) | `web-src-tauri/src/session_store.rs::SessionStore` (shape) + `tokio::sync::Semaphore` (stdlib) | role-match |
| `web-src-tauri/src/middleware.rs` (add `security_headers_middleware`) | middleware | request-response | `web-src-tauri/src/middleware.rs::csrf_middleware` | exact (same file, same convention) |
| `web-src-tauri/src/handlers.rs` (add `upload_csv`/`upload_backup`, rewrite `export_csv`/`read_csv_headers`/`import_csv_preview`/`restore_backup`, wrap `login`/`register`/`export_csv` with throttle+semaphore) | controller | request-response + file-I/O(bytes-over-HTTP) | `web-src-tauri/src/handlers.rs::login`/`issue_session` (auth flow) + `export_csv`/`restore_backup` (existing path-based handlers being replaced) | exact |
| `web-src-tauri/src/handlers.rs::build_router` (add layers/routes) | route wiring | request-response | `web-src-tauri/src/handlers.rs::build_router` (lines 646-708, itself) | exact |
| `web-src-tauri/src/error_map.rs` (add `"throttled"`/`"rate_limited"` arms) | utility (error mapping) | transform | `web-src-tauri/src/error_map.rs::status_for` | exact |
| `core/src/commands.rs` (add `_bytes`-suffixed variants of `do_export_csv`/`do_read_csv_headers`/`do_import_csv_preview`/`do_restore_backup`) | service | transform | `core/src/commands.rs` (existing path-based fns, same file) | exact |
| `core/src/error.rs` (add `throttled_error()`/`rate_limited_error()`) | utility | transform | `core/src/error.rs` (existing `forbidden_error()`/`invalid_input_error()` constructors) | exact |
| `src/lib/api.ts` (add `httpUpload`, 429-retry in `httpPost`, blob-download helper) | service/client-transport | request-response + file-I/O | `src/lib/api.ts::httpPost` (lines 143-160) | exact |
| `src/screens/CsvImport.tsx` (add `isTauri` branch) | component | file-I/O | `src/screens/Backup.tsx::handleCreate` (lines 13-32, already `isTauri`-branched) | exact |
| `src/screens/CsvExport.tsx` (add `isTauri` branch) | component | file-I/O | `src/screens/Backup.tsx::handleCreate` (lines 13-32) | exact |
| `src/screens/Backup.tsx::handleRestore` (add `isTauri` branch) | component | file-I/O | `src/screens/Backup.tsx::handleCreate` (lines 13-32, same file, sibling handler) | exact |
| `src-tauri/tauri.conf.json` (`app.security.csp`) + new `src-tauri/tauri.dev.conf.json` | config | config | none — `"csp": null` at line 15 is the field being replaced; no existing dev/prod config-overlay convention in this repo (see Open Question in RESEARCH.md) | no analog |

## Pattern Assignments

### `web-src-tauri/src/throttle_store.rs` (NEW — store, event-driven)

**Analog:** `web-src-tauri/src/session_store.rs`

**Imports pattern** (lines 1-6):
```rust
use pwmgr_core::autolock;
use pwmgr_core::session::Clock;
use pwmgr_core::state::AppState;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
```
Use the same `std::collections::HashMap` + `std::sync::RwLock` shape (no new dependency, per D-06). Drop the `pwmgr_core::state::AppState`/`autolock` imports (not relevant to throttle) but keep `Clock`/`Instant`/`Duration`.

**Core store shape** (lines 8-45, `UserSession`/`SessionStore`):
```rust
pub struct UserSession {
    pub username: String,
    pub app_state: AppState,
    pub last_seen: Mutex<Instant>,
}

#[derive(Clone, Default)]
pub struct SessionStore {
    inner: Arc<RwLock<HashMap<Token, Arc<UserSession>>>>,
}

impl SessionStore {
    pub fn new() -> Self { Self::default() }

    pub fn insert(&self, session: UserSession) -> Token {
        let token = new_token();
        self.inner.write().unwrap().insert(token.clone(), Arc::new(session));
        token
    }

    pub fn get(&self, token: &str) -> Option<Arc<UserSession>> {
        self.inner.read().unwrap().get(token).cloned()
    }

    pub fn remove(&self, token: &str) {
        self.inner.write().unwrap().remove(token);
    }
```
`ThrottleStore` should mirror this exactly: `Arc<RwLock<HashMap<String, ThrottleEntry>>>`, `check(key)`, `record_failure(key, clock)`, `record_success(key)`. Key format: `format!("user:{username}")` / `format!("ip:{ip}")` per D-09.

**Sweep pattern** (lines 46-58, `sweep`):
```rust
pub fn sweep(&self, clock: &dyn Clock) {
    {
        // never holds the map write lock while ticking - ticking takes the read lock,
        // so a slow handler can't be starved by the sweep
```
Follow the same "never hold write lock across expensive work" discipline; the throttle sweep only needs to prune stale (long-expired) entries to bound memory, called from the same periodic-task loop `main.rs` already runs for `SessionStore::sweep`.

**Poisoning-safety caveat (Pitfall 6 from RESEARCH.md):** `SessionStore` uses `.lock().unwrap()`/`.write().unwrap()` throughout — acceptable there per existing L1 risk acceptance, but the NEW throttle store's `record_failure` must use `saturating_mul`/checked arithmetic for the window-doubling math (never let a `Duration` overflow-panic while holding the write lock).

**Test pattern** (lines 93-194, `#[cfg(test)] mod tests` + `FakeClock`):
```rust
#[cfg(test)]
mod tests {
    struct FakeClock(RefCell<Instant>);
    impl FakeClock {
        fn new() -> Self { FakeClock(RefCell::new(Instant::now())) }
    }
    impl Clock for FakeClock { /* ... */ }
```
Reuse this exact `FakeClock` pattern (or the same trait) in `throttle_store.rs`'s own test module to deterministically test the D-08 window-doubling/reset sequence without real sleeps.

---

### `web-src-tauri/src/middleware.rs` (add `security_headers_middleware`)

**Analog:** `web-src-tauri/src/middleware.rs::csrf_middleware` (same file)

**Imports already present** (lines 1-8):
```rust
use crate::error_map::ApiError;
use crate::session_store::{SessionStore, Token};
use axum::body::Body;
use axum::extract::State;
use axum::http::{header, Method, Request};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use pwmgr_core::session::SystemClock;
```
Add `axum::http::HeaderValue` for header injection.

**Core pattern** (lines 42-47, `csrf_middleware` — the template to follow):
```rust
pub async fn csrf_middleware(req: Request<Body>, next: Next) -> Response {
    if req.method() != Method::GET && req.headers().get(CSRF_HEADER).is_none() {
        return ApiError::from(pwmgr_core::error::forbidden_error("Missing CSRF header."))
            .into_response();
    }
    next.run(req).await
}
```
`security_headers_middleware(req, next) -> Response` should call `next.run(req).await` first, then mutate `response.headers_mut()` to insert CSP/`X-Content-Type-Options`/`Referrer-Policy`/`X-Frame-Options` (D-18) before returning — applies unconditionally to every response including the static fallback, matching how `csrf_middleware` is layered.

**Layering pattern** (from `handlers.rs::build_router`, line 707):
```rust
.layer(axum::middleware::from_fn(crate::middleware::csrf_middleware))
```
Add `.layer(axum::middleware::from_fn(crate::middleware::security_headers_middleware))` the same way — outermost layer so it runs on every route including the `ServeDir` fallback.

**Auth pattern reference** (lines 18-37, `auth_middleware`) — not directly reused here but is the sibling pattern the throttle-check middleware/inline check (D-06) should follow if implemented as a middleware layer rather than inline in `login`/`register`:
```rust
pub async fn auth_middleware(
    State(store): State<SessionStore>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    let token = req.headers().get(header::COOKIE).and_then(|h| h.to_str().ok()).and_then(parse_session_cookie);
    let session = match token.and_then(|t| store.get(&t)) {
        Some(s) => s,
        None => return unauthorized(),
    };
    ...
    next.run(req).await
}
```

---

### `web-src-tauri/src/handlers.rs` (upload/download redesign + throttle/semaphore wiring)

**Analog:** `web-src-tauri/src/handlers.rs::login`/`issue_session` (lines 226-254) for the throttle/semaphore wrapping pattern; the existing `ExportCsvRequest`/`RestoreBackupRequest`/`ReadCsvHeadersRequest`/`ImportCsvPreviewRequest` structs (lines 197-220) for what's being removed.

**Current request DTOs being replaced** (lines 197-220):
```rust
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RestoreBackupRequest {
    path: String,
    confirmed: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExportCsvRequest {
    path: String,
    mode: ExportMode,
    confirmed: bool,
}

#[derive(Deserialize)]
struct ReadCsvHeadersRequest {
    path: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportCsvPreviewRequest {
    path: String,
    format: ImportFormat,
}
```
Per D-01/D-05: drop `path: String` from all four; `ExportCsvRequest` becomes `{ mode, confirmed }`, `RestoreBackupRequest` becomes `{ confirmed }` (or bodyless + a separate confirm call), `ReadCsvHeadersRequest`/`ImportCsvPreviewRequest` become bodyless/format-only, reading from `session.upload_cache` instead.

**Auth/session extraction + error-mapping pattern** (lines 226-244, `issue_session`):
```rust
fn issue_session(ctx: &AppContext, body: &AuthRequest, register: bool) -> Result<Response, ApiError> {
    let app_state = build_locked_state(&ctx.paths)?;
    if register {
        do_register(&app_state, &ctx.paths, &body.username, &body.master_password)?;
    } else {
        do_login(&app_state, &ctx.paths, &body.username, &body.master_password)?;
    }
    let session = UserSession { username: body.username.clone(), app_state, last_seen: Mutex::new(Instant::now()) };
    let token = ctx.store.insert(session);
    let cookie = format!("session={token}; HttpOnly; SameSite=Strict; Path=/");
    let mut res = Json(AuthSuccess { username: body.username.clone() }).into_response();
    res.headers_mut().insert(header::SET_COOKIE, cookie.parse().unwrap());
    Ok(res)
}

async fn login(State(ctx): State<AppContext>, Json(body): Json<AuthRequest>) -> Result<Response, ApiError> {
    issue_session(&ctx, &body, false)
}
```
`?`-propagation via `Result<_, ApiError>` is the established idiom — new throttle-check and semaphore-`try_acquire` calls should return `Err(ApiError::from(pwmgr_core::error::throttled_error(...)))` / `rate_limited_error(...)` the same way, and be inserted at the top of `login`/`register`/password-mode `export_csv` bodies (throttle check before semaphore acquire, per RESEARCH.md Pattern 4's ordering note).

**Router wiring** (lines 646-708, `build_router`) — new `/api/upload_csv`, `/api/upload_backup` routes go in the `protected` router (same block as `export_csv`/`restore_backup`, line 683-688) since they need an authenticated session:
```rust
.route("/api/create_backup", post(create_backup))
.route("/api/restore_backup", post(restore_backup))
.route("/api/export_csv", post(export_csv))
.route("/api/read_csv_headers", post(read_csv_headers))
.route("/api/import_csv_preview", post(import_csv_preview))
.route("/api/import_csv_commit", post(import_csv_commit))
```
followed by
```rust
.route_layer(axum::middleware::from_fn_with_state(ctx.store.clone(), crate::middleware::auth_middleware));
```
Add the new upload routes here, then a route-scoped `DefaultBodyLimit::max(25 * 1024 * 1024)` layer applied only to the upload routes (not the whole router) per Pitfall 4 in RESEARCH.md.

---

### `web-src-tauri/src/error_map.rs` (add `"throttled"`/`"rate_limited"` arms)

**Analog:** `web-src-tauri/src/error_map.rs::status_for` (itself, lines 18-27)

**Core pattern** (exhaustive match, catch-all last):
```rust
pub fn status_for(code: &str) -> StatusCode {
    match code {
        "invalid_credentials" | "locked" => StatusCode::UNAUTHORIZED,
        "invalid_input" => StatusCode::BAD_REQUEST,
        "not_found" => StatusCode::NOT_FOUND,
        "username_taken" | "confirm_required" => StatusCode::CONFLICT,
        "forbidden" => StatusCode::FORBIDDEN,
        "internal_error" => StatusCode::INTERNAL_SERVER_ERROR,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
```
Add `"throttled" | "rate_limited" => StatusCode::TOO_MANY_REQUESTS,` as a new arm above the catch-all (Pitfall 1 in RESEARCH.md — missing this silently 500s instead of 429s). `ApiError`/`IntoResponse` wrapper (lines 7-33) needs no change — it already calls `status_for(self.0.code())` generically.

---

### `src/lib/api.ts` (add `httpUpload`, 429-retry, blob-download)

**Analog:** `src/lib/api.ts::httpPost` (lines 143-160, itself)

**Full existing pattern to extend**:
```typescript
async function httpPost<T>(command: string, args: Record<string, unknown>): Promise<T> {
  const res = await fetch(`/api/${command}`, {
    method: "POST",
    headers: { "Content-Type": "application/json", "X-PWMGR-CSRF": "1" },
    credentials: "same-origin",
    body: JSON.stringify(args),
  });
  if (!res.ok) {
    let err: CommandError = { code: "internal_error", message: "Something went wrong. Please try again." };
    try {
      err = (await res.json()) as CommandError;
    } catch {
      // Non-JSON error body (e.g. a raw 404 from the static layer) - keep the generic fallback.
    }
    throw err;
  }
  const text = await res.text();
  return (text ? JSON.parse(text) : undefined) as T;
}
```
D-14's 429-retry: after the `if (!res.ok)` branch parses `err`, check `err.code === "rate_limited"` and retry once after a 1-2s backoff (do NOT retry on `"throttled"` — surface immediately, per Pitfall 2 in RESEARCH.md). New `httpUpload(command, file: File)` should follow the same `fetch("/api/...", { credentials: "same-origin", headers: { "X-PWMGR-CSRF": "1" }, ... })` shape but with `Content-Type: application/octet-stream` and `body: file` instead of JSON.

**Dispatch pattern** (lines 141, 164-167 — the `isTauri` gate every new web-only helper must respect):
```typescript
export const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
...
function call<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  return isTauri ? invoke<T>(command, args) : httpPost<T>(command, args);
}
```
New upload/download helpers are web-only (no Tauri IPC branch needed) — called directly from screens behind an `if (!isTauri)` check, not routed through `call()`.

---

### `src/screens/CsvImport.tsx`, `src/screens/CsvExport.tsx`, `src/screens/Backup.tsx::handleRestore`

**Analog:** `src/screens/Backup.tsx::handleCreate` (lines 13-32, same file — the one handler among these four that already branches on `isTauri`)

**Core pattern to replicate**:
```typescript
async function handleCreate() {
  let target: string | null = null;
  if (isTauri) {
    target = await save({
      defaultPath: "vault-backup.bak",
      filters: [{ name: "Backup", extensions: ["bak"] }],
    });
    if (!target) return; // cancelled - no busy state, no error, no status change
  }

  setBusy(true);
  try {
    setError(null);
    setStatus(null);
    const path = await api.createBackup(target);
    setStatus(`Backup written to ${path}`);
  } catch (err) {
    setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
  } finally {
    setBusy(false);
  }
}
```
`handleRestore` (currently unconditional, lines 34-... calling `open()` from `@tauri-apps/plugin-dialog`) needs the same `if (isTauri) { ... } else { /* web: <input type="file"> flow */ }` branch structure. `CsvImport.tsx`/`CsvExport.tsx` currently have NO `isTauri` branch at all (verified in RESEARCH.md Pitfall 3) — this is new code, not a modification; use `handleCreate`'s try/catch/finally + `isCommandError` error-handling shape as the template for the new web branch.

**Import pattern** (lines 1-6 of `Backup.tsx`):
```typescript
import { useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { api, isCommandError, isTauri } from "../lib/api";
import { confirmDialog } from "../lib/confirm";
import { Tooltip } from "../components/Tooltip";
import { Icon } from "../components/Icon";
```
Same import shape applies to `CsvImport.tsx`/`CsvExport.tsx` (swap `@tauri-apps/plugin-dialog`'s `open`/`save` for the desktop branch only; web branch needs no new import beyond a raw `<input type="file">` ref/handler and `api.uploadCsv`/`api.downloadExport`-style new API surface).

## Shared Patterns

### Error mapping (throttled/rate_limited)
**Source:** `web-src-tauri/src/error_map.rs::status_for` (lines 18-27) + `core/src/error.rs` existing constructors (`forbidden_error()`, `invalid_input_error()` — same file, same convention)
**Apply to:** `throttle_store.rs`, `kdf_guard.rs`/semaphore wrapper, `handlers.rs::login`/`register`/`export_csv`
```rust
match code {
    "invalid_credentials" | "locked" => StatusCode::UNAUTHORIZED,
    ...
    _ => StatusCode::INTERNAL_SERVER_ERROR,
}
```
Add explicit `"throttled"`/`"rate_limited"` arms — never rely on the catch-all for new codes (Pitfall 1).

### `Result<_, ApiError>` + `?` propagation
**Source:** `web-src-tauri/src/handlers.rs` throughout (e.g. `issue_session`, `login`)
**Apply to:** All new/modified handlers (upload_csv, upload_backup, export_csv, throttle-checked login/register)

### In-process store shape (Arc<RwLock<HashMap<...>>>, sweep, FakeClock tests)
**Source:** `web-src-tauri/src/session_store.rs` (whole file)
**Apply to:** `throttle_store.rs` (explicitly directed by D-06) and, if the upload cache is factored as its own small store rather than a `UserSession` field, that too (though D-04 says it should live directly on `UserSession`, not a separate store)

### `isTauri` transport/UI dispatch gate
**Source:** `src/lib/api.ts:141` (`isTauri` export) + `src/screens/Backup.tsx::handleCreate` (lines 13-32)
**Apply to:** `CsvImport.tsx`, `CsvExport.tsx`, `Backup.tsx::handleRestore`, and any new `api.ts` upload/download helper

### Middleware layering via `axum::middleware::from_fn`
**Source:** `web-src-tauri/src/middleware.rs::csrf_middleware` + its attachment at `handlers.rs:707`
**Apply to:** New `security_headers_middleware`

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `src-tauri/tauri.conf.json` (`app.security.csp`) | config | config | Currently `"csp": null` (line 15) — being replaced with a real string, no prior pattern to copy from within this repo |
| `src-tauri/tauri.dev.conf.json` (NEW) | config | config | No existing dev/prod config-overlay mechanism exists in this repo (RESEARCH.md Open Question 1 — confirmed absent from `package.json` scripts and `src-tauri/`); must be introduced fresh per Tauri's documented `--config` JSON Merge Patch mechanism, not copied from a local analog |

## Metadata

**Analog search scope:** `web-src-tauri/src/` (handlers.rs, middleware.rs, session_store.rs, error_map.rs), `core/src/` (commands.rs, error.rs), `src/lib/api.ts`, `src/screens/` (Backup.tsx, CsvImport.tsx, CsvExport.tsx), `src-tauri/tauri.conf.json`
**Files scanned:** 10
**Pattern extraction date:** 2026-09-18
