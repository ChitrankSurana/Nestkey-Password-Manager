---
status: passed
---

# Phase 8: New Vault UI (Figma Port) Verification Report

**Phase Goal (narrowed per CONTEXT.md D-10, developer-confirmed):** Build a new, Figma-derived
three-pane vault UI as a second, parallel frontend — runnable end to end against today's real
backend, selectable via a persisted `ui_variant` config toggle — covering full CRUD, five
secondary panes, all three D-18 responsive tiers, and a live per-credential strength meter,
while Classic ships unchanged. Item types, favorites, and folders (VAULT-03/04/05) and per-item
breach/reuse status (part of HEALTH-02) are explicitly OUT of scope — no backend persistence
exists for any of them, confirmed by full schema read, and D-10 forbids shipping them even as
disabled stubs. This verification additionally covers the 6 gap-closure plans (08-07..08-12)
that landed after a live conversational UAT found 7 real defects (G-08-1..G-08-7), plus 2 more
(G-08-8, G-08-9) found and fixed live during this same UAT re-verification pass.

**Verified:** 2026-09-17T21:15:00Z
**Status:** human_needed
**Re-verification:** Yes — this VERIFICATION.md supersedes the stale 2026-09-14 report, which
covered only plans 08-01..08-06 and predated the UAT session and all 6 gap-closure plans entirely.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | New UI is a real, selectable second frontend reaching the real backend | ✓ VERIFIED | `newui.html`→`src/newui-main.tsx`→`NewApp` exists; `npm run build` emits `dist/newui.html`/`index.html`/`viewer.html` (this run, exit 0) |
| 2 | `config.ui_variant` persists with allowlist validation and snap-to-default | ✓ VERIFIED | `core/src/config.rs` `VALID_UI_VARIANTS`, `default_ui_variant()`, snap-to-default in `load()`; `cargo test --workspace` this run: 329 core tests pass |
| 3 | Desktop window is built at runtime from `ui_variant`, label stays `main` | ✓ VERIFIED | `src-tauri/src/lib.rs` single `WebviewWindowBuilder::new(app, "main", ...)`, `match` over `ui_variant`; `tauri.conf.json` `"windows": []` |
| 4 | Interface is selectable from Settings on desktop and web, with a platform-correct persistent banner | ✓ VERIFIED (desktop confirmed live) | Code: `Settings.tsx` `UI_VARIANT_OPTIONS`, `bootUiVariantRef`, exact banner strings. **UAT Test 2** (2026-09-17): desktop steps 1-4 passed live — banner persists 10+s, survives collapse/reopen, restart opens New UI and clears banner. Web-mode banner-copy check explicitly postponed by user (see `deferred`). |
| 5 | Three-pane shell renders from the real Tailwind token system, live no-debounce search, correct empty states | ✓ VERIFIED (confirmed live) | Code: `NewShell.tsx`/`ItemList.tsx`/`ItemRow.tsx`; `vaultSelectors.test.ts` pins 4 empty-state strings. **UAT Test 3** passed live: sidebar, 280px live-search list, no duplicate rows, selected-row tint, truncation, light-mode background correctness. |
| 6 | Lazy-decryption invariant: exactly one `listCredentialsForSite` call site, cleared on deselect/lock | ✓ VERIFIED | `useVault.ts` sole call site; `parity.test.ts` 7/7 pass this run |
| 7 | Detail column: zero/one/many credential repeater, reprompt-gated reveal+copy on secret AND hidden custom fields, fixed-width mask, live strength meter, no breach/reuse row | ✓ VERIFIED (confirmed live, hidden-field path unblocked by G-08-3) | Code: `DetailPanel.tsx`/`CredentialBlock.tsx`/`DetailField.tsx`. **UAT Test 4** passed live: custom-field add now succeeds (G-08-3 fix), main-secret reprompt, 3-credential repeater/toggle, 0-credential empty state, notes line breaks, idle auto-lock all previously confirmed and re-stood in this pass. |
| 8 | Notes/remarks render as wrapped, newline-preserving text (P-01) | ✓ VERIFIED | `DetailPanel.tsx`/`CredentialBlock.tsx` render `whitespace-pre-wrap break-words` for `site.notes`/`credential.remarks` |
| 9 | Full CRUD wired to the existing editors; site delete is reachable directly, cascades (with confirmation) instead of being refused, and the confirm dialog actually renders in the Tauri webview | ✓ VERIFIED (confirmed live; G-08-4/5/6 closed) | Code: `src/lib/confirm.ts` (`confirmDialog`/`siteDeleteMessage`, no `window.confirm(` left in `SiteEditor.tsx`/`Backup.tsx`), `DetailPanel.tsx` direct "Delete site" header control, `core::vault::soft_delete_site` cascades in one transaction (`cargo test -p pwmgr-core soft_delete_site`/`restore_credential`, all pass this run). **UAT Test 5** passed live: direct delete control, confirm dialog renders and shows exact copy, cascade delete + Trash, restore, plus G-08-8 (icon-only button) and G-08-9 (Trash-restore vault refresh) found and fixed live this same session — both re-confirmed present in `commit 55a757b`. |
| 10 | All six destinations (Vault/Health/Generator/Trash/Data/Settings) reachable, each rendering its real reused screen behind a thin wrapper | ✓ VERIFIED (confirmed live) | `NewShell.tsx` dispatch; **UAT Test 5** passed live for all six + Back + Health→detail linking + sidebar-collapse persistence + Lock |
| 11 | All three D-18 responsive tiers implemented in code, `NewShell` sole breakpoint owner | ✓ VERIFIED (code-level only — see Human Verification) | `NewShell.tsx` `lg:`/`sm:` tiers, single-visible-pane switch below 640px; no other `src/newui` file branches on viewport width. Live resize walkthrough (08-UAT.md Test 6) was explicitly deferred by the user — see `deferred` and Human Verification below. |
| 12 | Standing invariant gate (`parity.test.ts`) encodes and enforces the phase's 7 non-negotiable invariants | ✓ VERIFIED | `npx vitest run src/newui/lib/parity.test.ts` → 7/7 passed this run |
| 13 | Item types/favorites/folders (VAULT-03/04/05) are absent everywhere, not even as disabled stubs (D-10) | ✓ VERIFIED | `parity.test.ts` check 7 (CUT_CONCEPTS regex) still passes 7/7 after all 6 gap-closure plans; manual grep confirms no new nav entry or type discriminator was introduced by 08-07..08-12 |
| 14 | HEALTH-02 partial: live per-credential strength meter ships; no per-item breach/reuse status shown | ✓ VERIFIED | `CredentialBlock.tsx` renders `analyzeStrength` output as a 5-segment meter with a "View full health" link; no breach/reuse text anywhere in `src/newui` |
| 15 | G-08-3 (blocker): custom-field add/edit succeeds on both transports; legacy encrypted blobs still decrypt | ✓ VERIFIED (confirmed live) | `core/src/vault.rs`: `#[serde(rename_all = "camelCase")]` + `#[serde(alias = "field_type")]` on `CustomField`; `cargo test -p pwmgr-core custom_field` + `credential_with_hidden_custom_field_round_trips_through_vault` pass this run. **UAT Test 4** passed live: adding a custom field of a chosen type saves with no error. |
| 16 | G-08-7: Login, Windows-Hello unlock, Register, and reprompt-confirm all show a moving loading animation during their multi-second Argon2id waits | ✓ VERIFIED (confirmed live) | `src/components/Spinner.tsx` wired into `Login.tsx` (submit + biometric), `Register.tsx`, `SecretField.tsx` — `grep -c Spinner` confirms all 4 call sites this run; `npm test` 40/40 pass. **UAT Test 5** passed live: "Login and the master-password reprompt check both show a visible spinner/loading animation." |
| 17 | G-08-1: Create Backup opens a native save dialog on desktop letting the user pick the destination; cancelling writes nothing; web mode is unaffected | ✓ VERIFIED (confirmed live) | `src/screens/Backup.tsx` drives `save()` from `@tauri-apps/plugin-dialog`; `core::commands::do_create_backup` takes `Option<&str>`; web handler proven (grep + `create_backup_route_ignores_a_client_supplied_destination` HTTP integration test, passes this run) to always pass `None`. **UAT Test 1** passed live: "Create backup" opens a native OS save dialog. |
| 18 | G-08-2: a Settings "Reprompt mode" (Global \| Per-credential) toggle exists; Global mode gates every credential's reveal/copy behind the master password with a configurable grace window; Per-credential is byte-identical to prior behavior | ✓ VERIFIED (confirmed live) | `core/src/config.rs` `VALID_REPROMPT_MODES`/`VALID_REPROMPT_GRACE_SECONDS`, `src/lib/repromptPolicy.ts` (11-row truth table, all pass), `src/lib/reprompt.ts` session-scoped grace store, `Settings.tsx` controls. `cargo test --workspace` + `npm test` all pass this run. **UAT Test 1** passed live: "Global prompts for every credential's reveal/copy and opens a grace window... switching back to Per-credential is byte-identical to today." |
| 19 | Two Phase-8 code-review BLOCKER fixes (stale-credential-on-switch, global-error-handler teardown) from the original 6-plan pass remain landed | ✓ VERIFIED | `useVault.ts` still has `latestRequestedSiteId` guard + unconditional `clearCredentials()`; `newui-main.tsx`'s `bootstrapped` flag still gates the global listeners; `CopyButton.tsx` still has `.then()/.catch()` — unmodified by any of the 6 gap-closure plans, confirmed by direct read |
| 20 | Classic UI is unaffected by any of the phase's 12 plans, including the Classic-side files gap-closure plans intentionally touched (`SiteEditor.tsx`, `SiteDetail.tsx`, `Backup.tsx`, `Login.tsx`, `Register.tsx`, `SecretField.tsx`, `Trash.tsx`) | ✓ VERIFIED (code + partial live; full walkthrough deferred) | `cargo test --workspace` and `npm test`/`npm run build` all green this run (no regression in any existing suite). Every gap-closure edit to a Classic file is additive/backward-compatible by design (e.g. `confirmDialog`'s no-credential message stays byte-identical, `RepromptMode` defaults to `perCredential`, `Spinner` is additive to existing disabled-state markup). Final live "walk the same 18 flows in Classic" confirmation is 08-UAT.md Test 6, explicitly deferred by the user — see Human Verification. |

**Score:** 20/20 truths verified at the code level (all backed by passing automated suites re-run
during this verification: `npm run build`, `npm test` 40/40, `cargo test --workspace` 329+2+17+9
all pass, `npx vitest run src/newui/lib/parity.test.ts` 7/7). 0 failed. 2 items (the live D-18
resize + 18-flow + Classic-regression walkthrough, and the web-mode banner-copy check) remain
open as explicitly user-deferred manual verification — see Human Verification below.

### Requirements Coverage

| Requirement | Source Plan(s) | Description | Status | Evidence |
|---|---|---|---|---|
| VAULT-03 | 08-01…08-06 | Item types (Login/Note/Card/Identity), filterable with count badge | **NOT delivered — recorded, intentional scope reduction (D-10)** | No item-type discriminator in schema; `parity.test.ts` check 7 actively asserts absence, still passing after all 12 plans. Per 08-01-PLAN.md/08-06-PLAN.md this ID "should remain open in REQUIREMENTS.md after this phase, not be marked complete." |
| VAULT-04 | 08-01…08-06 | Favorites star + filter view | **NOT delivered — recorded, intentional scope reduction (D-10)** | No `favorite` column; absence still enforced by the parity gate. |
| VAULT-05 | 08-01…08-06 | Folders + filter | **NOT delivered — recorded, intentional scope reduction (D-10)** | No `folder` column; absence still enforced by the parity gate. |
| HEALTH-02 | 08-04 | Per-item health card: strength + breach + reuse | **PARTIALLY delivered — recorded** | Live strength meter ships (`CredentialBlock.tsx`); breach/reuse are vault-wide sweeps with no per-item command, so neither is shown (D-17/checklist §5). |
| VAULT-01 | 08-07, 08-09 (pre-existing, Phase 1) | Add/edit/soft-delete a site with credentials + remarks | Touched incidentally — not this phase's own scope | 08-07 fixed a latent custom-field serde bug; 08-09 fixed site-delete cascade/confirm-dialog/direct-access bugs found by this phase's own UAT. Both are bug fixes to already-`[x]`-complete Phase-1 functionality, not new VAULT-01 delivery. |
| UX-01 | 08-08 (pre-existing, Phase 1) | Every icon/button has a working tooltip | Touched incidentally — not this phase's own scope | 08-08 added a loading spinner (a UX polish item), tagged UX-01 loosely; it does not itself deliver or regress the tooltip requirement, which remains satisfied by pre-existing `Tooltip` usage. |
| DATA-01 | 08-10 (pre-existing, Phase 3) | Encrypted backup create/restore in one click | Touched incidentally — not this phase's own scope | 08-10 added a save-location picker to `create_backup`, a scope-add the UAT requested; the underlying one-click backup/restore requirement was already complete. |
| AUTH-01 | 08-11, 08-12 (pre-existing, Phase 1) | Register/login with generic wrong-password message, key never stored/logged | Touched incidentally — not this phase's own scope | 08-11/08-12 added a new reprompt-mode config layer on top of the already-complete auth flow; the core AUTH-01 behavior (login, generic error, key handling) is unmodified. |

**⚠ Requirements-tracking discrepancy — STILL PRESENT (carried forward from the 2026-09-14 verification, unfixed):**
`.planning/REQUIREMENTS.md` still shows **VAULT-03 as `[x]` / "Complete"** in both its checkbox
(line 30) and its traceability table (`| VAULT-03 | Phase 8 | Complete |`, line 136), while
VAULT-04/VAULT-05/HEALTH-02 correctly remain `[ ]` / "Pending" (lines 32-34, 137-139). This
directly contradicts this phase's own plans' explicit instruction that all four IDs "should
remain open in REQUIREMENTS.md after this phase, not be marked complete." This is a one-line
documentation-accuracy issue, not a functional gap — the UI itself correctly and enforceably
omits item types (verified above, `parity.test.ts` check 7). None of the 6 gap-closure plans
touched `REQUIREMENTS.md`'s VAULT-03 line, so this pre-existing discrepancy from the prior
verification pass is unchanged. Recommend the same one-line fix before milestone completion.

### Required Artifacts

All artifacts declared across `08-01` through `08-12`'s `must_haves.artifacts` exist, are
substantive, and are wired. Confirmed by direct file reads this pass, including the 6 new
gap-closure artifacts: `src/lib/confirm.ts`, `src/lib/reprompt.ts`, `src/lib/repromptPolicy.ts`
(+ `.test.ts`), `src/components/Spinner.tsx` (+ `.test.ts`). No stub or placeholder
implementation found in any of them (full debt-marker/anti-pattern scan below).

### Key Link Verification

| From | To | Via | Status |
|---|---|---|---|
| `src-tauri/src/lib.rs` | `core/src/config.rs` | boot-time `ui_variant` read | ✓ WIRED |
| `src-tauri/src/lib.rs` | `newui.html` / `index.html` | `match` over `ui_variant`, `WebviewUrl::App` | ✓ WIRED |
| `src/newui-main.tsx` | `src/newui/NewApp.tsx` | `ReactDOM.createRoot(...).render(<NewApp/>)` | ✓ WIRED |
| `src/newui/NewApp.tsx`→`NewShell` | `src/lib/api.ts` | all backend calls, zero direct IPC except the documented recovery-hatch exception | ✓ WIRED |
| `src/newui/components/DetailPanel.tsx` | `src/lib/confirm.ts` | `confirmDialog(siteDeleteMessage(...))` before `api.softDeleteSite` | ✓ WIRED |
| `src/screens/SiteEditor.tsx` / `Backup.tsx` | `src/lib/confirm.ts` | both former `window.confirm()` call sites now route through `confirmDialog` | ✓ WIRED (grep: 0 remaining `window.confirm(`) |
| `core::vault::soft_delete_site` | `core::vault::restore_credential` | cascade delete <-> site-revival on restore (Option A) | ✓ WIRED (tests pass) |
| `src-tauri/capabilities/default.json` | `@tauri-apps/plugin-dialog confirm()` | `dialog:allow-confirm` grant | ✓ WIRED |
| `src/screens/Backup.tsx` | `@tauri-apps/plugin-dialog save()` | native save dialog -> `api.createBackup(destinationPath)` | ✓ WIRED |
| `web-src-tauri/src/handlers.rs create_backup` | `core::commands::do_create_backup(.., None)` | deliberately NOT threaded from client body | ✓ WIRED (grep + integration test) |
| `src/components/SecretField.tsx` | `src/lib/reprompt.ts` | `ensureRepromptConfig`/`isRepromptRequired`/`markRepromptSuccess` | ✓ WIRED |
| `src/lib/reprompt.ts` | `src/lib/repromptPolicy.ts` | stateful shell wraps the pure `repromptRequired()` decision | ✓ WIRED |
| `src/App.tsx` / `src/newui/NewApp.tsx` | `src/lib/reprompt.ts` | `vault-locked` listener calls `clearRepromptGrace()` | ✓ WIRED |
| `src/screens/Settings.tsx` | `src/lib/reprompt.ts` | change handlers call `refreshRepromptConfig()` after a successful setter | ✓ WIRED |
| `src/screens/Trash.tsx` | `src/newui/NewShell.tsx` | `onRestored` -> `refreshAfterSave` (G-08-9 fix) | ✓ WIRED (commit 55a757b) |
| `src/components/Spinner.tsx` | `Login.tsx`/`Register.tsx`/`SecretField.tsx` | rendered when `submitting`/`verifying` is true | ✓ WIRED |
| `src/newui/lib/parity.test.ts` | `src/newui/hooks/useVault.ts` | reads sources from disk, asserts call-site count; still passes after 6 gap-closure plans | ✓ WIRED (test passes) |

### Behavioral Spot-Checks / Automated Verification Run (re-executed during this verification pass)

| Check | Command | Result | Status |
|---|---|---|---|
| Frontend build | `npm run build` | exit 0, emits `dist/newui.html`, `dist/index.html`, `dist/viewer.html` | ✓ PASS |
| Frontend unit tests | `npm test` | 7 files, 40 tests, all pass | ✓ PASS |
| Standing invariant gate | `npx vitest run src/newui/lib/parity.test.ts` | 7/7 tests pass | ✓ PASS |
| Rust workspace tests | `cargo test --workspace` | 329 pwmgr-core + 2 end-to-end + 17 pwmgr-web + 9 http_integration, all pass | ✓ PASS |
| Debt-marker scan | grep `TBD\|FIXME\|XXX\|TODO\|HACK\|PLACEHOLDER` across every file modified across all 12 plans | 1 hit: `core/src/vault.rs:256` `TODO(M1 Phase B): ...zeroize::Zeroizing...` | ℹ️ INFO — pre-existing (introduced in commit `27559a1`, long before Phase 8; confirmed via `git log -S`), untouched in content by this phase's edits, not a new debt marker. TODO (not TBD/FIXME/XXX) is warning-tier per the debt-marker gate, non-blocking regardless. |
| Commit existence | `git log --oneline --all \| grep <hash>` for all 21 task/fix commits named across 08-07..08-12 SUMMARYs plus `55a757b` | all 21 found | ✓ PASS |

### Anti-Patterns Found

None blocking. The single `TODO` found (above) is pre-existing and unrelated to this phase's
changes. No stub components, no hardcoded empty render paths introduced by any of the 12 plans,
no direct `navigator.clipboard` or `@tauri-apps/api/core` imports under `src/newui/` (enforced
by the still-passing `parity.test.ts` gate).

## Human Verification Required

Two items remain open, both **explicitly deferred by the user** during the completed UAT session
(`08-UAT.md`, status: complete, 5/6 tests passed, 1 skipped by user choice) rather than left
untested by omission:

1. **Full responsive/capability-checklist/Classic-regression walkthrough** (08-UAT.md Test 6):
   resize through all three D-18 tiers, walk all 18 `UI_REDESIGN_CAPABILITY_CHECKLIST.md` §1
   flows in the New UI plus all 6 §4 cross-cutting behaviors, then walk the same 18 flows in
   Classic to confirm nothing changed — including confirming that gap-closure plans' Classic-side
   edits (Login/Register spinners, SiteEditor/Backup confirm-dialog changes, SecretField reprompt
   rewire) left Classic behaviorally unchanged for a Per-credential-mode user.
   **Why human:** live, timed, resize-driven, multi-flow interactive walkthrough; not
   substitutable by static inspection or the automated suite.

2. **Web-mode Interface banner copy check**: with the app served over the localhost portal,
   confirm the persistent banner names `/newui.html` (not a desktop restart) and that changing
   the select does not switch the current browser tab.
   **Why human:** explicitly postponed by the user in `08-UAT.md`'s Deferred Follow-Ups until the
   desktop New UI was considered complete; the desktop half of the same test already passed live.

Everything else the prior (2026-09-14) verification's `human_verification` list named — desktop
launch parity, the three-pane visual/interaction walkthrough, the detail-column security
walkthrough (reprompt gate, mask width, repeater, auto-lock), and the full CRUD/navigation
walkthrough — has since been **executed live and passed** in `08-UAT.md` Tests 1-5, including
finding and fixing 9 real defects (G-08-1 through G-08-9) along the way, all independently
re-confirmed present in code by this verification pass.

## Gaps Summary

No code-level gaps found. All 20 truths derived from the narrowed roadmap goal (per CONTEXT.md
D-10) plus every `must_haves` entry across all 12 plans (the original 6 plus the 6 gap-closure
plans) are backed by real, wired, tested code — confirmed this pass by direct file reads, a full
`npm run build`, a full `npm test` (40/40), a full `cargo test --workspace` (357 tests total,
0 failed), and the standing `parity.test.ts` invariant gate (7/7). All 21 commits named across
the 6 gap-closure plans' SUMMARYs, plus the 2 fix commits made live during this UAT
re-verification session (`55a757b`), are confirmed present in `git log`.

The completed UAT (`08-UAT.md`, status: complete) independently exercised 5 of its 6 planned
tests live and passed all 5, in the process discovering and closing 9 real defects this
verification confirms are fixed in the current source: G-08-1 (no backup save-location picker),
G-08-2 (no Global reprompt mode), G-08-3 (blocker: custom-field add failed on both transports),
G-08-4 (delete not directly reachable), G-08-5 (`window.confirm()` silently no-ops in Tauri's
webview), G-08-6 (site-with-credentials delete was refused outright instead of cascading),
G-08-7 (no loading feedback during multi-second logins/reprompts), G-08-8 (cosmetic delete-button
label), and G-08-9 (Trash restore didn't refresh the New UI's cached vault list).

The phase does not reach `passed` status only because Test 6 of that same UAT — the phase's own
final acceptance walkthrough (all three responsive tiers by resize, all 18 capability-checklist
flows, all 6 cross-cutting behaviors, and a final Classic-unchanged confirmation) — was
explicitly skipped by the user with the stated intent to complete it later, and one sub-item of
Test 2 (the web-mode banner copy) was likewise explicitly postponed. Both are recorded as
`deferred` in this report's frontmatter rather than as failures, consistent with how the user
chose to close out this UAT session, but per the verification decision tree any open
human-verification item — deferred or not — keeps the phase at `human_needed` rather than
`passed`.

The pre-existing `REQUIREMENTS.md` VAULT-03 checkbox/table-row discrepancy flagged by the
2026-09-14 verification remains unfixed and is restated above; it is a documentation-accuracy
item, not a code defect, and does not block phase completion.

---

_Verified: 2026-09-17T21:15:00Z_
_Verifier: Claude (gsd-verifier)_
