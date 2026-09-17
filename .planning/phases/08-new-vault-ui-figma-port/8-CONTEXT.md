# Phase 8: New Vault UI (Figma Port) - Context

**Gathered:** 2026-09-13
**Status:** Ready for planning

<domain>
## Phase Boundary

Build a NEW, Figma-derived three-pane vault UI as a **second, parallel frontend**
inside this same repo/app — not a replacement of the existing production `src/`
frontend. A restart-required Settings toggle switches the desktop window between
"Classic" (today's UI) and "New" (this phase's deliverable). Scope is wired
against **today's 54 backend commands only** — no backend schema changes in
this phase. This supersedes the phase's original ROADMAP.md goal text (which
described item types/favorites/folders/per-item health as in-scope); the user
explicitly narrowed scope this session after redesigning the Figma mockup and
confirming the backend has zero persistence for those fields. Roadmap text was
deliberately left unedited by user choice — this CONTEXT.md is the authoritative
scope statement for planning.

</domain>

<decisions>
## Implementation Decisions

### Deployment shape
- **D-01:** Build as a 3rd Vite entry point (`newui.html` → `src/newui-main.tsx`
  → `src/newui/NewApp.tsx`), alongside the existing `index.html`/`viewer.html`
  pair. Not a separate repo, branch, or replacement.
- **D-02:** A new `config.ui_variant: "classic" | "new"` field (default
  `"classic"`) persists the choice in `~/.pwmgr/config.json`, following the
  exact existing pattern of `mobile_nav_style`/`accent_color` in
  `core/src/config.rs` (VALID_* const, default fn, snap-to-default on invalid
  load, added to `Default` impl and both exhaustive test struct literals).
- **D-03:** `src-tauri` currently has NO explicit window-creation code — the
  window comes entirely from the static `windows` array in `tauri.conf.json`.
  Change `tauri.conf.json`'s `windows` to `[]` and build the window explicitly
  inside `.setup()` in `src-tauri/src/lib.rs`, reading `ui_variant` the same
  way `ssh_agent_enabled_at_startup` is already read (right after `AppState::open`),
  choosing `index.html` or `newui.html` as the `WebviewUrl::App` target.
- **D-04:** Window label MUST stay `"main"` — `src-tauri/capabilities/default.json`
  scopes `clipboard-manager:*` and `dialog:*` permissions to that label. A
  different label silently breaks clipboard copy and file dialogs.
- **D-05:** Web portal (`web-src-tauri`) needs NO routing change — `dist/newui.html`
  is already served by its existing `ServeDir`/`ServeFile` fallback once built.
  Register the `update_ui_variant` setter there too (so its Settings screen can
  save the preference) but the portal does not act on it locally — no "restart"
  concept in a browser tab. Settings hint text differs: desktop shows a restart
  banner, web shows "open /newui.html in this browser."
- **D-06:** Viewer (`viewer-src-tauri`) is explicitly EXCLUDED from the toggle —
  it has only 9 commands and no `verify_master_password`, so the new UI's
  `SecretField`-based reprompt gate cannot function there.
- **D-07:** Settings UI gets a new "Interface" section (after Appearance) with
  a select control (same markup pattern as the mobile-nav-style control) plus
  a **persistent banner** (not a toast — `ToastHost` auto-dismisses in 2000ms)
  shown when the saved value differs from the value captured at app boot.
  Self-relaunch via `tauri-plugin-process` is explicitly DEFERRED (needs a new
  capability grant, saves the user only one click since the session key is
  destroyed on restart either way).

### Data model
- **D-08:** Item = site, detail panel is a credential repeater. This matches
  the real `sites` 1:N `credentials` schema AND the Figma mockup's own
  redesigned `credentials: Credential[]` nesting (the mockup's second iteration
  validated this independently). 0 credentials → empty state + "Add credential"
  CTA. 1 credential → single expanded block (the common case). 2+ → first
  expanded, rest behind "Other logins (N−1)" (local UI state only, no
  persistence needed).
- **D-09:** New `useVault()` hook (`src/newui/hooks/useVault.ts`) replaces the
  mockup's module-scope `VAULT_ITEMS` const entirely — sites load eagerly with
  no secrets (`api.listSites()`), credentials for a site load ONLY on
  selection (`api.listCredentialsForSite`), cleared on deselect and on
  `vault-locked`. This is the **lazy-decryption invariant** and is
  non-negotiable — no credential-derived sidebar badge counts are possible
  under this rule (there is no cross-site credential listing command).

### Scope cuts (no backend persistence exists for these — confirmed by full
schema read, not assumption)
- **D-10:** OMIT entirely from v1: favorites star/nav entry, folders
  sidebar section, item-type filters (Logins/Notes/Cards/Identities nav
  entries) AND the add-item type picker, sync button (already a recorded
  non-goal), the mockup's fabricated "History" tab (no backing command/field
  exists). This is parity with today's real nav (Vault/Health/Generator/
  Trash/Data/Settings), not a regression — do not show these as disabled
  stubs either; absence, not fake functionality.

### Reuse vs. build new
- **D-11:** Reuse wholesale, unchanged: `SecretField`, `TotpDisplay`,
  `PasswordGeneratorPanel`, `Icon` (bootstrap-icons — do NOT port the
  mockup's 23 hand-written inline SVGs), `Tooltip`, `ToastHost`,
  `useActivityWatcher`, `Login`, `Register`, `SiteEditor`, `CredentialEditor`,
  and the 5 secondary screens (`Generator`, `HealthDashboard`, `Data`,
  `Trash`, `Settings`) — each wrapped in a thin `{onBack}`-taking pane
  component re-homed into the flex-1 detail column.
- **D-12:** Reuse `src/lib/api.ts` as-is — the new UI imports this same
  module (never `invoke` directly). No new IPC wrapper file.
- **D-13:** Adopt the real Tailwind token system in `src/App.css`
  (`--bg`/`--surface`/`--fg`/`--border`/`--accent*`, `.dark` class,
  `data-accent` attribute) — do NOT port the mockup's `DARK`/`LIGHT` JS maps
  + inline `style={{}}` theming, which doesn't compose with the real 5-accent
  picker.
- **D-14:** Build new (no equivalent exists today): `Sidebar`,
  `ItemList`/`ItemRow`, `DetailPanel`/`CredentialBlock`, `NewShell`
  (the single breakpoint owner for responsive tiers).

### Bug fixes vs. the mockup (do not reproduce)
- **D-15:** Length-revealing secret mask (mockup's `"•".repeat(min(len,24))`)
  → use `SecretField`'s existing fixed-width mask instead.
- **D-16:** Light-mode-leaves-dark-background bug (mockup's `index.css`
  hardcodes dark palette on `html,body,#root`) → do not replicate; `newui.html`
  keeps the same static `class="dark" data-accent="indigo"` anti-flash
  pattern `index.html` already uses, and the shell renders via `--bg`/`--fg`
  tokens so light mode is correct by construction.
- **D-17:** Health block gated on `item.type==="login" && item.password`
  (never true for nested-credential items in the mockup) → gate on
  `credentials.length > 0` per credential instead.

### Responsive story (mockup has none)
- **D-18:** `NewShell` owns the one breakpoint. ≥1024px: full sidebar
  (respects `navCollapsed`) + 280px list + flex-1 detail. 640-1023px:
  icon-rail sidebar + same list/detail. <640px: sidebar replaced by whichever
  of `BottomTabBar`/`NavDrawer` the user's `mobileNavStyle` config names
  (reused unchanged), list/detail collapse to one visible pane switched
  purely by `selectedSiteId !== null`.

### Claude's Discretion
- Exact component prop shapes within `src/newui/components/` beyond what's
  named in D-11/D-14.
- Precise banner copy/styling in Settings' Interface section (beyond "must be
  a persistent banner, not a toast").
- Order of Phase 3 (CRUD wiring) sub-steps.

</decisions>

<specifics>
## Specific Ideas

Directory layout (fixed, not discretionary — matches the approved implementation plan):

```
newui.html                        copy of index.html, class="dark" data-accent="indigo" kept
src/newui-main.tsx                mirrors src/main.tsx
src/newui/
  NewApp.tsx                      auth gate, vault-locked listener, theme bootstrap,
                                   useActivityWatcher, <ToastHost/> (mounted exactly once —
                                   toast.ts and the theme effect are module-level/per-root,
                                   not React context; a 3rd root needs its own of each)
  NewShell.tsx                    three-pane layout + breakpoint owner
  components/
    Sidebar.tsx, SidebarNavItem.tsx
    ItemList.tsx, ItemRow.tsx
    DetailPanel.tsx, CredentialBlock.tsx, DetailField.tsx, CopyButton.tsx, PaneSurface.tsx
  screens/
    GeneratorPane.tsx, HealthPane.tsx, DataPane.tsx, TrashPane.tsx, SettingsPane.tsx
  hooks/
    useVault.ts
```

Phasing (Classic must never break): 0) plumbing only, placeholder new UI —
toggle → restart → placeholder, toggle back → restart → Classic byte-identical
→ 1) shell + real vault (list/detail/search/reveal/copy/TOTP) → 2) auth+lock
integration → 3) CRUD wiring → 4) secondary panes → 5) responsive tiers →
6) parity pass against `docs/UI_REDESIGN_CAPABILITY_CHECKLIST.md` §1 (18
flows) and §4 (6 cross-cutting behaviors), plus greps for zero
`navigator.clipboard`, zero direct `invoke` imports in `src/newui/`, and
every `fieldType==="hidden"` routed through `SecretField`.

"I want both UIs shippable with a toggle, not a hard cutover" — direct user
quote-equivalent from this session's decisions.

</specifics>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Backend contract (ground truth — do not re-derive)
- `docs/UI_REDESIGN_CAPABILITY_CHECKLIST.md` — the authoritative flow→command
  map (§1), full 54-command inventory with per-binary availability (§2), wire
  DTOs (§3), the 6 non-negotiable cross-cutting behaviors (§4), data-model
  facts the new UI must accept as given (§5), and explicit non-goals (§6).
  Use §1 as the literal acceptance checklist at Phase 6 (parity pass).

### Gap analysis / design intent
- `docs/FIGMA_UI_VS_CURRENT_FEATURES.md` — feature-by-feature comparison
  between the Figma design and the shipped app; explains WHY each gap exists
  and what's deliberately deferred (sync, CSV for new types, viewer support,
  accent picker uniformity).

### Existing code the new UI reuses (not re-derived — read directly)
- `src/lib/api.ts` — the complete IPC seam (58 wrapper methods, dual
  transport `isTauri ? invoke() : httpPost`), DTOs, `isTauri` export.
- `src/App.css` — the real Tailwind v4 theming contract: `@source` globs
  (any new source path/extension needs a matching line or classes render
  unstyled with no build error — `newui.html` sits outside `src/`, needs its
  own `@source "../newui.html"` line), class-based dark mode, `data-accent`
  accent-ramp aliasing, semantic tokens.
- `core/src/config.rs` — the field-add pattern to copy exactly for `ui_variant`
  (see `mobile_nav_style` as the closest analog, including its 2 test
  struct literals and 2 analog tests).
- `src-tauri/src/lib.rs` — current window/command-registration shape (52
  registered commands as of this session; `update_ui_variant` becomes #53
  — checklist doc's "54 commands" figure needs a follow-up update once this
  phase ships).
- `web-src-tauri/src/handlers.rs` — `MobileNavStyleRequest`/
  `update_mobile_nav_style` handler is the exact pattern to mirror for the
  new setter route.
- `src-tauri/capabilities/default.json` — confirms the window-label
  constraint (D-04); no edit needed here, just respect it.

No ADRs exist in this project (confirmed during a separate doc-ingest pass
this session) — these two docs plus this CONTEXT.md are the full spec.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/components/SecretField.tsx` — reprompt-before-reveal-or-copy gate +
  fixed-width mask + colorized digit/symbol reveal. Apply to BOTH the main
  secret and any `hidden`-type custom field (the real `SiteDetail.tsx` does
  both — easy to miss one).
- `src/components/TotpDisplay.tsx` — live 1s-polled 6-digit code.
- `src/components/PasswordGeneratorPanel.tsx`, `src/hooks/useActivityWatcher.ts`,
  `src/lib/toast.ts` (module-level pub/sub — see D-11 note on why).
- `src/screens/Vault.tsx`'s `refreshSites`/`runSearch` (including the
  deliberate no-debounce rationale) and `src/screens/SiteDetail.tsx`'s
  `Promise.all([listSites, listCredentialsForSite])` — lift this LOGIC into
  `useVault()`, don't copy-paste the JSX.

### Established Patterns
- Every secondary screen already takes `{ onBack }` and renders
  `flex-1 p-4` (`Data.tsx`, `Trash.tsx`, `Generator.tsx`, `Settings.tsx`,
  `HealthDashboard.tsx`) — dropping them into `NewShell`'s detail column is a
  one-line wrapper each.
- Editors (`SiteEditor`, `CredentialEditor`) are already modal overlays
  (`fixed inset-0 z-50` pattern from `SiteDetail.tsx`) — they float above a
  three-pane shell with no layout conflict.
- Config field pattern: `VALID_X` const → `default_x()` fn → field with
  `#[serde(default = "default_x")]` → `Default` impl entry → snap-to-default
  block in `load()` → setter in `commands.rs` (validate → clone → mutate →
  save → swap) → Tauri command → web handler + route → `api.ts` field +
  wrapper.

### Integration Points
- `src-tauri/src/lib.rs`'s `.setup()` closure is where the window gets built
  (new code) and is also where the existing autolock tick-loop spawns —
  window creation must happen before or independently of that loop, order
  doesn't matter functionally since they don't depend on each other.
- `Settings.tsx`'s `editedRef` guard (prevents a late `getConfig()` from
  clobbering an in-flight edit) — the new Interface section's boot-value
  capture for the restart banner must use a SEPARATE ref, not reuse `editedRef`,
  since the banner's comparison semantics differ (boot value vs. current
  saved value, not edited-vs-not).

</code_context>

<deferred>
## Deferred Ideas

- Backend Tier B additions (`sites.favorite`, `sites.folder`, `sites.item_type`
  columns) — explicitly out of scope for this phase by user decision. Would
  be a separate future phase/milestone if the user later wants these
  Figma-mockup features to actually persist.
- Self-relaunch after toggling the UI variant (`tauri-plugin-process`) — see
  D-07. Trivial follow-up, not blocking.
- `docs/UI_REDESIGN_CAPABILITY_CHECKLIST.md`'s "54 commands" count needs a
  follow-up doc update once `update_ui_variant` ships as command #53 (or
  wherever it lands in the final count).
- M4 Health Dashboard "Weak"-classification doc-vs-shipped-behavior mismatch —
  already tracked separately as
  `.planning/todos/pending/2026-09-13-resolve-m4-health-dashboard-weak-classification-doc-conflict.md`.
  Unrelated to this phase, not re-litigated here.

</deferred>

---

*Phase: 08-new-vault-ui-figma-port*
*Context gathered: 2026-09-13*
