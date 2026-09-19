# Phase 09 UAT — Manual Process

Deferred check for SEC-03 (strict CSP). Do this once, then run `/gsd-verify-work 09` and report results.

## Prep
```
npm install
npm run build
cargo build -p pwmgr-web
```

## Test 1 — Web mode CSP sweep (browser)
1. Run the web binary: `cargo run -p pwmgr-web` (serves http://127.0.0.1:7373).
2. Open the URL. Open DevTools → Console (clear it).
3. Log in, then visit: Generator, Health, Settings, Backup, Export CSV, Import CSV.
4. Pass: no message containing "Content Security Policy" or "Refused to"; icons/fonts render; password strength meter bar animates; theme + accent applied.

## Test 2 — Response headers
1. DevTools → Network → click any response (page, an `/api/*` call, a static asset).
2. Pass: Response Headers include
   - `content-security-policy`
   - `x-content-type-options: nosniff`
   - `referrer-policy: no-referrer`
   - `x-frame-options: DENY`

## Test 3 — Web export / import / restore (SEC-01)
1. Export CSV: file downloads via the browser (no server-side path prompt).
2. Import CSV: choose file via picker, preview shows, commit succeeds.
3. Backup restore: choose backup file via picker, overwrite confirm appears, restore succeeds. (Note: restoring over your own vault may lock your session; re-login is expected.)
4. Pass: all three work with no console errors.

## Test 4 — Desktop shell under strict CSP
1. `npm run tauri:dev` — window loads, edit a source file, hot reload works, DevTools console shows no CSP violations.
2. `npm run tauri:build`, launch the built binary, click through every screen.
3. Pass: everything works, no CSP violations.

## If something breaks
- Production breakage (web or built binary): fix the policy in `web-src-tauri/src/middleware.rs` (`CSP_POLICY`) and mirror byte-for-byte in `src-tauri/tauri.conf.json`.
- Dev-only breakage: fix only `src-tauri/tauri.dev.conf.json`.
- Note the violating directive from the console message and report it in `/gsd-verify-work 09`.

## Afterward
- All pass → `/gsd-verify-work 09` (answer "pass"), then SEC-03 and phase 09 get marked complete.
- Known unrelated bug: registering a taken username returns 500 not 409 (see `deferred-items.md`).
