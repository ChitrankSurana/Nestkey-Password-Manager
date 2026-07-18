# M6 Design — Localhost Web Mode

**Status:** Approved (brainstormed interactively), pending write-up as an implementation plan via
`superpowers:writing-plans`.
**Last updated:** 2026-07-18

**Scope note:** M6 per `docs/PRD.md` §7 is one roadmap line ("Localhost-bound web mode, responsive
layout") covering two independent pieces of work. This spec covers **only web mode**. Responsive
layout is a separate, later spec/plan/implementation cycle, deliberately not addressed here.

## 1. Purpose

Deliver M6's web-mode half per `docs/PRD.md` user story 12: "As a user (future), I access my vault from
a browser on localhost when I want, without exposing it to the network." `docs/ARCHITECTURE.md:56-57`
already commits to the shape: "Web mode (M6) reuses the same Rust core, adding an HTTP layer bound to
127.0.0.1 only, with a session-token middleware in front of the same commands module." This design
fills in everything that decision left open: which framework, what the session/auth model actually is,
how it relates to the M5 read-only viewer, and how the existing frontend gets served.

## 2. Scope

**In scope:**
- A new binary crate, `web-src-tauri` (package `pwmgr-web`) — a pure Rust HTTP server, no Tauri
  dependency, no GUI window. Depends on `pwmgr-core`, `axum`, `tower`, `tower-http`, `tokio` (full
  runtime, not just the `time` feature `viewer-src-tauri` uses).
- **Full command parity with the desktop app** — every command in `src-tauri/src/commands.rs` becomes
  reachable over HTTP, not a curated subset. This requires finishing the promotion M5's Slice 1 started:
  moving the remaining ~25 `do_*` functions from `src-tauri` into `pwmgr-core` (`do_register`,
  `do_add_site`, `do_add_credential`, `do_update_credential`, `do_soft_delete_credential`,
  `do_list_trash`, `do_restore_credential`, `do_purge_trash`, the 7 settings-update functions
  `do_update_{auto_lock_minutes,strength_algorithm,theme,accent_color,old_password_days,hibp_opt_in,
  nav_collapsed}`, `do_generate_password`, `do_analyze_strength`, `do_create_backup`,
  `do_restore_backup`, `do_health_report`, `do_check_breach`, `do_export_csv`, `do_read_csv_headers`,
  `do_import_csv_preview`, `do_import_csv_commit`) — same mechanical pattern as M5's Task 9 cutover
  (copy into `core`, verify byte-identical, cut `src-tauri` over to import instead of define). After
  this, all three front-ends (`src-tauri`, `viewer-src-tauri`, `pwmgr-web`) consume one shared copy of
  every command; no command's logic exists in more than one place.
- **Multi-session support** — the *first* concurrent-session-capable surface in this codebase. Multiple
  independently-authenticated browser sessions can be unlocked at once, each with its own vault
  connection and lock-timer, tracked in a `token → session` store. This is scoped specifically to
  **one shared OS user account with multiple app-level named vaults** (e.g. a household PC everyone
  logs into the same OS account on, with separate registry-level users like "alice"/"bob") — *not*
  isolation between genuinely separate OS user accounts, which `127.0.0.1` TCP binding does not provide
  and which is explicitly out of scope (see §5).
- **Session-token auth**: 256-bit random token per login (per `docs/TRD.md:68-70`), HttpOnly +
  `SameSite=Strict` cookie, required on every request after login. A background sweep (mirroring the
  existing 2-second `autolock::tick` cadence, generalized to iterate the session map) locks any session
  past its idle timeout; locked entries persist in the map (so the same browser can re-authenticate
  against the same entry) but get pruned after 24h of inactivity to bound memory growth.
- **CSRF mitigation**: `SameSite=Strict` cookie + a required custom header on all mutating (non-GET)
  requests that a simple cross-site form/image tag cannot set. No CORS headers configured at all — the
  web UI is served same-origin, so nothing needs cross-origin access.
- **Frontend reuse, not a rewrite**: the existing built React bundle (`index.html`/`main.tsx`/`App.tsx`
  and its full screen tree) is served as-is by the new server's static-file layer. `src/lib/api.ts`
  gains a runtime environment check (`window.__TAURI__` presence) branching between the existing
  `invoke()` calls and new `fetch()`-to-HTTP calls — no new Vite entry point, no new frontend project.
  This keeps the door open for a future purpose-built web UI (a pure frontend swap against the same
  stable JSON API, zero backend changes) without blocking this milestone on that design work.
- **One explicit behavior port**: `copy_to_clipboard` is not exposed as an HTTP endpoint — a browser
  has its own `navigator.clipboard.writeText` API. The frontend's clipboard call branches: Tauri context
  → existing Tauri command; browser context → direct browser clipboard call, including re-implementing
  the 30-second auto-clear as a JS `setTimeout` (currently Rust-only). This is a real second
  implementation of a security-relevant timing property, not just a routing decision — flagged
  explicitly so it gets its own test coverage rather than inheriting the Rust tests' coverage by
  assumption.
- HTTP status-code mapping for `CommandError`'s existing `code()`/`message()` (`locked`→401,
  `invalid_credentials`→401, `invalid_input`→400, `not_found`→404, `internal_error`→500, etc.) — a
  documented, reviewed convention, not ad hoc per-endpoint choices.
- Binds to `127.0.0.1` only, hard-coded at the listener level — not a config option, matching M5's
  "read-only isn't a UI restriction, it's a compile-time fact" philosophy applied to bind address.

**Out of scope (explicit, so it isn't accidentally built):**
- Responsive layout / CSS — separate spec, not part of this design.
- Isolation between genuinely separate OS user accounts on a shared machine (e.g. a Unix domain socket
  with file permissions instead of/alongside TCP) — out of scope per the shared-machine scoping decision
  in §2 above. If this is needed later, it's a new spec, not an extension of this one.
- A toggle inside the desktop app to start/stop web mode — `pwmgr-web` is a fully separate binary you
  launch directly, not a Settings option inside `src-tauri`.
- A new/redesigned web-specific UI — explicitly deferred; the existing React app is reused as-is for v1.
- Token rotation-per-request or a token lifetime independent of the existing auto-lock mechanism — the
  token stays valid exactly as long as its session-store entry exists.

## 3. Architecture — module placement

- **`pwmgr-core`** gains the remaining ~25 `do_*` functions listed in §2, completing the promotion M5's
  Slice 1 started for the 8 read-only ones. Same "promotion, not rewrite" principle: no behavior change,
  verified via the existing test suite staying green (tests relocate with the code, same as M5's Task 9).
  `core` remains framework-free — no `axum`, no `tokio` beyond what it might already need, no HTTP
  concepts leak into this layer at all. This is a materially bigger and riskier refactor than M5's
  Slice 1 (25 functions across auth, vault-mutation, settings, generator, backup, CSV, health/breach vs.
  8 pure-read functions) — plan accordingly with more granular tasks and review checkpoints than M5 used.
- **`src-tauri`** and **`viewer-src-tauri`**: both switch to importing the now-complete `pwmgr-core`
  command set instead of defining any of it locally (`src-tauri` already does this for the 8 promoted in
  M5; this extends the same pattern to the rest). No functional change to either existing binary.
- **`web-src-tauri`** (new): an `axum::Router` with one thin HTTP handler per command — extracts the
  authenticated session from a request-scoped extension (injected by the auth middleware), calls the
  matching promoted `do_*` function, serializes the result to JSON. No business logic in this crate,
  same "thin wrapper" philosophy as the Tauri command wrappers. Also owns: the session-store module
  (token→session map + the sweep/prune background task), the auth+CSRF `tower::Layer` middleware, and
  the static-file layer serving the built `dist/` frontend assets.
- **Session store shape**: `Arc<RwLock<HashMap<Token, UserSession>>>`, where each `UserSession` bundles
  its own vault connection (`Mutex<Option<(Connection, PathBuf)>>`) and its own lock-timer `Session`
  (from `pwmgr-core::session`, unchanged type) — independent per logged-in browser/user. The registry
  connection stays shared/single across all sessions (registry.db is user metadata only, not secrets,
  same trust level as today's single-registry-per-process model).
- **Dependencies**: `web-src-tauri`'s `Cargo.toml` needs everything `src-tauri` needs *except* Tauri
  itself and its plugins (`tauri`, `tauri-plugin-*`) — `axum`, `tower`, `tower-http` replace them.
  `reqwest` (HIBP client) and `zxcvbn`/`csv` stay, since full parity includes those features.
- **Data location**: same fixed on-disk paths as the other two binaries (`~/.pwmgr/registry.db`,
  `~/.pwmgr/vaults/<user>.vault`, `~/.pwmgr/config.json`) via the existing `AppPaths` resolution — no
  new path logic.

## 4. Data flow

**Login:** Browser POSTs credentials to `POST /api/login` → server calls the existing `auth::login`
(unchanged, in `pwmgr-core`) → on success, creates a new `UserSession` entry keyed by a fresh 256-bit
token, stores it in the session-store, sets the HttpOnly/`SameSite=Strict` cookie → subsequent requests
carry that cookie, resolved by the auth middleware before reaching any handler.

**Every other command:** browser fetch → auth middleware resolves the session from the cookie (401 if
missing/invalid/locked) → CSRF middleware checks the anti-CSRF header on mutating requests (403 if
missing) → thin axum handler extracts request params → calls the matching promoted `do_*` function
(identical call as any other front-end makes) → `CommandError` (if any) maps to an HTTP status + JSON
body via the documented table in §2; success maps to 200 + JSON body.

**Idle/lock:** background sweep task ticks every 2 seconds (same cadence as today's single-session
autolock), iterating the session map, calling each entry's existing `Session::check_expiry` — matches
today's single-session `autolock::tick` logic exactly, just applied per-entry instead of once. A locked
entry's vault connection is cleared (same as today); the entry itself persists until the 24h inactivity
prune removes it.

**Clipboard:** browser-side only, per §2 — no HTTP round-trip, `navigator.clipboard.writeText` +
JS-side 30-second auto-clear timer, mirroring (not sharing) the existing Rust-side behavior.

## 5. Security notes

- **The `127.0.0.1`-only bind is the primary network-exposure mitigation** (per `docs/THREAT_MODEL.md`
  A4, already documented) — LAN traffic to this feature never occurs.
- **Session-token + `SameSite=Strict` cookie + required CSRF header together close the CSRF gap** a bare
  cookie alone leaves open (a cookie is auto-attached by the browser on cross-origin requests even with
  `HttpOnly`; `SameSite=Strict` stops modern browsers from attaching it cross-site at all, and the
  header requirement is a second line of defense for older/misconfigured browsers). **This needs its own
  `docs/THREAT_MODEL.md` entry** (extending the existing A4 line, which currently only documents the
  bind-address + token mitigations, not CSRF specifically) **and dedicated test coverage** — a bug in
  this middleware silently reopens the hole, and nothing else in the codebase would catch it.
- **Scoping decision, stated explicitly so it isn't silently assumed**: multi-session support is for one
  shared OS user account with multiple app-level vaults, not isolation between separate OS accounts.
  `127.0.0.1` TCP is reachable by any process/user on the machine; if genuine OS-user isolation is ever
  needed, that requires a different mechanism (e.g. a Unix domain socket with restrictive file
  permissions on platforms that support it) and is explicitly a future spec, not assumed to already work
  here.
- **Resource-usage difference from today's single-session model**: N simultaneous logged-in sessions
  means N open SQLite connections to N vault files at once (vs. always exactly one today) — more file
  handles, more memory. Expected to be negligible for a realistic small household-scale N, but a real,
  named difference from every other front-end this codebase has shipped so far.
- **In-memory-only session store**: a `pwmgr-web` process crash or restart loses every active session at
  once — not a new property (the single-session desktop app has always had this), but now potentially
  affecting multiple concurrent users instead of one.
- **The clipboard auto-clear now has two independent implementations** (Rust for the Tauri apps, JS for
  web mode) — a future change to the 30-second value, or a bug in either implementation, needs to be
  caught in both places independently; the existing Rust tests provide zero coverage for the JS path.
- **`window.__TAURI__` feature-detection in `api.ts` becomes permanent, ongoing surface**: every future
  Tauri-only feature needs someone to consciously decide whether it also needs an HTTP-mode equivalent,
  and both paths need testing going forward — an ongoing maintenance tax, not a one-time cost of this
  milestone.

## 6. Testing

- **Mandatory/automated (Rust):** the `core` promotion of the remaining ~25 functions must leave the
  full existing test suite green (tests relocate with the code, unchanged, same pattern as M5's Task 9).
  New, genuinely novel test surface with no existing precedent to lean on: the session store (concurrent
  logins stay isolated from each other), the sweep-lock behavior generalized from one session to a map
  (reusing the existing `FakeClock` pattern from the autolock tests — needs confirming that pattern
  actually generalizes to the map-iteration version, not just assumed), the 24h inactivity prune, the
  auth middleware (missing/invalid/expired/locked token → 401), and the CSRF middleware
  (missing-header-on-mutating-request → 403). Plus a small HTTP-level integration test spinning up a
  real server and exercising it with real requests, mirroring this codebase's existing 2 end-to-end
  tests — this is the first HTTP-level test harness in the codebase, so building it is part of this
  milestone's cost, not free.
- **New, not-inherited-for-free coverage**: the JS-side clipboard auto-clear timer (browser-mode only)
  needs its own test, since it does not share any code path with the existing Rust-side coverage of the
  same property.
- **Lighter/manual (frontend/visual):** exercising the actual browser UI end-to-end (login, full command
  set, multi-tab/multi-session behavior) — deferred to the single consolidated manual walkthrough after
  M6 (both halves), per the standing 2026-07-14 decision (see `RESUME_HERE.md`).

## 7. Open items / deferred

- **Port number**: not yet pinned down to a specific value in this spec — a reasonable fixed default
  (not user-configurable for v1, matching the "not a config option" bind-address philosophy) should be
  chosen during implementation planning, checked against likely conflicts with the existing Vite dev
  server's port 1420.
- **Login POST reuse-vs-new-token semantics**: when a login POST arrives from a browser that already
  holds a valid cookie for the *same* user, whether it reuses that existing session-store entry or
  creates a fresh one is an implementation detail for the plan to pin down, not decided here — either
  choice is consistent with this design.
- Responsive layout (the other half of M6 per the roadmap) — separate future spec, not started.
- Unrelated, still pending: the M4 follow-up question about whether the Health Dashboard's "Weak"
  classification should respect the Settings strength-algorithm toggle instead of always using zxcvbn —
  still queued for after M6 is fully complete (both halves), tracked in the memory system
  (`project_m4_weak_scoring_followup.md`), not part of this spec.
