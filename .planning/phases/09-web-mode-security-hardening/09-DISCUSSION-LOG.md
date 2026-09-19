# Phase 09: Web Mode Security Hardening — Discussion Log

Audit trail of `/gsd-discuss-phase 09` Q&A, source: `09-DISCUSS-CHECKPOINT.json`.

**Discussed:** 2026-09-17/18 | **Mode:** interactive (no `--auto`) | **Areas:** 4/4 complete

---

## Path confinement (SEC-01)

| # | Question | Options Presented | Selected |
|---|----------|--------------------|----------|
| 1 | How should web-mode export/import/restore stop accepting arbitrary client paths? | Confine to app directory (Recommended) · Full upload/download redesign | **Full upload/download redesign** |
| 2 | Does the upload/download redesign apply only to web mode, or does desktop switch too? | Web mode only (Recommended) · Both transports unified | **Web mode only** |
| 3 | For CSV import specifically: how should the multi-step flow (headers/preview/commit) work with upload bodies? | Upload once, cache server-side (Recommended) · Client re-uploads at each step | **Upload once, cache server-side** |
| 4 | Where should the cached CSV upload live between headers/preview/commit, and how long does it persist? | In UserSession, cleared on commit/cancel/logout (Recommended) · Temp file on disk, session-scoped | **In UserSession, cleared on commit/cancel/logout** |
| 5 | Does `restore_backup` become a client upload too? | Yes — client uploads the backup file (Recommended) · No — restore stays path-based but confined | **Yes — client uploads the backup file** |

## Login throttling (SEC-02)

| # | Question | Options Presented | Selected |
|---|----------|--------------------|----------|
| 1 | Where should attempt/lockout state live? | New in-process store in web-src-tauri (Recommended) · Persist to registry.db | **New in-process store in web-src-tauri** |
| 2 | Should `/api/register_user` (open, pre-auth) also be throttled? | Yes, same throttle as login (Recommended) · No, login only | **Yes, same throttle as login** |
| 3 | What should the backoff/lockout shape look like? | Fixed threshold + timed lockout (Recommended) · Exponential backoff, no hard lockout | **Hybrid, user-authored (see D-08):** 5 failed attempts → lockout for fixed base window `W`. Each subsequent lockout triggered while still failing doubles `W` (window itself grows exponentially across cycles, not a per-attempt delay). |
| 4 | Key the throttle by username, by IP, or both? | Both, independently (Recommended) · Per-username only | **Both, independently — whichever limit hit first blocks the request** |
| 5 | When does the exponential lockout-window multiplier reset? | On next successful login (Recommended) · After a long cooldown of no attempts | **On next successful login** — resets both failure count and window multiplier |

## Argon2 concurrency cap (SEC-02)

| # | Question | Options Presented | Selected |
|---|----------|--------------------|----------|
| 1 | What should happen when the Argon2 concurrency limit is saturated? | Queue and wait (Recommended) · Reject with 429 immediately | **Reject with 429 immediately** (`try_acquire`, no queueing) |
| 2 | What should the permit count (max concurrent Argon2 derivations) be? | Small fixed number, e.g. 2-4 (Recommended) · Decide at planning/research time | **Small fixed number, 2-4** |
| 3 | Which handlers share the same Argon2 concurrency semaphore? | All three, one shared semaphore (Recommended) · Login/register only | **All three (login, register_user, export_csv password mode), one shared semaphore** |
| 4 | On a 429 rejection, how should the web frontend behave? | Generic retry-later error message (Recommended) · Automatic client-side retry with backoff | **Automatic client-side retry with backoff (1-2s), invisible unless repeatedly failing** |

## CSP & headers (SEC-03)

| # | Question | Options Presented | Selected |
|---|----------|--------------------|----------|
| 1 | How strict should the CSP be for the served SPA? | Strict default-src 'self' (Recommended) · Strict but with style-src 'unsafe-inline' | **Strict `default-src 'self'; script-src 'self'; style-src 'self'`**, no `'unsafe-inline'` — React `style={{}}` sets DOM properties directly, not parsed CSS, so it isn't blocked |
| 2 | Does the same CSP apply to both the web-mode server response header AND desktop's `tauri.conf.json`? | Same policy, both places (Recommended) · Different policies per surface | **Same policy, both places** |
| 3 | Should dev mode (Vite HMR) get a relaxed CSP? | Relax only in dev, strict in production build (Recommended) · Same strict policy always, fix dev separately if it breaks | **Relax only in dev** (`connect-src ws://localhost`, `script-src 'unsafe-eval'` for sourcemaps); strict applies to production build only |
| 4 | Should the same response-header layer also add `Referrer-Policy` and `X-Frame-Options` (named in SECURITY_CONCERNS.md M3 but not in SEC-03's locked success criteria)? | Yes, add both while touching this layer (Recommended) · No, only what SEC-03 requires | **Yes — add both**, even though not strictly required by SEC-03's locked wording |

---

*Full decision rationale and reversibility notes: `09-CONTEXT.md` §Implementation Decisions (D-01…D-18).*
