# Modern UI Overhaul — Figma Make port as a third UI variant

**Status:** Planning — export analysed, decisions outstanding. No code written.
**Branch:** `feature/modern-ui-overhaul`
**Drafted:** 2026-09-20 · **Revised:** 2026-09-20 after reading the Figma Make export

---

## Context

The app is a local-first zero-knowledge password manager (Tauri 2 + React 19 + Vite 7 +
Tailwind v4, Rust `core/` backend). The goal is a UI matching the prototype built in **Figma Make**:
`figma.com/make/41R6vv2SQkLtUZdLXpjevA/Password-manager-UI-design`.

The Make URL cannot be read programmatically — it is a Make URL, not a Design URL, so the Figma
MCP's `get_design_context` (built for Design-file nodes) does not apply, and `WebFetch` returns
nothing against an auth-gated JS app. That does not matter, because **the source export is already
in the repo at `figma-make/`** and Figma Make emits the same stack this app runs. The design has
been read directly. Everything below is grounded in that source, not inferred.

**A Figma port already shipped once.** Phase 8 built `src/newui/` behind `ui_variant: "new"`. The
decision is to **ignore it and start anew** as a **third variant**, so Phase 8 stays on disk,
untouched and still selectable.

That choice pays off concretely: `src/newui/lib/parity.test.ts:104` hard-fails the build if the
words *favorite*, *folder*, *types*, *sync*, or *history* appear anywhere under `src/newui/` — the
D-10 scope cuts, banned "in any state (not even as disabled stubs)". That test walks `src/newui/`
only (`parity.test.ts:16`), so a new sibling directory is out of its scope. **The prototype uses all
five of those concepts**, and building them fresh needs no change to that test and no unpicking of
the Phase 8 contract.

**Outcome:** a `ui_variant: "make"` UI matching the prototype, with the cut features built for real
against the encrypted vault — not mocked, not disabled stubs.

---

## Part A — What the export actually contains

`figma-make/` — 6 source files, **zero UI dependencies** (just `react`, `react-dom`,
`tailwindcss@4`). No shadcn, no lucide-react, no radix, no CVA.

| File | Lines | Contents |
|---|---|---|
| `src/App.tsx` | 1140 | The entire design — every component, all mock data, all 23 icons |
| `src/index.css` | 48 | Google-Fonts imports, a Tailwind v4 `@theme inline` block, scrollbar styling |
| `src/main.tsx` | 10 | Mount |
| `src/imports/*.md` | — | Copies of this repo's `docs/research/FIGMA_UI_VS_CURRENT_FEATURES.md` and `docs/research/UI_REDESIGN_CAPABILITY_CHECKLIST.md`, used to seed the Make prompt |

### A1. The three open questions are now answered

1. **Light and dark — both.** `DARK` and `LIGHT` palette maps at `App.tsx:16-31`, toggled by a
   sun/moon button in the sidebar footer (`App.tsx:479-486`). The existing theme toggle survives.
2. **The accent picker does not survive.** The accent is hardcoded `#175ddc` (Bitwarden blue) in
   both palettes and in ~15 literal `bg-[#175ddc]` class strings. There is no five-hue concept. This
   conflicts head-on with the app's `data-accent` system (`src/App.css:27-77`) and its Settings →
   Appearance picker. **Decision needed** — see D-A below.
3. **Fonts: Inter + JetBrains Mono, pulled from the Google Fonts CDN** (`index.css:1-2`). A
   local-first offline vault app must not fetch fonts over the network at runtime. These get
   bundled into the app instead — see B2-8. It is a real change from today (the app ships no
   custom font at all).

### A2. What the prototype actually covers

Screen coverage is **partial**, and this is the single most important planning fact.

| Surface | State in the prototype | Reality |
|---|---|---|
| Auth / unlock | ✅ Designed | `AuthScreen`, `App.tsx:1040` |
| Sidebar | ✅ Designed | `App.tsx:366-493` |
| Item list (middle pane) | ✅ Designed | `App.tsx:535-702`, incl. search + empty state |
| Item detail (right pane) | ✅ Designed | `App.tsx:768+`, incl. Details/History tabs |
| Health Dashboard | ⚠️ **Stub** | Static numbers, and rendered *inside the 280px middle pane* — `max-w-2xl` content inside `w-[280px]` inside `h-40` (`App.tsx:594-620`). A layout bug, not a design. |
| Password Generator | ⚠️ **Stub** | Same — static string, non-functional slider, wrong pane (`App.tsx:623-659`) |
| Data Hub | ⚠️ **Stub** | Same — four static cards, wrong pane (`App.tsx:662-687`) |
| Trash | ⚠️ Filter only | A `trashed` flag filter (`App.tsx:1097`); no restore/purge UI |
| Settings | ❌ Missing | Footer button exists (`App.tsx:487`), opens nothing |
| Register / account creation | ❌ Missing | — |
| Add / edit item | ❌ Missing | `+` button exists (`App.tsx:571`), opens nothing |
| CSV import / export, Backup / restore | ❌ Missing | Named on Data Hub cards only |
| Responsive / mobile | ❌ **Absent** | **Zero breakpoints** in 1140 lines. Fixed `w-[220px]` + `w-[280px]`. The app has a shipped responsive mode (M6: `BottomTabBar`, `NavDrawer`) the design does not address. |

So "make it look exactly like Figma" is achievable for roughly **four surfaces**. The other ten are
either sketches in the wrong pane or absent. They still have to exist in a shipping app — they will
need to be designed *in the prototype's visual language* rather than copied from it.

### A3. What the prototype adds to the data model

All five D-10 concepts are present, plus history:

| Concept | Evidence in `App.tsx` |
|---|---|
| Favorites | `IconStar`, `favorite: boolean` on every item, a Favorites nav entry with a count (`:411`, `:432`) |
| Folders | `FOLDERS` array (`:156`), collapsible sidebar section (`:443-457`), `folder?: string` per item |
| Item types | `type ItemType = "login" \| "note" \| "card" \| "identity"` (`:34`), four nav entries with counts, per-type fields (`cardNumber`/`cardExpiry`/`cardCVV`, `noteContent`) |
| Sync | Footer button (`:472-475`) |
| History | A **Details / History tab pair** in the detail panel (`:771`, `:838`) |

**Good news on the model fit.** The prototype was clearly built against this app's real schema:

- `CustomField { label, value, fieldType: "text" \| "hidden" \| "boolean" }` (`App.tsx:37-41`) is
  **identical** to `CustomField`/`CustomFieldType` in `core/src/vault.rs:111-130`.
- `reprompt?: boolean` matches the existing `credentials.reprompt` column.
- The nested `credentials?: Credential[]` shape (`App.tsx:68`, sample at `:90-93`) matches the
  app's site→credentials model exactly (Phase 8 decision D-08).

So types, favorites, folders and history are genuinely new; custom fields, reprompt, TOTP and the
site/credential relationship already exist and map cleanly.

---

## Part B — What needs to be built

### B0. Decisions required before any code

These are the forks that change the shape of everything downstream. None can be defaulted safely.

| ID | Decision | Why it cannot be deferred |
|---|---|---|
| **D-A** | **Accent: keep the five-hue picker, or go fixed Bitwarden blue?** | The design hardcodes `#175ddc` in ~15 places. Keeping the picker means routing those through `--accent` and accepting the design will not look identical on four of five hues. Dropping it means removing a shipped, user-facing feature from Settings → Appearance. |
| **D-B** | **Theming: convert to CSS variables, or keep inline style objects?** | The prototype has **110 inline `style={{…}}`** and only **5** `var(--…)` reads — it themes via JS objects (`T = dark ? DARK : LIGHT`), while this app themes via a `.dark` class plus CSS variables (`src/App.css:83-120`). Keeping the JS approach breaks the accent system and the existing dark-mode seam. **Recommendation: convert `DARK`/`LIGHT` into CSS variables once, then port markup against those.** Doing this first makes every later screen cheap; doing it late means rewriting 110 sites. |
| **D-C** | **Icons: 23 inline SVGs, or bootstrap-icons?** | The app's rule is one choke point, `src/components/Icon.tsx`, bootstrap-icons only, **never inline SVG** (D-13). The prototype hand-writes 23 inline SVG components (`App.tsx:159-358`). Pixel fidelity requires the SVGs. **Recommendation: keep the 23 SVGs, scoped to the new variant only** — they are the design, and this variant is isolated. Do not add a third icon system later. |
| **D-D** | **The ten undesigned surfaces — who designs them?** | Settings, Register, add/edit forms, CSV, backup/restore, and the three stub panes must exist. Either go back to Figma Make and design them, or extend the visual language here and accept they are not Figma-derived. |
| **D-E** | **Responsive: does the new variant support mobile?** | The design has zero breakpoints. The app ships a responsive web portal (M6). Either the new variant is desktop-only (and the web portal keeps Classic), or breakpoints get invented. |
| **D-F** | **Sync** | ⚠️ The design shows a Sync button. This app is local-first zero-knowledge by design (`.planning/ROADMAP.md`). Sync means a server, accounts, conflict resolution, and a new threat model. **Recommendation: build no sync and render no button** — an inert button in a security product is a lie about where your data is. |

### B1. Backend / vault schema

Current schema is `core/src/vault.rs:52-82`. `sites` has exactly
`id, name, url, notes, created_at, updated_at, deleted_at` — no favourite, no folder, no type.
The migration mechanism is `add_credential_columns_if_missing` (`core/src/vault.rs:93-109`), an
idempotent `PRAGMA table_info` + `ALTER TABLE ADD COLUMN` check. **There is no migrations table**;
a `sites` equivalent must be written the same way.

| # | Feature | Schema change | Rust work | Notes / risk |
|---|---|---|---|---|
| B1-1 | **Favorites** | `sites.favorite INTEGER NOT NULL DEFAULT 0` | `toggle_site_favorite`; `favorite` on `Site` (`vault.rs:158`) + DTO | Smallest. Plaintext boolean — a favourite flag is metadata, not a secret. |
| B1-2 | **Folders** | New `folders` table + `sites.folder_id INTEGER REFERENCES folders(id)` | CRUD; folder filter; reassign-on-delete rule | FK enforcement is per-connection (`vault.rs:44`) — already on. Decide orphan vs. cascade. Design shows flat folders only (no nesting) — keep it flat. |
| B1-3 | **Item types** | `sites.item_type TEXT NOT NULL DEFAULT 'login'` | Type enum + validation; type-aware add/edit; per-type field sets | **Largest.** Four types, and `card`/`identity`/`note` carry fields `sites` has no columns for. **Check first whether the existing encrypted `CustomField` blob (`vault.rs:111-130`) can carry them** — it is already the right shape, and reusing it avoids new plaintext columns for card numbers. Card numbers and CVVs must land in the encrypted blob, never a plaintext column. |
| B1-4 | **Password history** | New `credential_history` table, encrypted per row (mirroring `secret_enc`/`secret_nonce`) | Write-on-update hook; history read command; retention cap | Every historical password is as sensitive as the live one — same seal, same reprompt gate. Also changes **backup/restore and CSV export** (`core/src/backup.rs`, `portio.rs`). |
| B1-5 | **Sync** | — | — | Not built. See D-F. |

All of B1-1..B1-4 also need DTO updates (`core/src/dto.rs`), command registration
(`core/src/commands.rs`), the TS side of the single IPC seam (`src/lib/api.ts`), and backup/restore
round-trip coverage.

### B2. Frontend — variant scaffolding

| # | Item | Files | Notes |
|---|---|---|---|
| B2-1 | New entry point | `makeui.html`, `src/makeui-main.tsx` | Mirror `newui.html` + `src/newui-main.tsx`, **including its vanilla-DOM crash fallback** (`newui-main.tsx:25-72`) |
| B2-2 | Register with the bundler | `vite.config.ts:36-40` | 4th rollup input beside `main`/`viewer`/`newui` |
| B2-3 | **Register the Tailwind source glob** | `src/App.css:11-12` | ⚠️ Add `@source "../makeui.html";`. Per `App.css:17-19` an uncovered source **fails silently** — classes render unstyled with no build error. Easiest thing to forget. |
| B2-4 | Allow the variant | `core/src/config.rs:186` (`VALID_UI_VARIANTS`) | Currently `"classic"` / `"new"` only |
| B2-5 | Boot routing | `src-tauri/src/lib.rs:28-48` | Add `"make" => "makeui.html"`, **keeping the asset-guard pattern** (`asset_resolver().get(...)` → fall back to Classic). Never interpolate the config value into a path (T-08-01). |
| B2-6 | Extend the variant test | `core/src/commands.rs:2394` | Add a positive case for `"make"` |
| B2-7 | Settings toggle | `src/screens/Settings.tsx` Interface section (D-07) | Two-way radio becomes three-way |
| B2-8 | **Bundle the fonts as npm packages** | `package.json`, `src/App.css` | See B2-8 detail below. |
| B2-9 | Web portal | none | No change — `web-src-tauri` serves `dist/` wholesale (`handlers.rs:867`), per D-05 |

#### B2-8 detail — fonts ship with the app, no CDN, no manual font files

**Requirement:** the fonts must be part of the project and work for anyone who runs the app, with
no network call and no hand-managed font files.

**Approach: install them as npm dependencies.** The repo already does exactly this — `bootstrap-icons`
is an npm dep whose woff2 is pulled in by a plain CSS `@import` at `src/App.css:4`. Fonts use the
same mechanism, so there is no new pattern to learn and nothing to invent.

```bash
npm install @fontsource/inter @fontsource-variable/jetbrains-mono
```

Then in `src/App.css`, alongside the existing `bootstrap-icons` import (CSS requires all `@import`
rules to precede any other statement — the file already notes this at `:1-3`):

```css
@import "@fontsource/inter/400.css";
@import "@fontsource/inter/500.css";
@import "@fontsource/inter/600.css";
@import "@fontsource/inter/700.css";
@import "@fontsource-variable/jetbrains-mono";
```

Weights 400/500/600/700 match what the prototype requests (`figma-make/src/index.css:1-2`).

**Why this satisfies the requirement:**

| Concern | How it is met |
|---|---|
| "Installable via setup" | `npm install` — the same step that already installs React and bootstrap-icons. No separate font installer, nothing for the user to do. |
| "Available for anyone who runs my app" | Vite fingerprints the woff2 files into `dist/assets/`, and Tauri embeds `dist/` into the binary. The fonts travel inside the app; they do not depend on the machine having Inter installed. |
| "I don't want to host it" | Nothing is hosted. There is no server and no CDN — the font bytes sit in the app bundle, exactly like the bootstrap-icons glyph font does today. |
| Offline / zero-knowledge | No network request at runtime. This is required, not optional, for a local-first vault app. |

**Terminology note:** in web terms this *is* "self-hosting" the fonts — it means serving them from
your own bundle instead of Google's CDN. It does **not** mean running a font server. The plan's
earlier wording was imprecise; the mechanism above is what was always meant.

**Must also do:** delete the two Google Fonts `@import url(...)` lines when porting
`figma-make/src/index.css:1-2`. Leaving them in reintroduces a runtime network call that would
silently fall back to a system font when offline — and on a machine that happens to have Inter
installed, that failure is invisible during testing. The offline check in *Verification* below
exists specifically to catch this.

**Licensing:** both fonts are SIL Open Font License, so bundling and redistributing them inside a
shipped binary is permitted. Retain the license files that ship inside the packages.

### B3. Frontend — the design itself

| # | Item | Notes |
|---|---|---|
| B3-1 | **Token layer first** | Convert `DARK`/`LIGHT` (`App.tsx:16-31`) into CSS variables in the app's system (`src/App.css:83-120`). Per D-B this is the first task, not a later cleanup. 19 tokens per theme; the app currently has 11. |
| B3-2 | Icon module | Extract the 23 SVGs from `App.tsx:159-358` into `src/makeui/icons.tsx`. Per D-C, scoped to this variant. |
| B3-3 | The four designed surfaces | `AuthScreen`, `Sidebar`, `ItemList`, `DetailPanel` — port from the export, wire to `src/lib/api.ts`, replace `VAULT_ITEMS`/`FOLDERS` mock data with real vault reads |
| B3-4 | The ten undesigned surfaces | Blocked on D-D |
| B3-5 | **Reuse the security-critical parts** | `SecretField` (reprompt-gated reveal, fixed-width mask — D-15), `MasterPasswordPrompt`, `TotpDisplay`, `src/lib/api.ts`, `src/lib/clipboard.ts`. **Restyle these, never reimplement them.** The prototype's `DetailField` (`App.tsx:732`) renders secrets with a plain local `useState` and no reprompt gate — it is a mockup and must not be ported as-is. The mask must not reveal length; clipboard must not go through `navigator.clipboard`. |

---

## Part C — Fidelity verification

The export is a runnable Vite app in the same stack, so fidelity can be checked by running it
rather than by eye.

⚠️ **It will not run as-is.** `figma-make/vite.config.ts` imports `./.figma/make/site.json` and four
Figma-internal plugins (`figmaSiteConfiguration`, `figmaErrorOverlayReplay`,
`figmaReactRefreshBoundaryFallback`, `figmaMakeKitPlugin`) that only exist inside Make's sandbox.
Running it locally needs a stripped `vite.config.ts` — react + tailwind plugins and the `@` alias
only. The `.figma/make/` directory is present, so this is a small edit, not a blocker.

1. Strip the config, `pnpm install && pnpm dev` (the lockfile is `pnpm-lock.yaml`, not npm)
2. Screenshot every surface, both themes, into `figma-make/screenshots/`
3. Port, then screenshot the ported variant at the same sizes
4. Diff side by side, surface by surface

Step 2 doubles as the reference set — no manual screenshot export from Figma is needed.

---

## Sequencing

1. **Resolve B0** (D-A … D-F) — everything else is blocked on these
2. Part C steps 1-2 — get the prototype running locally, capture the reference screenshots
3. B3-1 + B3-2 — token layer and icon module (do these before any screen)
4. B2 — scaffold the variant, prove an empty `makeui.html` boots via `ui_variant: "make"`
5. B1 — schema + Rust, feature by feature, each with backup/restore round-trip coverage
6. B3-3 — port the four designed surfaces against real vault data
7. B3-4 — the ten undesigned surfaces, per D-D
8. Part C steps 3-4 — visual diff pass

Steps 5-7 are large enough to run as real GSD phases (`.planning/phases/`). Steps 1-4 are this
branch's immediate work.

## Verification

- `npm test` — must stay green, **including `src/newui/lib/parity.test.ts`** (untouched Phase 8
  contract) and the source-text gate `src/components/Spinner.test.ts`
- `npm run build` — `tsc` type-check plus the 4-input Rollup build
- `cargo test` — `core/` migration tests, incl. opening a **pre-migration vault file** and
  confirming the new `sites` columns are added idempotently
- `npm run tauri:dev` — switch `ui_variant` across all three values; confirm each boots, and that a
  deliberately missing `makeui.html` asset falls back to Classic instead of a blank window
- **Offline check** — run with the network disabled and confirm no font request is attempted
- Manual: unlock → reveal a secret → confirm the reprompt gate fires and the mask stays fixed-width
- Final: side-by-side screenshot diff against `figma-make/screenshots/`

---

## Housekeeping

`figma-make/` is currently **not gitignored** and would be committed. That is probably right — it is
the design contract and worth version-controlling. Two notes if it is committed: its mock data
contains realistic-looking sample passwords (fake, but noisy in a password-manager repo), and
`node_modules/` must stay ignored if it is ever installed there.

## Reference: prior art

- `.planning/phases/08-new-vault-ui-figma-port/8-UI-SPEC.md` — Phase 8 design contract
- `.planning/phases/08-new-vault-ui-figma-port/8-CONTEXT.md` — the D-01..D-18 decisions
- `docs/research/FIGMA_UI_VS_CURRENT_FEATURES.md`, `docs/research/UI_REDESIGN_CAPABILITY_CHECKLIST.md` — also the two
  files used to seed the Make prompt (`figma-make/src/imports/`)
