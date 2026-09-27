# UAT Remediation Plan — `feature/modern-ui-overhaul`

Source findings: [docs/testing/UAT_FINDINGS_2026-09-23.md](docs/testing/UAT_FINDINGS_2026-09-23.md)

## Context

Two independent UAT passes (Claude's and the user's) were run against
`feature/modern-ui-overhaul` across desktop (D) and web (W) modes. The raw
findings file holds ~43 distinct defects and open design questions at mixed
severities, several already retracted or re-rooted mid-testing, five where the
two runs contradict each other, and several collapsing to one shared root cause.
It is a record, not a work list.

This plan is the joint review output. Every finding has a ruling, duplicates are
collapsed to their root cause, and work is ordered so shared roots and unknowns
land before the symptoms that depend on them. Intended outcome: the branch
becomes fully retestable, then every correctness and security defect is closed,
then polish.

**First action on execution:** copy this plan to `docs/Plans/` in the repo (plan
mode forces the `~/.claude/plans` path; the project's own convention is
`docs/Plans/`).

### Standing decisions

- **Scope:** everything, in wave order — Blockers, then Majors, then Minors/UX.
- **Platform:** desktop (Tauri) only. Web mode is tested and fixed in a later
  pass, so W-only findings are **deferred, not closed**.
- **Reveal policy:** every secret is gated by the master password. A Settings
  toggle chooses *grace window* vs *prompt on every reveal*; a separate
  per-item toggle overrides for one credential. The old `perCredential` mode,
  where an unflagged secret revealed with no prompt at all, is retired.
- **No network:** all new validation (card brand/number) must be fully offline —
  local IIN prefix table and Luhn, no lookups. HIBP stays the only network call
  in the app.
- **Tests:** one test per behavioral fix that has real logic (base32 padding,
  Luhn, search exclusion, grace expiry, folder blur semantics). No tests for
  pure layout or copy changes. A fresh UAT script for the fixed cases is written
  separately after the code lands.

## Wave 0 — Investigate before writing code

Nothing here is a fix; each unknown changes what gets built.

1. **Vault data loss (promoted to Blocker).** User reports vault contents gone
   after closing the `npm run tauri:dev` terminal or restarting the machine.
   Determine whether dev mode resolves a different data directory than the
   installed build, or whether persistence itself is broken. Everything else is
   moot if vault writes are not durable.
2. **Blank-window launch (promoted to Blocker).** App occasionally launches to a
   blank window (`npm run tauri:dev`, and hit once during M1.3). Find the root
   cause — a WebView2 asset-load race is the leading suspect given the embedded
   -assets design.
3. **Retest R1–R5 live** — the five disagreements between the two D runs.
   Settle them before coding, because they change what needs fixing:
   R1 "No folders yet" placeholder present or not (M4.2) · R2 Escape-cancels
   rename (M4.5 step 1) · R3 duplicate rename error shown or silent (M4.5
   step 3) · R4 password history 5 entries or 1 (M7.1) · R5 Chromium CSV
   fixture validity (M10.3).
4. **Settings persistence race (N15).** Diagnose before fixing: fire-and-forget
   write, debounce, or a stale config read clobbering on navigation.
5. **B8 timing audit.** Measure whether the nonexistent-username login path
   returns measurably faster than the wrong-password path.
6. **Raw-spawn audit (part of B4).** Grep the Tauri layer for other
   `tokio::spawn` / blocking calls on the webview thread with the same hazard as
   `ssh_agent_service.rs:138`.

## Wave 1 — Blockers

**B1 — Make UI shows "Vault is locked" with an empty/stale item list.**
Root cause: `useMakeVault()` is mounted at `src/makeui/MakeApp.tsx:41`, above the
`screen === "login"` early return (`:105-118`). Its mount effect
(`src/makeui/hooks/useMakeVault.ts:85-88`, stable deps so it never re-runs) fires
`list_sites` while the vault is still locked; `core/src/error.rs:122` supplies the
banner string; nothing refetches on login success (`MakeApp.tsx:109-112`).
`refreshAfterSave` (`MakeApp.tsx:79-83`) is why an Add Item sometimes cleared it.
Classic (`src/App.tsx:58` → `src/screens/Vault.tsx:48`) and newui
(`src/newui/NewShell.tsx:39`) mount the same hook *inside* the vault shell, hence
are unaffected.
Fix (Option A): effect in `MakeApp` calling `refreshSites`/`refreshFolders` when
`screen === "vault"`, and extend the `vault-locked` listener at
`useMakeVault.ts:103-111` to clear `sites` and `error` as well as `credentials`.
Closes M1.1, M2.3, M6.4-postreboot-lockout and the whole M10 stale-list cluster
(M10.1/M10.2/M10.3 were already retracted as knock-ons).

**B3 — Credential editor's Save/Cancel unreachable.**
`src/screens/CredentialEditor.tsx:79` has no `overflow-y-auto`, `max-h-*` or
`h-full`, centered in a viewport-height backdrop (`:78`), so the footer at
`:199-214` overflows off-screen. `ItemDrawer.tsx:163` already scrolls, so only
this modal is affected. Fix: add a `max-h` + `overflow-y-auto` scroll container;
mask the password field behind the standard master-password gate (N22); make
`PasswordGeneratorPanel` (`:101`) collapsible. Closes M6.1-modaloverflow and
M3.5.

**B4 — SSH agent enable crash-loops the app.**
`src-tauri/src/ssh_agent_service.rs:138` calls raw `tokio::spawn` off-runtime,
panicking on the native/webview thread; the panic unwinding across the FFI
boundary is what produced the `0xc0000409` fastfail, and the persisted
`ssh_agent_enabled: true` made it repeat every launch. Fix: use
`tauri::async_runtime::spawn` and change the handle type to
`tauri::async_runtime::JoinHandle<()>` (verified to compile during
investigation); **add a startup guard** so a failure in this boot path surfaces
as disabled-with-error rather than taking the process down; complete the
raw-spawn audit from Wave 0; and close only after **live pipe-up
verification** — the named pipe binds and serves a key, not merely that the app
launches. Closes M9.0 and D2.

**B5 — Windows Hello button hidden until the password field is non-empty.**
Fix: gate the button on a username being entered *and* biometrics enrolled for
that user, plus a Settings toggle to opt into remembering the last-used username
(which then shows the button immediately). Closes D1.

**B6 — Master-password change form.**
Reject a New/Confirm mismatch in the form before submit; force a logout to the
Login screen on success; always render an explicit, visible result for both
success and failure. Same-as-old stays permitted but must report a real result
rather than silently claiming success. Closes M9.1, M9.2, M9.3, M9.5.

**B7 — Reprompt / grace-period semantics.**
`src/components/SecretField.tsx:45-57` runs the gate without checking
`revealed`, so *hiding* prompts too; Cancel (`:132-143`) never re-masks; nothing
watches grace expiry; `src/lib/reprompt.ts:8-9` holds a single module-scoped
timestamp with no subscriber mechanism.
Fix: skip the gate when the action is a hide; Cancel re-masks; add a single
expiry timer + subscriber in `reprompt.ts` that re-masks every revealed field
together when the grace window ends. Retire the `perCredential` /`global` modes
at `core/src/config.rs:13` in favour of a grace-on/off toggle, migrating existing
configs to the grace-window setting. Also un-hardcode `reprompt: false` at
`src/makeui/components/ItemDrawer.tsx:120,132,134` so Secure Notes and cards are
gated (N3), and render the PROTECTED banner for non-login item types
(`DetailPanel.tsx:250`). Closes M6.1, M6.2, N3, and the reveal-policy user notes.

**B8 — Login failure wording.** Reword to a credential-agnostic message; fix any
timing side channel found in Wave 0. Closes M2.2.

**N20 / N21** — fixes follow from the Wave 0 investigations.

*Deferred to the web pass:* **B2** (lock does not tear down the SPA — only the
web-served `viewer.html` and web Make UI are affected; the desktop shells do
listen for `vault-locked`. The real gap is that `SESSION_LOCKED_EVENT`
(`src/lib/api.ts:198-202`) has exactly one listener, `src/App.tsx:47-56`).

## Wave 2 — Majors

**J1 — Boolean custom field.** `ItemDrawer.tsx:254-269` has no boolean branch, so
a Boolean field still renders a text box and `Yes` is stored literally while
readers compare strictly against `"true"` (`DetailField.tsx:28`). Port the
working checkbox branch from `CredentialEditor.tsx:130-136`.

**J2 — TOTP.** Four parts: strip `=` padding (plus tabs/newlines) in
`core/src/totp.rs:16-21`, which today uses `BASE32_NOPAD` and so rejects secrets
pasted from real websites; persist the normalized secret at
`core/src/commands.rs:449` instead of the raw string; validate the secret in the
frontend before a save begins; and make the six-command add sequence at
`ItemDrawer.tsx:105-121` atomic (single command/transaction, or rollback) so a
late failure cannot leave an orphan site and a retry cannot create a duplicate —
that non-atomicity is the "two items appeared" report.

**J3 — Password history.** Backend is correct and tested (cap-5 at
`core/src/vault.rs:670-675`, `:1445`). `CredentialBlock.tsx:36-48` fetches
history once and never invalidates, and `DetailPanel.tsx:122-129` renders the
block with no `key`, so cached history survives a save — that is both "only 1
entry" and "took until M10 to appear". Fix the cache/invalidation, then move
history into its own tab in the credential detail.

**J4 + N10 — Search.** Exclude **all** hidden custom-field values from the index
at `core/src/vault.rs:1038-1044`, and separately index the non-secret
`card_last4` column (`core/src/vault.rs:117`) so the last four digits are
searchable — no other part of a card number is. Closes M5.1 and M5.2.

**J5 / J6 — Folders.** `src/makeui/components/Sidebar.tsx:85-99` (rename) and
`:251-265` (create) both wire `onBlur` straight to the commit handler with no
suppression, so Escape's unmount-blur commits what it was meant to cancel and
Enter commits twice. Fix: **Enter commits; blur and Escape cancel** — drop
commit-on-blur. Move the duplicate-name error inside the input's conditional
block (currently rendered at `:267` and `:123`, outside it, and only cleared at
the top of the next commit attempt) so it clears on dismiss. Closes M4.3, M4.4,
M4.5.

**M8.1 — Credential delete has no confirmation** (`CredentialBlock.tsx:50-58`).
Route it through `src/lib/confirm.ts` like site, folder and Empty-Trash delete,
naming the credential.

*Deferred to the web pass:* **J7** (`window.confirm` at `src/lib/confirm.ts:15`
freezes the renderer in web mode; the desktop branch uses the Tauri dialog by
design, and there is no custom confirm component in `src/` to switch to).

## Wave 3 — Minors / UX

- **N1** — expose a vault-existence probe command wrapping
  `core/src/registry.rs:132 list_usernames` (zero callers today, not surfaced as
  a command) so a fresh vault opens Register instead of the hardcoded
  `"login"` (`MakeApp.tsx:32` and the three sibling shells); replace the
  `title=` tooltip at `src/screens/Register.tsx:56` with visible no-recovery
  warning copy; show the same caution in the Settings change-password flow.
- **N4/N5/N6** — cards: MM/YY auto-format on expiry (`ItemDrawer.tsx:334`,
  currently a bare input), digit/length/Luhn validation on the card number
  (`:330`, currently accepts `41111`), derive `cardLast4` from the number rather
  than the separate hand-typed field (`:320-325`), and derive the brand from a
  **local** IIN prefix table — with the derived brand editable. No network.
- **N7** — Notes input at `ItemDrawer.tsx:350-355` is gated on `mode === "add"`,
  so edit mode never shows it while `notes` is still submitted at `:123`.
  Un-gate it.
- **N2** animate the login spinner · **N17** replace the literal `"tauri-app"`
  window title at `src-tauri/src/lib.rs:54`.
- **N8** randomize mask length instead of the fixed `"••••••••"` at
  `SecretField.tsx:95` · **N9** stop non-secret copies (username
  `CredentialBlock.tsx:80`, URL `DetailPanel.tsx:265`, text custom fields
  `DetailField.tsx:32`) from auto-clearing the clipboard and from claiming a 30s
  clear, keeping all copies routed through `api.copyToClipboard` as
  `src/newui/lib/parity.test.ts:65-67` requires.
- **N18** collapsible nav pane and folder pane with remembered state ·
  **N19** Copy button in the password generator.
- **N12** let the plaintext-export confirmation field accept a paste ·
  **N23** drop the extra `note` column from the `%TEMP%\uat\chrome.csv` fixture.
- **N13** make the CSV gap loud in the UI now (what is dropped, use Backup for
  fidelity); full-fidelity JSON export+import is filed as its own work.
- **N15** fix per the Wave 0 diagnosis.
- **N16** fix the Viewer launch — `HOW_TO_TEST_VIEWER_APP.md` /
  `viewer-src-tauri/tauri.conf.json`'s `beforeDevCommand` resolves
  `--prefix ..` against the repo root, landing on `Account_1` instead of
  `Password_Manager` — **and** port the Viewer to the new UI.

## Wave 4 — Filed follow-ups (not this pass)

- **MakeShell restructure** (B1 Option B): extract the vault-screen tree from
  `MakeApp` into a shell mounted only when `screen === "vault"`, with
  `useMakeVault()` inside it, matching `newui`/Classic. Do this only once all
  fixes are landed and verified.
- **Full-fidelity JSON export + import** (from N13).
- **Per-credential change log** (from M7.2): a general audit trail — username
  changed, field added — separate from password history, which correctly stays
  password-only (`core/src/vault.rs:656`).
- **Web-mode pass:** B2, J7, N11 (off-screen error at `y:-152`), N14 (TOTP
  countdown not ticking), plus a full W re-run.

## Verification

1. **Wave 0 first:** R1–R5 retested live and recorded in the findings file; the
   vault-persistence and blank-window investigations written up with root causes
   before any Wave 1 code.
2. **Per fix:** `cargo test --workspace` (baseline 420 passing) and `npm test`
   (baseline 7 files / 40 tests) stay green, plus the new logic tests — base32
   padding, Luhn, hidden-field search exclusion, grace expiry re-mask, folder
   Enter/blur/Escape semantics.
3. **`npm run build`** clean, with `dist/` still producing index.html,
   viewer.html, newui.html, makeui.html.
4. **B4 specifically:** enable the SSH agent, confirm the named pipe binds and
   serves a key, then relaunch to confirm no crash-loop; separately confirm a
   deliberately bad persisted flag is handled by the startup guard rather than
   killing the process.
5. **Per-wave live re-run** of only the UAT cases each fix touches, on desktop.
6. **A fresh UAT script** covering the fixed cases is authored after the code
   lands, in the same format as `UAT_MODERN_UI_OVERHAUL.md`, for the major
   re-test pass.

## Progress (2026-09-27, implementation session)

Landed, tested (`cargo test --workspace` 421/421, `npm test` 40/40,
`npm run build` clean), and committed on `feature/modern-ui-overhaul`:

- Raw-spawn audit (Wave 0.6) — closed, no other hazard found.
- B8 timing audit (Wave 0.5) — closed; decoy KDF derivation already exists
  in `core/src/auth.rs`, a smaller residual gap is documented above, not
  reworked.
- **B1** — fixed (`bb4bfdd`).
- **B3 / N22** — fixed (`7dab2c5`).
- **B4** — fixed (`bb4bfdd`); still needs the **live pipe-up verification**
  step from the Verification section before D2/M9.0 can be marked closed.
- **B8** — fixed (`bb4bfdd`).
- **J1** — fixed (`bb4bfdd`).
- **J4 + N10** — fixed (`bb4bfdd`).
- **J5 / J6** — fixed (`bb4bfdd`).
- **N7** — fixed (`bb4bfdd`).
- **M8.1** — fixed (`bb4bfdd`).

- **B5** — fixed (`ef44145`): debounced check on username change (was
  blur-only) + opt-in "remember last username" toggle.
- **B6** — fixed (`1b47756`): local error next to the form (was
  page-level, could scroll out of view), forced logout on success.
- **B7** — fixed (`900512d`): every credential now gated by default; the
  per-credential flag is a new `reprompt_exempt` column (opt-out, not
  opt-in) so no existing credential's protection is silently flipped by
  reinterpreting the old column; hide no longer re-prompts; Cancel
  re-masks; a shared expiry timer auto-re-masks every revealed field when
  grace ends; config modes renamed grace/always with a load-time
  migration from the retired spellings; PROTECTED banner now covers every
  item type.
- **N2** — fixed (`900512d`): login spinner's `<i>` icon forced
  `inline-block` — CSS transforms are a no-op on inline elements, so
  `animate-spin` was silently doing nothing.
- **N17** — fixed (`900512d`): window title "tauri-app" → "LocalVault".

Still open (not started this session):

- **Wave 0** — R1 (folder placeholder) is very likely explained by
  `foldersOpen` defaulting to `false` in `Sidebar.tsx`, so nothing shows
  under a collapsed Folders section on a fresh vault, whether or not it's
  a bug; R2–R5, the vault-data-loss investigation, the blank-window
  investigation, and the N15 settings-race diagnosis are all still live
  retests/investigations, not yet done.
- **B2** — deferred to the web pass per standing decision.
- **J2, J3** — not started.
- **N1, N4–N6, N8/N9, N12/N13/N15/N16/N18/N19, N23** — not started.
- **Wave 4 follow-ups** — not started (expected; filed for later).
