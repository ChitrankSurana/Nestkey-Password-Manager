# Fix Wave 0 live-testing findings

## Context

A 2026-09-29 live UAT session (computer-use driving `npm run tauri:dev`)
retested Wave 0's open blocker investigations. Results are written up in
[docs/Plans/UAT_REMEDIATION_PLAN_2026-09-27.md](../../../../../E:/Coding_Projects-CS/Account_1/Password_Manager/docs/Plans/UAT_REMEDIATION_PLAN_2026-09-27.md)
under "Wave 0 live retests (2026-09-29 session)". This plan turns the
concrete, fixable findings from that session into code changes.

## All findings from the session (for the record)

| Finding | Verdict | Action |
|---|---|---|
| R1 (folders "No folders yet" placeholder) | PASS, settled | none — collapsed-by-default explains the original disagreement |
| R2 (Escape cancels folder rename) | Reported FAIL live, but **current source already implements the correct fix** (see below) | verify-only, no code change |
| R3 (duplicate-folder-name error shown) | PASS, settled | none |
| R4 (password history 5 vs 1 entries) | not reached this session | none — already unit-tested (`credential_history_caps_at_five_most_recent`) |
| R5 (Chromium CSV fixture) | not reached, needs the tester's own fixture file | none, out of scope for a code session |
| Vault data loss on terminal close | **not reproduced** — vault file byte-identical across 3 kill/relaunch cycles | none — explained by the single-instance finding below, not real storage loss |
| Blank-window launch | **CONFIRMED**, 3 of 4 launches | **fix below (two root causes)** |
| New: no single-instance guard | **CONFIRMED** — relaunch spawns duplicate processes | **fix below** |
| N15 (settings persistence race) | pre-existing, still inconclusive from an earlier session | out of scope — needs live reproduction, not a code plan |

Three things get fixed here: the single-instance guard, the WebView2
black-screen mitigation + instrumentation, and a verification pass on the
folder-input Escape/blur behavior (which turns out to need no code change).

## Bug 1 — Folder input Escape/blur: verify, don't fix

`src/makeui/components/Sidebar.tsx` already wires this correctly:
`cancelEdit`/`cancelAddFolder` (lines ~57-61, ~181-185) are bound to both
`onBlur` and the `Escape` branch of `onKeyDown` (lines ~103-107, ~294-298),
and neither ever calls `commitRename`/`commitAddFolder`. This exact contract
landed in commit `bb4bfdd` and the working tree matches HEAD with no local
diff to this file. `MakeVaultShell.tsx` (the parent) has no effect that
touches `editing`/`addingFolder` state and doesn't remount `Sidebar`.

Static reading finds **no plausible in-repo cause** for either symptom
(Escape no-op, or blur committing instead of reverting) — the live
behavior directly contradicts what this code does. Two things could
explain the mismatch instead of a real bug: (a) the live session may have
been driven against a stale build (pre-`bb4bfdd` bundle or an old orphaned
process — see Bug 2, which shows exactly this kind of process staleness
happening in the same session), or (b) computer-use's synthetic Escape
key-delivery not reaching this specific WebView2 build reliably.

**Action:** no code change. Once Bug 3's `devtools` gate is in place,
re-verify with real keyboard hardware and the F12 console open. Only
reopen as a code bug if it reproduces against a confirmed-current single
process with a real user's hands on the keyboard.

## Bug 2 — Add a single-instance guard

Root cause of both the blank-window flakiness and the original "data loss"
report: launching the app while an instance is already running starts a
**second independent process** instead of focusing the existing window
(watched `Get-Process pwmgr-app` go 1→2→3 across relaunches this session).

**`src-tauri/Cargo.toml`** — add to `[dependencies]`:
```toml
tauri-plugin-single-instance = "2"
```
No `deep-link` feature needed — this app has no deep-link handling.

**`src-tauri/src/lib.rs`** — register it **first** in the builder chain
(Tauri requires this plugin to run before others so it can intercept a
second-launch signal), before the existing `.plugin(tauri_plugin_clipboard_manager::init())`:
```rust
.plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}))
```
Window label stays `"main"` (fixed by the existing capabilities scoping
comment already in this file). No `capabilities/default.json` change
needed — this plugin has no window-scoped permission surface.

`viewer-src-tauri` has the same gap but is out of scope for this pass (per
your direction) — worth a follow-up note, not bundled here.

## Bug 3 — WebView2 stuck black screen

Reproduced with exactly one process running (rules out Bug 2 as the
cause): window chrome/title render fine, process responds, no panic, but
content never paints — persists through resize/maximize/minimize-restore.
This is a harder, stuck-compositor case, not just a brief init flash.

Combine a best-effort code mitigation with instrumentation so a
recurrence is actually diagnosable instead of another guess-and-check pass:

**`src-tauri/Cargo.toml`** — add the `devtools` feature to `tauri` (Tauri
strips this from release builds by its own convention; the *call site*
in code is additionally gated to debug builds):
```toml
tauri = { version = "2", features = ["devtools"] }
```

**`src-tauri/src/lib.rs`**, inside `.setup()` (currently lines ~24-58):
1. Before building the window, on Windows only, disable GPU compositing —
   the most evidence-backed fix for this exact WebView2/ANGLE black-screen
   class, set before WebView2 initializes:
   ```rust
   #[cfg(windows)]
   std::env::set_var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", "--disable-gpu");
   ```
   Acceptable tradeoff for a CRUD password-manager UI with no heavy
   rendering.
2. Bind the window builder's result instead of discarding it:
   ```rust
   let window = tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::App(entry.into()))
       .title("LocalVault")
       .inner_size(800.0, 600.0)
       .build()?;
   ```
3. Force a repaint nudge right after build (resize by 1px and back) as a
   cheap, safe mitigation for any remaining stuck-paint case the GPU flag
   doesn't cover.
4. Debug-only devtools access, so a future recurrence is diagnosable via
   F12 instead of blind guessing:
   ```rust
   #[cfg(debug_assertions)]
   window.open_devtools();
   ```
5. Lightweight startup logging (`eprintln!` or `window.on_window_event`)
   noting when the window is created/focused — enough to tell in a field
   report whether the window ever reached a ready state, without building
   an event bus for what's one native app.

No `tauri.conf.json` change — the window stays code-built per the
existing D-03 decision already documented in the file's own comment.

## Verification

1. **Build/tests stay green** — `cargo test --workspace` and `npm run
   build` after editing `lib.rs`/`Cargo.toml` (no logic changes, only
   setup-time config, so this should be a pure regression check).
2. **Bug 2** — `npm run tauri:dev` (or a packaged build), launch the OS
   entry point twice; confirm only one `pwmgr-app.exe` in Task Manager
   both times, and the second launch attempt focuses the existing window
   instead of opening a new one. Also confirm a normal single launch is
   unaffected.
3. **Bug 3** — repeated cold launches (aim for the same ~4-launch sample
   used in UAT) checking the black-screen rate drops to zero. If it still
   occurs, the debug-mode devtools console is now available to read the
   actual WebView2/ANGLE error instead of guessing further.
4. **Bug 1** — re-attempt live with real hardware once devtools access
   exists; only act further if it reproduces against a verified-current,
   single-process build.
