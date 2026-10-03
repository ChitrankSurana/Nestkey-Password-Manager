# How to Test — Web Portal

> **Status note (2026-09-30):** Written for the Classic UI before the New/Make UIs and Phase 9. For current UAT use `docs/testing/UAT_MODERN_UI_OVERHAUL.md` and `docs/testing/UAT_USER_TESTING_GUIDE.md`.

**Scope:** the localhost-bound HTTP server (`nestkey-web`, crate `web-src-tauri`), which is **not a
Tauri app at all** — it's a plain Axum binary that serves the same built React bundle as the Main
App and exposes the full command set (identical to `src-tauri`'s) over `POST /api/*` routes, gated
by a session cookie + CSRF header instead of Tauri IPC.

> **Heads up:** actually launching this server and browsing it end-to-end was explicitly deferred
> through all of M6 (see `docs/KNOWN_LIMITATIONS.md` §1) — this is the first written procedure for
> it. If a step doesn't match, that's a real, useful finding.

---

## Part 1 — Automated tests

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;C:\Strawberry\perl\bin;C:\Strawberry\c\bin;" + $env:PATH
cargo test -p nestkey-web
```

**Expected:** all pass. This includes unit tests for `session_store`, `error_map`, and `middleware`
(CSRF header + auth-cookie checks), plus the 3 tests in `web-src-tauri/tests/http_integration.rs`:
`register_then_list_sites_roundtrips_over_http`,
`a_mutating_request_without_the_csrf_header_is_rejected`, and
`an_authed_route_without_a_session_cookie_is_unauthorized`.

You can also run `cargo test --workspace` to confirm nothing else regressed at the same time.

---

## Part 2 — Launching the Web Portal

The server serves static files from `./dist` **relative to wherever you launch it from** — so build
the frontend first, and launch from the **repo root**, not from `web-src-tauri/`.

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;C:\Strawberry\perl\bin;C:\Strawberry\c\bin;" + $env:PATH
npm run build              # produces ./dist (index.html, viewer.html, assets)
cargo run -p nestkey-web      # from the repo root
```

**Expected console output:**
```
nestkey-web listening on http://127.0.0.1:7373 (serving ./dist)
```

Open **http://127.0.0.1:7373** in your browser (Chrome/Edge/Firefox — any modern browser). You
should see the same Login screen as the Main App, served as a normal web page.

**To stop the server:** Ctrl+C in the terminal it's running in.

---

## Part 3 — Manual walkthrough (functional)

**Prerequisite:** the Web Portal reads the same `~/.pwmgr/registry.db` and vault files as the Main
App and Viewer — you can log in with the same test user from
[`HOW_TO_TEST_MAIN_APP.md`](HOW_TO_TEST_MAIN_APP.md), or register a fresh one directly in the
browser.

Functionally, the Web Portal exposes the **entire** Main App command set (every route in
`build_router`'s `protected` group matches a `src-tauri` command 1:1) — so every scenario in
`HOW_TO_TEST_MAIN_APP.md` §1–13 applies here too, just running in a browser tab instead of a Tauri
window. Rather than duplicate all 13 sections, run through that entire checklist once against
`http://127.0.0.1:7373`, with one deliberate exception:

- **No clipboard command exists over HTTP.** The frontend's `api.ts` calls the browser's own
  `navigator.clipboard` API in web mode instead of a `copyToClipboard` Tauri/HTTP command — confirm
  **Copy** buttons (Generator, Credential editor) still work, but understand they're using a
  different code path than the desktop apps for this one feature.

The sections below cover what's **specific to the Web Portal** and not already covered by the Main
App checklist.

### 1. Session cookie behavior

1. **Action:** Log in, then open your browser's DevTools → Application/Storage → Cookies for
   `127.0.0.1:7373`.
   **Expected:** A `session` cookie is present, flagged `HttpOnly` (not readable from
   `document.cookie` in the console — try `document.cookie` in the DevTools console and confirm the
   session cookie does not appear in the output) and `SameSite=Strict`.
2. **Action:** Open a **second tab** to the same URL.
   **Expected:** Already logged in (same cookie, same browser) — but this is still a single session
   underneath; see §3 for true multi-session behavior.

### 2. CSRF header enforcement

This requires a manual HTTP request outside the normal UI — use your browser's DevTools console
while logged in (so the session cookie is attached automatically) or a tool like `curl`/Postman.

1. **Action:** From the DevTools console on `http://127.0.0.1:7373`, run:
   ```js
   fetch("/api/lock", { method: "POST" }).then(r => console.log(r.status))
   ```
   **Expected:** `403` — rejected for missing the `x-pwmgr-csrf` header, even though the session
   cookie was sent automatically by the browser.
2. **Action:** Now run:
   ```js
   fetch("/api/touch", { method: "POST", headers: { "x-pwmgr-csrf": "1" } }).then(r => console.log(r.status))
   ```
   **Expected:** `200` — the header alone (any value) satisfies the check.
3. **Action:** Try a plain `GET` without the header:
   ```js
   fetch("/api/list_sites", { method: "POST", headers: { "x-pwmgr-csrf": "1" } }).then(r => r.json()).then(console.log)
   ```
   **Expected:** Works — this confirms the mutating-route pattern; note `list_sites` is a `POST`
   route in this app (all API routes are `POST`, see `handlers.rs`), so it also needs the header.

### 3. Multi-session isolation (two independent logins)

1. **Action:** Log in as your test user in your normal browser window. In a **separate private/
   incognito window** (a genuinely separate cookie jar), register or log in as a **different**
   username.
   **Expected:** Each window shows only its own user's vault data — sites/credentials never leak
   across the two sessions.
2. **Action:** In one window, click **Lock**. Check the other window.
   **Expected:** Only the locked window's session is affected; the other stays logged in and
   functional (each session is independent server-side state, per `docs/design/THREAT_MODEL.md` §5).

> Note: an automated version of this exact scenario (two real cookie jars over HTTP) is not yet
> covered by `cargo test -p nestkey-web` — see `docs/KNOWN_LIMITATIONS.md` §1. This manual check is
> currently the only verification of it.

### 4. Idle sweep and auto-lock over HTTP

1. **Action:** Log in, set Auto-lock to 1 minute in Settings, then leave the browser tab idle (no
   clicks, no requests) for over a minute.
   **Expected:** The next action you take (e.g. clicking into the Vault) gets a `401 {code:"locked"}`
   response — there's no `vault-locked` browser event like the desktop apps have, so the UI doesn't
   auto-redirect on its own; you'll need to reload or navigate to see the Login screen again. (This
   UI gap is tracked, not a new bug — see the M6 web-mode plan's note on this.)
2. **Background detail, not separately actionable:** the server's idle sweep runs every 2 seconds
   (visible only via source: `web-src-tauri/src/main.rs`), matching the desktop app's autolock tick
   cadence — you don't need to do anything to trigger it, it's always running while the server is up.

### 5. Network exposure — confirm it's actually localhost-only

1. **Action:** While the server is running, check what it's actually bound to:
   ```powershell
   netstat -ano | findstr :7373
   ```
   **Expected:** The listening address is `127.0.0.1:7373`, **not** `0.0.0.0:7373` — confirming
   THREAT_MODEL A4's mitigation (no LAN exposure) is actually in effect, not just documented.
2. **Optional, if you have a second device on the same network:** try browsing to
   `http://<this-machine's-LAN-IP>:7373` from that other device.
   **Expected:** Connection refused/times out — the server genuinely never accepts non-loopback
   connections.

### 6. Server crash / restart behavior

1. **Action:** With an active logged-in session, stop the server (Ctrl+C) and start it again
   (`cargo run -p nestkey-web`).
   **Expected:** Your browser tab's next request gets a `401` — the in-memory session store doesn't
   survive a restart by design (see `docs/design/THREAT_MODEL.md` §5). Log in again to continue; your vault data
   itself is untouched (it's durable on disk, only the "who's logged in" state was lost).

---

## If something doesn't match

Note exactly: which step, what request/action, what you expected, what actually happened (status
code, response body, or screenshot). Since this is the first real execution of this app's launch and
walkthrough, also flag anything about Part 2 (build/launch) that needed adjusting.

## If everything matches

All three apps' manual walkthroughs are complete. This closes out the "manual UI walkthrough stays
deferred until both halves of M6 are complete" item — except the responsive-layout half, which still
has zero implementation code (see `docs/KNOWN_LIMITATIONS.md` §4) and will need its own pass once
built.
