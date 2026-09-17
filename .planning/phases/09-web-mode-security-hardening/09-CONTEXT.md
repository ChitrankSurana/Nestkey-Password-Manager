# Phase 09: Web Mode Security Hardening - Context

**Gathered:** 2026-09-18
**Status:** Ready for planning

<domain>
## Phase Boundary

The localhost web server (`web-src-tauri`) can no longer be used to read/write arbitrary files,
brute-force logins, or exhaust memory via unbounded Argon2 derivations, and it ships baseline
security headers — closing H1, M1, M2, and M3 from `SECURITY_CONCERNS.md`, mapped to SEC-01,
SEC-02, and SEC-03 in `.planning/REQUIREMENTS.md`. Desktop (`src-tauri`) is out of scope except
for the single `tauri.conf.json` CSP setting, which SEC-03 explicitly names.

</domain>

<decisions>
## Implementation Decisions

### Path confinement (SEC-01 / H1)

- **D-01:** Web mode adopts the full upload/download redesign, not path-confinement. Exports
  become an HTTP download response body; CSV import and backup restore accept an uploaded file
  body. No server-side path is ever named by a web client for any of these four handlers.
  — **Reversibility:** costly — rewrites the `export_csv`/`read_csv_headers`/
  `import_csv_preview`/`restore_backup` web-mode API contract (request/response shape), touching
  `web-src-tauri/src/handlers.rs`, `src/lib/api.ts`, and the Data/Backup/CsvExport/CsvImport
  screens. Reverting to path-confinement later means re-touching all of those call sites again.
- **D-02:** This redesign is web-mode only. Desktop keeps its existing native-file-dialog path
  flow (`do_export_csv`, `do_restore_backup`, `do_read_csv_headers`, `do_import_csv_preview`
  still take a `Path` on the desktop Tauri command layer) — that path comes from the user's own
  OS dialog, not attacker-controlled input, so it isn't the H1 threat surface.
- **D-03:** CSV import's multi-step flow (headers → preview → commit) uploads the file once and
  caches the bytes server-side rather than having the client re-upload at each step.
- **D-04:** The cached upload lives on the existing per-session struct in
  `web-src-tauri/src/session_store.rs` (`UserSession`) — not a new global store, not a temp file
  on disk. It is cleared explicitly on commit success, on user cancel, and on session end/logout.
- **D-05:** `restore_backup` gets the same treatment as CSV import: the client uploads the backup
  file directly; no path parameter for restore in web mode either.

### Login throttling (SEC-02 / M1)

- **D-06:** Attempt/lockout state lives in a new in-process store in `web-src-tauri`, alongside
  `session_store.rs`, swept by the same periodic-task pattern the session sweep already uses. No
  new dependency, no `registry.db` schema change — throttle state does not need to survive a
  server restart.
- **D-07:** `/api/register_user` (currently open, pre-auth) gets the identical throttle/lockout
  treatment as `/api/login` — SECURITY_CONCERNS.md M1 names both in the same finding.
- **D-08:** Backoff/lockout shape (**user-specified, not a menu option — implement exactly this**):
  5 failed attempts locks that key out for a fixed base window `W`. If the same key fails again
  (still attacking) after a lockout expires and hits the 5-attempt threshold again, the *next*
  lockout window doubles (`W → 2W → 4W → ...`) — the lockout window itself grows exponentially
  across repeated lockout cycles, not a per-attempt delay curve.
  — **Reversibility:** reversible — isolated to the new throttle store; no external contract.
- **D-09:** Throttle keys are tracked both by username and by IP, independently. Whichever limit
  is hit first blocks the request.
- **D-10:** The exponential window multiplier (and the failure count) resets back to the base
  window on the next successful login for that username/IP — not on a time-based cooldown.

### Argon2 concurrency cap (SEC-02 / M2)

- **D-11:** A saturated concurrency limit rejects immediately (`try_acquire`, HTTP 429) rather
  than queueing the request. Chosen over queueing specifically for DoS resistance — a queue could
  itself become a pile-up vector under attack.
- **D-12:** Permit count is a small fixed number (2–4 in-flight derivations), per
  SECURITY_CONCERNS.md M2's own suggestion ("max 3 in-flight"). Caps worst-case memory at
  permits × 64 MiB regardless of burst size. Exact number is the planner/researcher's call within
  this 2–4 range.
- **D-13:** One shared semaphore covers all three Argon2-driving handlers named in M2: `login`,
  `register_user`, and `export_csv` (password-protected mode) — all are the same memory-flood
  attack surface.
- **D-14:** On a 429, the web frontend (`src/lib/api.ts`) automatically retries with a short
  backoff (1–2s), invisible to the user unless it keeps failing. This is new client retry logic,
  not present today.

### CSP & headers (SEC-03 / M3)

- **D-15:** Production CSP is strict: `default-src 'self'; script-src 'self'; style-src 'self'`
  — no `'unsafe-inline'`. Confirmed non-blocking for the one existing inline style use
  (`src/components/PasswordGeneratorPanel.tsx:186`, `style={{ width: ... }}`): React sets that as
  a DOM property assignment (`element.style.width = ...`), not parsed inline CSS text, so
  `style-src` does not need to relax for it.
- **D-16:** The same policy string is used in both places: the `web-src-tauri` response-header
  layer AND `src-tauri/tauri.conf.json`'s `app.security.csp` (replacing the current `null`).
  — **Reversibility:** reversible — a header/config value, no external contract.
- **D-17:** Dev mode (Vite `npm run dev`, HMR over websocket, eval-based sourcemaps) gets a
  relaxed policy (e.g. `connect-src` including `ws://localhost`, `script-src 'unsafe-eval'` for
  sourcemaps); only the production build gets the strict policy from D-15. Gate this on the same
  dev/prod signal the codebase already uses to distinguish Vite dev server from the built bundle.
- **D-18:** While adding the response-header middleware layer for CSP + `X-Content-Type-Options:
  nosniff` (both required by SEC-03's locked wording), also add `Referrer-Policy` and
  `X-Frame-Options: DENY` — SECURITY_CONCERNS.md M3 names both, and they cost nothing extra once
  the layer exists, even though the roadmap's success criteria only name CSP + nosniff by name.

### Claude's Discretion

- Exact Argon2 permit count within the 2–4 range (D-12).
- Exact base lockout window `W` and the doubling cap (if any, to avoid an unbounded lockout for a
  legitimate user who mistypes repeatedly for a long time) — not discussed; planner/researcher
  should propose a concrete value and cap.
- Exact CSP directive values beyond `default-src`/`script-src`/`style-src` (e.g. `img-src`,
  `connect-src` for the production build, `frame-ancestors`) — not discussed in detail.
- Whether the new throttle store and the new Argon2-semaphore live in the same module or separate
  modules within `web-src-tauri` — an internal organization choice.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Audit findings driving this phase (SEC-01/02/03)
- `SECURITY_CONCERNS.md` §H1, §M1, §M2, §M3 — the original audit findings, with **where**/
  **what**/**impact**/**fix** for each; the fix sections directly informed D-01 through D-18.
- `.planning/REQUIREMENTS.md` §"Security Hardening (SEC)" — the locked SEC-01/02/03 wording this
  phase must satisfy.
- `.planning/ROADMAP.md` §"Phase 9: Web Mode Security Hardening" — phase goal and success criteria.

### Codebase context
- `.planning/codebase/ARCHITECTURE.md` — layered monolith overview; `AppState`, session/auth flow,
  Tauri IPC vs. HTTP dispatch via `isTauri` in `src/lib/api.ts`.
- `.planning/codebase/CONCERNS.md` §"Fragile Areas: Web Mode Path Handling", §"Security
  Considerations" — corroborates H1/M1/M2/M3 and adds test-coverage priorities (integration tests
  for web mode are flagged CRITICAL — H1's fix must be evidence-based, not trust-based).
- `web-src-tauri/src/session_store.rs` — existing `SessionStore`/`UserSession` pattern that D-04
  (CSV upload cache) and D-06 (throttle store) both extend rather than duplicate.
- `src-tauri/tauri.conf.json` — current `"csp": null` (line 15) that D-16 replaces.

[No user-referenced docs beyond the above were introduced during discussion.]

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `web-src-tauri/src/session_store.rs::UserSession` / `SessionStore` — per-session state
  container with an existing sweep/prune background-task pattern (24h idle prune). D-04's upload
  cache and D-06's throttle store both follow this same shape (in-process, swept periodically)
  rather than introducing a new persistence layer.
- `web-src-tauri/src/middleware.rs` — existing `csrf_middleware`, applied as an axum
  `middleware::from_fn` layer in `build_router` (`web-src-tauri/src/handlers.rs:707`). The new
  CSP/header layer (D-18) and any throttle-check middleware follow this same layering convention.

### Established Patterns
- Frontend transport dispatch: `src/lib/api.ts` — `isTauri ? invoke(...) : httpPost(...)`. Any
  new web-only behavior (upload/download bodies, 429 retry) is isolated to the HTTP branch; the
  Tauri IPC branch used by desktop is untouched (per D-02).
- Error handling: `isCommandError()` guard + generic-fallback message, used throughout the React
  screens. D-14's 429 retry logic should follow this convention rather than introducing a new
  error-handling pattern.
- `CommandError` DTO (`core/src/dto.rs`) is the serializable error contract; a new 429/throttle
  response should stay consistent with this shape if the retry logic (D-14) inspects the error
  code.

### Integration Points
- `web-src-tauri/src/handlers.rs::build_router` (line ~646) — where the CSRF middleware layer is
  currently attached; the CSP/security-headers layer (D-18) and the throttle-check layer (D-06)
  both attach here.
- `web-src-tauri/src/handlers.rs` lines 199–219 — the four `path: String` fields on the
  export/import/restore request structs that D-01/D-05 remove in favor of multipart/body uploads.
- `web-src-tauri/src/handlers.rs::login` (line 254) — the handler D-06/D-11 wrap with throttle and
  concurrency-cap checks.

</code_context>

<specifics>
## Specific Ideas

- The lockout-window growth shape in D-08 was specified precisely by the user, not chosen from
  offered options: 5 failures → lock for base window `W`; each subsequent lockout cycle (still
  failing after the prior lockout expires) doubles `W`. This is the authoritative shape — do not
  substitute a different backoff curve (e.g. per-attempt exponential delay) during planning.
- The style-src CSP concern (D-15) was resolved by checking the one actual inline-style call site
  in the repo (`PasswordGeneratorPanel.tsx:186`) rather than assuming `'unsafe-inline'` was needed
  — confirm this reasoning holds if any other inline style usage is found during planning/research.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope. The one pending todo checked against this phase
(`2026-09-13-resolve-m4-health-dashboard-weak-classification-doc-conflict.md`) scored 0.2 (very
low relevance — Health Dashboard docs, unrelated to web-mode security) and was not folded in.

### Reviewed Todos (not folded)
- `2026-09-13-resolve-m4-health-dashboard-weak-classification-doc-conflict.md` — cosmetic
  doc-vs-shipped-behavior mismatch in the Health Dashboard (Phase 4), unrelated to Phase 9's web
  security scope. Reviewed via `todo.match-phase`, scored 0.2 (keyword-only match on "plans"), not
  folded in.

</deferred>

---

*Phase: 09-web-mode-security-hardening*
*Context gathered: 2026-09-18*
