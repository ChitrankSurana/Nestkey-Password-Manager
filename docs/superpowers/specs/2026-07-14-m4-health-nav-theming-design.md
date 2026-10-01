# M4 Design — Health Dashboard, Navigation Rail, Theming, Generator Screen

**Status:** Approved (brainstormed interactively, including the visual companion for layout/color
questions), pending write-up as an implementation plan via `superpowers:writing-plans`.
**Last updated:** 2026-07-14

## 1. Purpose

Deliver M4 per `docs/design/PRD.md`'s milestone table: a health dashboard (weak/reused/old passwords +
opt-in HIBP breach check), a navigation rail, and full theming (light/dark × 5 accent colors). Also
folds in a standalone Generator screen — explicitly deferred from M2 to M4 at the time, and a natural
nav-rail destination once the rail exists.

## 2. Scope

**In scope:**
- `core::health` — pure weak/reused/old classification logic (no I/O).
- HIBP Pwned-Passwords k-anonymity check — opt-in, manually triggered, app-crate-only (see §4).
- Health Dashboard screen: 4 cards (Weak, Reused, Old, Breached), each expandable to an affected-
  credential list that links to Edit Credential.
- Fix: `vault::update_credential` must bump `password_updated_at` only when the secret actually
  changes (currently never bumps it at all — see §3).
- Navigation rail: Vault, Health, Generator, Trash, Data (hub for Backup/Export/Import CSV),
  Settings, plus Lock. Collapsible (icon+label ⇄ icon-only), toggle pinned at the top, flyout
  tooltip on hover when collapsed, accent-tinted highlight on the active item, collapsed/expanded
  state persisted in `config.json`.
- Standalone Generator screen: generate + copy only, no vault interaction (reuses the existing
  `generate_password`/`analyze_strength`/`copy_to_clipboard` commands, no new IPC).
- Theming: make the existing (currently inert) `config.theme` field actually drive `<html>`'s `dark`
  class; add a 5-value `accent_color` config field (Indigo/Sky/Teal/Purple/Pink — cool-toned palette,
  chosen to stay visually distinct from the red/green already used for error/success text) driving a
  `data-accent` attribute and CSS custom properties; mechanically swap the ~11 hardcoded
  `bg-blue-600`/`text-blue-400`/etc. occurrences across 9 screen files to reference the accent
  variable instead.
- New Settings controls: old-password-days threshold, HIBP opt-in toggle, theme picker, accent-color
  swatch picker — same screen, same idiom as the existing auto-lock/strength-algorithm controls.

**Out of scope (explicit, so it isn't accidentally built):**
- Automatic/background HIBP checking — opt-in only unlocks a manual "Check now" button; checking
  never happens without that click, even after opting in.
- Saving a password directly from the Generator screen to a new/existing site — copy only; saving
  still goes through the existing Add/Edit Credential flow.
- Per-destination icon coloring in the nav rail (the Dribbble reference's multi-color icons) — icons
  stay monochrome; only the active-item highlight uses the accent color, to keep "one consistent
  accent color everywhere" (PRD user story 10) actually true.
- A persisted reuse fingerprint — reuse detection decrypts and compares in memory on each report
  request, nothing new is stored (see §3 for why).
- Any HIBP endpoint beyond the Pwned-Passwords range (k-anonymity) API — no full breach-database
  lookups, no email-based checks.
- Responsive/mobile layout and localhost web mode — both explicitly M6.

## 3. Health Dashboard — design decisions

- **Weak:** reuses the strength scoring already built in M2 (`do_analyze_strength`, respecting the
  user's `strength_algorithm` setting). A credential is "weak" if its score is 0 or 1 ("Very weak" /
  "Weak"). No new scoring code.
- **Reused:** computed **on demand**, not via a stored fingerprint. The health report decrypts every
  credential's secret in memory (same trust boundary as `search`, already-unlocked territory),
  groups by exact plaintext match, and any group of 2+ is a reused group. Rejected the alternative
  (an HMAC fingerprint stored per credential) because it adds a new persisted derived-secret artifact
  that needs its own threat-modeling, plus a migration path for every M1-M3 credential that predates
  it — not worth it at this app's target scale (NFR-3: ~5,000 entries, decrypt-and-compare is fine).
- **Old:** a credential is "old" if `password_updated_at` is older than a configurable threshold
  (`old_password_days` in `config.json`, default 365, same validated-enum pattern as
  `auto_lock_minutes`). **Prerequisite fix:** `vault::update_credential` currently only updates the
  generic `updated_at` column, never `password_updated_at`, even when the secret itself changes — so
  "old" would be silently wrong today (a password changed yesterday would still show its original
  creation date). Fix: before writing, decrypt the *currently stored* secret and compare to the
  incoming one; only set `password_updated_at = datetime('now')` when they differ. Editing just the
  username or remarks must not reset the age clock.
- **Breached:** opt-in via a new `hibp_opt_in` config bool (default false — NFR-1 holds). Once
  enabled, the dashboard's Breached card shows a "Check now" button; nothing is sent over the network
  until that specific click, every time (not just once after opt-in) — the more conservative of the
  two options considered, matching this project's established "defense in depth at the trust
  boundary" pattern (plaintext CSV export's typed-confirmation + backend `confirmed: bool` gate is
  the precedent). The backend's `check_breach` command itself refuses to run unless
  `hibp_opt_in` is true, independent of what the frontend sends — same shape as the plaintext-export
  gate, not just a UI-level restriction.
- Before hitting HIBP, dedupe identical passwords (already grouped for reuse detection in the same
  report pass) so one HTTP request covers a group, not one per credential.

## 4. Architecture — module placement

Following the precedent M2 set with `zxcvbn` ("core stays dependency-light" — the zxcvbn-backed
`analyze_strength` lives in the app crate only, not `core`):

- **`core::health`** (new, `core/src/health.rs`): pure functions — weak/reused/old classification
  over already-decrypted data, and the k-anonymity match logic (given a SHA-1 hash and a list of
  `(suffix, count)` pairs from HIBP, determine whether the password is in the list). No I/O, no new
  crate dependency for `core` itself beyond what it can compute from bytes it's handed. Fully unit
  testable without a network or a real vault.
- **`src-tauri` app crate only**: the actual HIBP HTTP call and the SHA-1 hashing that feeds it.
  Two new dependencies, both app-crate-scoped:
  - `reqwest` — the app already runs an async runtime (`tauri::async_runtime::spawn` /
    `tokio::time`, used today for the clipboard auto-clear and autolock timers), so this reuses
    existing async machinery rather than introducing a second concurrency model (a blocking client
    would need `spawn_blocking` wrapping to avoid stalling the executor).
  - `sha1` — required specifically because HIBP's Pwned-Passwords protocol is defined in terms of a
    SHA-1 hash of the password (send the first 5 hex chars, get back matching suffixes+counts,
    compare the rest locally). Not a security design choice, a protocol-compatibility one — nothing
    else in this app uses or needs SHA-1.
  - The HTTP call's base URL should be injectable (constructor param or similar), not hardcoded, so
    Rust tests can point it at a local mock instead of the real HIBP endpoint — keeps the "mandatory,
    automated" testing bar from TRD §7 achievable for this command without live network access in CI.
  - This keeps `core` itself network-free, which matters beyond just this feature: `core` is reused
    as a library by M5's read-only viewer (per `ARCHITECTURE.md`), which has no business making
    network calls at all — enforcing "no network code in `core`" as an invariant is cheap insurance
    for that milestone, the same way "no serde codegen in `core`" was kept as an invariant through M3.

**New IPC commands** (`src-tauri/src/commands.rs`, following the established `do_*` + wrapper split):
- `health_report()` → weak/reused/old lists (no params — operates on the whole unlocked vault).
- `check_breach()` → breached list. Errors if `hibp_opt_in` is false, independent of frontend state.
- `update_old_password_days(days)`, `update_hibp_opt_in(enabled)`, `update_theme(theme)`,
  `update_accent_color(color)`, `update_nav_collapsed(collapsed)` — all mirror
  `update_auto_lock_minutes`'s existing shape (validate against a fixed enum, persist to
  `config.json`, return the saved value or void).

**`config.json` gains 4 fields:** `accent_color` (5 valid values), `old_password_days` (validated
enum, default 365), `hibp_opt_in` (bool, default false), `nav_collapsed` (bool, default false — rail
starts expanded). `theme` already exists as a field but the frontend has never read it; this
milestone is what finally wires it up.

## 5. Navigation Rail & Data Hub

- Destinations: **Vault** (home), **Health**, **Generator**, **Trash**, **Data** (hub), **Settings**
  — 6 icons — plus **Lock** pinned at the bottom, separate from the scrollable destination list (an
  action, not a screen).
- **Data hub**: a new small screen with 3 cards (Backup/Restore, Export CSV, Import CSV) linking to
  the existing screens from M3, unchanged themselves — just one level deeper in the nav instead of 3
  separate top-level rail icons. Keeps the rail from growing unbounded as future milestones add
  vault-wide actions.
- **Collapse/expand**: a dedicated toggle button pinned above the destination list (not part of the
  destination list itself). Expanded shows icon+label per item; collapsed shows icon-only at ~48px
  width, with the existing `<Tooltip>` component's flyout appearing on hover — reusing established
  UI, not introducing a new tooltip mechanism. Toggling is a deliberate click, not a hover effect; the
  state persists (`nav_collapsed` in config) and starts expanded.
- Active item gets an accent-tinted background (`var(--accent)` at reduced opacity, or similar) in
  both collapsed and expanded states.
- Replaces the current button row in `Vault.tsx`'s header — that row has grown to 6+ buttons already
  and the rail is a direct, structural replacement, not an addition alongside it.

## 6. Theming

- **Light/dark**: `config.theme` already exists and defaults to `"dark"`, but `index.html` hardcodes
  `class="dark"` on `<html>` and nothing reads the config value. Fix: apply the class reactively in
  `App.tsx` based on `config.theme`, on load and whenever Settings changes it. Because Tailwind's
  `dark:` variant here is already class-based (`@custom-variant dark (&:where(.dark, .dark *))` in
  `App.css`) and every screen only adds `dark:`-prefixed *overrides*, a working light-mode fallback
  already exists implicitly in every component — this is a small, low-risk change, not a rewrite.
  Light mode gets its first real visual check during the (deferred, per-user-decision) manual
  walkthrough after M6, since it's literally never been switched on before.
- **Accent color**: 5 cool-toned options — Indigo, Sky, Teal, Purple, Pink — chosen over a "vibrant"
  or "muted" alternative specifically to avoid clashing with the red/green this app already uses for
  error/success text. Implemented as CSS custom properties (`--accent`, `--accent-hover`, etc.)
  defined per value of a `data-accent="indigo"` attribute on `<html>`, set the same way as the theme
  class. The ~11 existing hardcoded `bg-blue-600`/`text-blue-400`/`ring-blue-*`/`border-blue-*`
  occurrences across `Login.tsx`, `Register.tsx`, `Vault.tsx`, `CredentialEditor.tsx`,
  `SiteEditor.tsx`, `SiteDetail.tsx`, `Backup.tsx`, `CsvExport.tsx`, `CsvImport.tsx` get swapped to
  reference the variable (e.g. Tailwind arbitrary-value syntax `bg-[var(--accent)]`) — a small,
  mechanical, well-bounded refactor across 9 files, not a redesign.
- Both pickers live in `Settings.tsx`, next to the existing auto-lock and strength-algorithm
  controls — same screen, same `submitting`/`error` idiom already established there.

## 7. Generator Screen

Standalone screen reachable from the nav rail: pick a generation method, generate, see live strength
feedback, copy to clipboard (auto-clears after 30s, reusing the existing `copy_to_clipboard` command
unchanged). No new IPC commands — `generate_password` and `analyze_strength` already exist from M2.
No site-picking, no "save to vault" step; if the user wants to keep what they generated, they go
through Add/Edit Credential as normal (which already has its own embedded generator from M2 — this
is a separate, simpler entry point for "I just need a password right now," not a replacement).

**Implementation note for the plan:** `CredentialEditor.tsx` already has method-selector + strength-
meter + copy-button UI embedded (M2). Before writing the standalone screen from scratch, check
whether extracting a shared component (e.g. a `<PasswordGeneratorPanel>`) both screens use is a
smaller diff than duplicating that UI — "look before you write" per this project's established
ponytail discipline. This is an implementation-time call, not a design-level one; either shape
satisfies this spec.

## 8. Testing

- **Mandatory/automated (Rust):** `core::health`'s weak/reused/old classification against known
  fixtures; the k-anonymity match logic against a mocked suffix list (no real HIBP call in tests);
  `update_credential`'s password-changed-vs-unchanged branching for `password_updated_at`; the 4 new
  config fields' validation (mirroring the existing `auto_lock_minutes`/`strength_algorithm` test
  pattern exactly — missing-field defaults, invalid-value fallback, round-trip); every new IPC
  command including `check_breach`'s opt-in gate (`hibp_opt_in: false` must reject regardless of
  what the frontend sends, same shape as the plaintext-export test in M3).
  - `check_breach`'s HTTP-calling code needs the injectable base URL from §4 to be testable without
    live network access — this is a hard requirement for TRD §7's "mandatory, automated" bar, not
    optional polish.
- **Lighter/manual (frontend/visual):** nav rail collapse/expand behavior, theme/accent visual
  correctness, tooltip flyout positioning, Health Dashboard card layout — all deferred to the single
  consolidated manual walkthrough after M6, per the user's 2026-07-14 decision (see `RESUME_HERE.md`).

## 9. Open items / deferred

None blocking — all decisions above were made interactively during brainstorming. One thing flagged
but intentionally left as-is: `ExportMode::WithPassword.password` (from M3) still isn't
`Zeroizing`-wrapped, inconsistent with `ImportRecordDto`/`CredentialDto`'s stricter bar — not part of
M4's scope, mentioned here only so it isn't lost; revisit only if the user wants that stricter bar
applied project-wide.
