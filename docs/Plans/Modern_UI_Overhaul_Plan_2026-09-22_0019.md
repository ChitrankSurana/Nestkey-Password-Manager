# Modern UI Overhaul — Figma Make port as a third UI variant (`ui_variant: "make"`)

**Supersedes:** `docs/Plans/Modern_UI_Overhaul_Plan.md` (drafted 2026-09-20 against a 1140-line export).
**Re-analysed:** 2026-09-22 against the current `figma-make/` export (2282 lines) and the 16 screenshots in `figma-make/screenshots/`.

---

## Context

This app is a local-first, zero-knowledge password manager (Tauri 2 + React 19 + Vite + Tailwind v4, Rust `core/`). It already ships two UIs: Classic (`src/screens/`, `src/components/`) and New (`src/newui/`, the Phase 8 Figma port).

A new prototype was built in Figma Make and exported into `figma-make/`. It is a complete, runnable React app in the same stack, and it is materially more finished than the version the previous plan analysed — what that plan listed as "stubs in the wrong pane" and "❌ Missing" are now real, full-width, working screens.

The goal is a third variant, `ui_variant: "make"`, that matches the prototype closely and is wired to the real encrypted vault. Phase 8's `src/newui/` stays on disk, untouched and still selectable, because `src/newui/lib/parity.test.ts:104-125` hard-fails the build if the words *favorite*, *folder*, *types*, *sync*, or *history* appear anywhere under `src/newui/` — and the prototype uses four of those five concepts. That test walks only `src/newui/` (`parity.test.ts:16`), so a sibling directory is out of scope and needs no change to the Phase 8 contract.

**Outcome:** a pixel-close port of the prototype, backed by real data, with the four new concepts (favorites, folders, item types, password history) built for real — not mocked, not disabled stubs.

---

## Part A — What the export actually contains (re-analysed)

`figma-make/` — 6 source files, **zero UI dependencies** (just `react`, `react-dom`, `tailwindcss@4`). No shadcn, no lucide-react, no radix, no CVA. Theme is two JS objects (`DARK`/`LIGHT`, `App.tsx:15-30`) applied through ~110 inline `style={{…}}` props. 31 hand-written inline SVG icon components (`App.tsx:208-301`; 30 ported, `IconWifi` dropped with Sync).

### A1. Screen coverage — the previous plan's table is out of date

| Surface | Prototype state | Evidence |
|---|---|---|
| Auth: unlock **and** register | ✅ Complete, with strength meter + "no recovery path" warning | `App.tsx:1718-1840` |
| Sidebar | ✅ Complete | `App.tsx:438-560` |
| Item list + search + empty state | ✅ Complete | `App.tsx:598-657` |
| Detail panel, Details/History tabs | ✅ Complete, per-type bodies | `App.tsx:660-926` |
| **Add / Edit item drawer** | ✅ **Complete** — right-side drawer, 4 type tabs, multi-credential repeater, custom fields, generate, reprompt checkbox | `App.tsx:1953-2188` |
| **Health Dashboard** | ✅ **Complete, full-width** — 5 expandable categories, breach opt-in toggle + k-anonymity note | `App.tsx:946-1096` |
| **Password Generator** | ✅ **Complete, full-width** — 4 methods (random/memorable/pattern/pronounceable), live strength | `App.tsx:1121-1300` |
| **Data Hub** | ✅ **Complete, full-width** — backup, restore, 3-mode CSV export, 3-step CSV import wizard | `App.tsx:1303-1508` |
| **Settings** | ✅ **Complete, full-width** — 5 collapsible sections | `App.tsx:1523-1715` |
| Trash | ✅ Filter + Restore / Delete Forever in detail header | `App.tsx:705-715`, `:2203` |
| Toasts, confirm dialog, reprompt modal | ✅ Present | `App.tsx:37-103`, `:375-408` |
| Responsive / mobile | ❌ **Absent** — zero breakpoints in 2282 lines; fixed `w-[220px]` + `w-[280px]` | `App.tsx:479`, `:616` |
| Sync | ❌ **Gone** — the sidebar footer is now lock / theme / settings only. No sync button anywhere. | `App.tsx:532-557` |

So the design now covers **eleven** surfaces, not four. The prototype's own `SettingsScreen` and `DataHub` are close enough to the shipped Classic screens to port directly rather than redesign.

### A2. Model fit — confirmed good

- `CustomField { label, value, fieldType: "text"|"hidden"|"boolean" }` (`App.tsx:109`) is **identical** to `CustomField`/`CustomFieldType` in `core/src/vault.rs:111-130`.
- `reprompt?: boolean` matches the migrated `credentials.reprompt` column (`vault.rs:98`).
- The nested `credentials?: Credential[]` shape (`App.tsx:110`, `:137-140`) matches the site→credentials model exactly (D-08).
- Every Settings control in the prototype **except one** already has a backend (see A3).

### A3. Settings — almost everything already exists

`src/screens/Settings.tsx` (756 lines) already implements, with working backends:

| Prototype control | Exists at | Backend |
|---|---|---|
| Auto-lock timeout | `Settings.tsx:388-404` | `do_update_auto_lock_minutes` `commands.rs:367` |
| Password age threshold | `Settings.tsx:406-422` | `do_update_old_password_days` `commands.rs:428` |
| Breach check opt-in | `Settings.tsx:424-434` | `do_update_hibp_opt_in` `commands.rs:440` |
| PIN quick-unlock | `Settings.tsx:436-472` | `do_set_pin`/`do_clear_pin` `commands.rs:64,72` |
| Biometric / Windows Hello | `Settings.tsx:474-518` | `do_set_biometric` etc. `commands.rs:110` |
| SSH agent + key list | `Settings.tsx:522-587` | `do_add_ssh_key` etc. `commands.rs:467-495` |
| Theme | `Settings.tsx:592-605` | `do_update_theme` `commands.rs:404` |
| Accent colour (5 hues) | `Settings.tsx:607-623` | `do_update_accent_color` `commands.rs:416` |
| UI variant | `Settings.tsx:648-667` | `do_update_ui_variant` `commands.rs:519` |
| Strength algorithm | `Settings.tsx:683-699` | `do_update_strength_algorithm` `commands.rs:388` |
| Reprompt mode + grace | `Settings.tsx:701-741` | `do_update_reprompt_mode` `commands.rs:531,543` |

**Only "Wipe Vault Data" (`App.tsx:1705-1707`) has no backend — and it is dropped** (decision D-G). Restore-from-backup already overwrites a vault; shipping a fake destructive button in a security product is worse than omitting it.

### A1b. Screenshot re-analysis — 16 screenshots (5 new, 2026-09-22)

The 5 new shots match `App.tsx` exactly; nothing contradicts the tables above. What they add:

| Screenshot | What it confirms / adds |
|---|---|
| `Folder.png` | Folder filter ("Development", 5 items, mixed logins + a note). Empty detail pane = shield-in-tile placeholder "Select an item to view details / Your vault is encrypted locally". Active folder row uses the accent-muted pill. **Folders list is fixed (6 hardcoded), no create/rename/delete UI** → gap surface B3-9 stands. |
| `type_of_credential,data_saved.png` | "Logins" type filter, count badge turns accent-coloured when active. Same empty-detail state. |
| `payment_card.png` | Card detail: number shown as `4*** **** **** 3921`, expiry `09/27`, CVV masked. **The mock stores the PAN pre-masked** — the real app must store the full PAN sealed and render this masked form by default (last-4 from the plaintext `card_last4` hint, D-H). CVV is `secret` (reprompt-capable), PAN is not in the prototype. |
| `SSH_Key_passphrase.png` | Secure note detail: note body in a `badge`-tinted card, **fully visible, no reveal control**. Favorite star shown hollow (not favorited). Real note bodies are sealed secrets → decision **D-M** below. |
| `website_multi_credential.png` | Multi-credential login: two stacked "Credential N" cards, a `🔒 PROTECTED` chip on the reprompt one, TOTP tile on credential 2, Website + Folder + dates below. Confirms D-08 repeater. |

**Still not in any screenshot:** Identity detail, light theme, any state below 1024px wide, the Settings screen, the register screen, the confirm dialog, and toasts. These are covered only by source (`App.tsx`) — hence Part C step 1 (capture light theme) and the Identity/Settings/Register captures.

**D-M (resolved) — note bodies:** the design shows note text always visible; note bodies are sealed (D-H). **Decision: masked with a reveal button** — render every note body through the real `SecretField` (fixed-width mask, reveal/copy, reprompt policy). This is a deliberate deviation from `SSH_Key_passphrase.png`; the note card keeps the design's `badge` tint and spacing, only the text is masked until revealed.

---

## Part B — Decisions (all resolved)

| ID | Decision | Resolution |
|---|---|---|
| **D-A** | Accent | **Keep the 5-hue picker.** Route the ~15 literal `bg-[#175ddc]` strings through `--accent`/`--accent-hover` (`src/App.css:27-31`). Retune the `indigo` pair to `#175ddc`/`#1252b8` **inside a makeui-only stylesheet**, so the default matches Figma exactly and Classic/New are untouched. |
| **D-B** | Theming | **Convert `DARK`/`LIGHT` to CSS variables first**, then port markup against them. 19 tokens per theme. Doing this late means rewriting ~110 inline-style sites. This is task 1, not a cleanup. |
| **D-C** | Icons | **Keep the 30 inline SVGs**, scoped to `src/makeui/icons.tsx`. They *are* the design. D-13's bootstrap-icons-only rule stays in force for `src/components/`. No third icon system beyond this. |
| **D-D** | Undesigned surfaces | **Extend the prototype's visual language** for the remaining gaps: folder CRUD, change master password, TOTP live code, real history entries, empty-trash-for-sites. Screenshot review after each. |
| **D-E** | Responsive | **Build it.** Copy the 3-tier pattern `NewShell.tsx:21-152` already proves: ≥1024px full sidebar, 640–1023px forced-collapsed sidebar, <640px master-detail collapse driven by `selectedSiteId` plus the `mobileNavStyle` branch. Reuse `BottomTabBar`/`NavDrawer` restyled. |
| **D-F** | Sync | **Nothing to do.** The prototype no longer has a sync button. |
| **D-G** | Wipe Vault Data | **Dropped.** No backend, and restore-from-backup covers the real need. |
| **D-H** | Card / note / identity storage | **Hybrid.** See B1 below. |
| **D-I** | Search scope | **Metadata + secret content**, decrypt-and-match in Rust. |
| **D-J** | Auth | **Add a Username field** to the prototype's unlock and create cards. The real app is multi-user (`do_login` `commands.rs:13`, vault per username); the prototype card shows master password only. |
| **D-K** | App name | **"LocalVault"** throughout the Make UI, as in the prototype. `productName` in `src-tauri/tauri.conf.json` is out of scope for now. |
| **D-L** | Variant lifetime | Third variant now; flip the default once it reaches parity, and consider retiring `"new"` then. Not this branch. |

---

## Part B1 — Backend work

Current schema: `core/src/vault.rs:50-86`. `sites` is exactly `id, name, url, notes, created_at, updated_at, deleted_at`.

**Two facts that shape everything here:**

1. **There is no `sites` migration helper.** `add_credential_columns_if_missing` (`vault.rs:93-109`) covers `credentials` only — `PRAGMA table_info` + guarded `ALTER TABLE ADD COLUMN`, called from `init_schema` (`vault.rs:84`) on every open. There is no migrations table and no `PRAGMA user_version` (`vault.rs:88-92`). **A new `sites` column added only to the DDL is invisible to every existing vault file.** Writing `add_site_columns_if_missing` in the same shape is task zero of this part.
2. **Backup needs no changes.** `encode_bundle` (`backup.rs:25-36`) carries the entire raw SQLCipher vault file as one opaque field (`commands.rs:639`). Schema changes ride along. Restore reopens through `init_schema`, so the migration helpers are also what repairs a stale restored vault.

### D-H, resolved: hybrid storage

The vault is SQLCipher — every column is already encrypted at rest. `secret_enc` / `custom_fields_enc` are a *second* seal (`crypto::seal`, `crypto.rs:57`) so a leaked DB key still yields no passwords. That means a dedicated `card_number_enc` column is an opaque BLOB and **is not SQL-searchable** — B bought no searchability while costing ~20 columns, each needing a hand-audited `Debug` impl (`dto.rs:78-79` warns the compiler gives no help there).

So:

- **Queryable metadata → real columns** on `sites`: `item_type`, `favorite`, `folder_id`, plus non-secret display hints (card brand, last 4 digits — already shown unmasked in the design, `App.tsx:179`).
- **Secret values → sealed `CustomField` blob** under reserved labels: full PAN, CVV, note body, identity fields. Reuses `seal_custom_fields`/`open_custom_fields` (`vault.rs:135-156`) — zero new crypto.
- **Reserved labels are a constant + validated**, not a convention. A typo must fail, not silently fork the field.

| # | Feature | Schema | Rust | Risk / note |
|---|---|---|---|---|
| B1-0 | **`add_site_columns_if_missing`** | — | Mirror `vault.rs:93-109` for `sites`; call from `init_schema` | **Do this first.** Everything below is invisible to existing vaults without it. Test against a pre-migration vault file. |
| B1-1 | Favorites | `sites.favorite INTEGER NOT NULL DEFAULT 0` | `toggle_site_favorite`; field on `Site` `vault.rs:158-164` + `SiteDto` `dto.rs:13-24` | Smallest. A favourite flag is metadata, not a secret. |
| B1-2 | Folders | new `folders` table + `sites.folder_id INTEGER REFERENCES folders(id)` | CRUD + filter + reassign-on-delete | FKs are on per-connection (`vault.rs:44`). Flat only — the design shows no nesting (`App.tsx:514-518`). Decide orphan vs. cascade explicitly. |
| B1-3 | Item types | `sites.item_type TEXT NOT NULL DEFAULT 'login'`, `card_brand TEXT`, `card_last4 TEXT` | Type enum + validation; per-type field sets via reserved `CustomField` labels | Largest. **CVV and full PAN never touch a plaintext column.** |
| B1-4 | Password history | new `credential_history` table, sealed per row (mirror `secret_enc`/`secret_nonce`) | Write-on-update hook in `update_credential` `vault.rs:450-471`; read command; **cap at 5**, prune oldest | Every historical password is as sensitive as the live one — same seal, same reprompt gate. Store metadata rows (created / changed, timestamp) alongside, as the design's History tab shows both (`App.tsx:909-920`). |
| B1-5 | **Empty trash for sites** | — | `do_purge_trash` (`commands.rs:361` → `vault.rs:545-548`) hard-deletes trashed *credentials* only; soft-deleted `sites` rows live forever | Extend to purge orphaned trashed sites in the same transaction. |
| B1-6 | **Change master password** | — | New command | **Highest-risk item — give it its own phase.** `derive_key` (`crypto.rs:47`) derives the vault key straight from the master password; there is no intermediate data key. So a change means: `PRAGMA rekey` the DB, **re-seal every sealed blob** (`secret_enc`, `custom_fields_enc`, `totp_secret_enc`, `ssh_keys.private_key_enc`), write a new verifier + sentinel, and **clear the PIN and biometric records** (`registry.rs:180,211`) since they wrap the old key. Must be one transaction with a pre-flight backup. |
| B1-7 | **Search over secret content** | — | Extend `do_search` (`commands.rs:196`) to decrypt-and-match note bodies and custom-field values | O(n) decrypt per query. Fine at a few hundred items; revisit only if it measurably drags. Never writes a secret anywhere queryable. |

**Per-column touch list** (from the existing code, for each new `sites` column): DDL `vault.rs:52-60`; the new migration helper; the SELECT lists `list_sites` `vault.rs:175-177`, `search` `vault.rs:594`, `list_trash` `vault.rs:522-528`; `Site` struct + its manual `Debug`; `SiteDto` `dto.rs:13-24`; command signatures `commands.rs:239-262`; `src/lib/api.ts`. **Backup: nothing.**

**CSV is the known gap.** `build_csv_rows` (`commands.rs:712-732`) and `portio.rs` carry a fixed 6 fields and already drop custom fields, reprompt and TOTP (`vault.rs:692` hardcodes `&[]` and `false` on import). Favorites, folders, item types and history will also not survive a CSV round-trip. **Document this in the export UI rather than silently losing data** — backup/restore is the lossless path and already is.

---

## Part B2 — Variant scaffolding

| # | Item | Files |
|---|---|---|
| B2-1 | Entry point | `makeui.html`, `src/makeui-main.tsx` — mirror `newui.html` + `src/newui-main.tsx`, **including the vanilla-DOM crash fallback** (`newui-main.tsx:17-93`) that offers a one-click switch back to Classic |
| B2-2 | Bundler | `vite.config.ts:34-42` — 4th rollup input beside `main`/`viewer`/`newui` |
| B2-3 | **Tailwind source glob** | `src/App.css:11-16` — add `@source "../makeui.html";`. The existing `./**/*.tsx` glob already covers `src/makeui/`. ⚠️ Per the warning at `App.css:17-19`, an uncovered source **fails silently** — classes render unstyled with no build error. Easiest thing to forget. |
| B2-4 | Allow the variant | `core/src/config.rs:10` — `VALID_UI_VARIANTS` is `["classic","new"]`; add `"make"` |
| B2-5 | Boot routing | `src-tauri/src/lib.rs:29-49` — add a `"make" => "makeui.html"` arm **keeping the `asset_resolver().get(...)` guard and the Classic fallback**. Never interpolate the config value into a path (T-08-01). |
| B2-6 | Variant test | `core/src/commands.rs:2394` — add a positive case for `"make"` |
| B2-7 | Settings toggle | `src/screens/Settings.tsx:648-667` — it is a `<select>`, not a radio group; add a third option |
| B2-8 | **Bundle the fonts** | `package.json`, makeui stylesheet — see below |
| B2-9 | Web portal | none — `web-src-tauri` serves `dist/` wholesale (`handlers.rs:867`), per D-05 |

### B2-8 — fonts ship in the binary, no CDN

`figma-make/src/index.css:1-2` pulls Inter and JetBrains Mono from the Google Fonts CDN. A local-first offline vault must not fetch fonts at runtime. Install them as npm deps, exactly as `bootstrap-icons` already is (`src/App.css:4`):

```bash
npm install @fontsource/inter @fontsource-variable/jetbrains-mono
```

```css
@import "@fontsource/inter/400.css";
@import "@fontsource/inter/500.css";
@import "@fontsource/inter/600.css";
@import "@fontsource/inter/700.css";
@import "@fontsource-variable/jetbrains-mono";
```

Weights 400/500/600/700 match what the prototype requests. Vite fingerprints the woff2 into `dist/assets/` and Tauri embeds `dist/` in the binary — nothing hosted, no network call, works offline and on machines without Inter installed. Both fonts are SIL OFL; retain the licence files that ship inside the packages.

**Must also do:** delete the two `@import url(...)` lines when porting. Left in, they silently fall back to a system font when offline — and invisibly succeed on any dev machine that happens to have Inter. The offline check in *Verification* exists to catch exactly this.

---

## Part B3 — The UI itself

| # | Item | Notes |
|---|---|---|
| B3-1 | **Token layer first** | `src/makeui/theme.css`, imported only by `src/makeui-main.tsx`. Converts `DARK`/`LIGHT` (`App.tsx:15-30`) into CSS variables layered over the app's existing `.dark` + `data-accent` system (`src/App.css:25-120`). Because the file is loaded only by `makeui.html`, the `indigo` → `#175ddc` retune is automatically scoped and cannot affect Classic or New. |
| B3-2 | Icon module | `src/makeui/icons.tsx` — extract the 30 SVGs from `App.tsx:208-301` verbatim. |
| B3-3 | Shell + the three panes | `Sidebar`, `ItemList`, `DetailPanel` from `App.tsx:438-926`, wired to `src/lib/api.ts`. Replace `VAULT_ITEMS`/`FOLDERS` mocks with real vault reads. Follow `src/newui/hooks/useVault.ts` as the single owner of fetch state. |
| B3-4 | Auth | `App.tsx:1718-1840` + a Username field (D-J). Keep the register screen's "no recovery path" warning (`App.tsx:1788-1795`) — it is correct and the Classic register screen should arguably borrow it. |
| B3-5 | Full-width screens | Health (`App.tsx:946`), Generator (`:1121`), Data Hub (`:1303`), Settings (`:1523`). Each maps onto an existing Classic screen's backend; port the markup, reuse the logic. |
| B3-6 | Add/Edit drawer | `App.tsx:1953-2188`. Wire the Generate button to `api.generatePassword` rather than the prototype's local `Math.random` loop (`App.tsx:1905-1910`). |
| B3-7 | **Reuse the security-critical parts — restyle, never reimplement** | `SecretField` (`src/components/SecretField.tsx:23`), `MasterPasswordPrompt` (`:7`), `TotpDisplay` (`:8`), `src/lib/api.ts`, `src/lib/clipboard.ts`, `src/lib/reprompt.ts`. See the warning below. |
| B3-8 | Responsive | Per D-E, mirror `NewShell.tsx:21-152`. |
| B3-9 | Gap surfaces | Folder CRUD, change-master-password form, live TOTP codes, real history rows, empty-trash. Designed in the prototype's language (D-D). |

> ⚠️ **B3-7 is not optional.** The prototype's `DetailField` (`App.tsx:356-435`) is a mockup: it renders secrets from a plain local `useState`, its reprompt modal accepts *any* non-empty string as the master password (`App.tsx:392`, `:400`), its mask is a fixed 16-bullet literal, and `CopyButton` (`App.tsx:322`) calls `navigator.clipboard` directly with no 30-second clear. The real components enforce the reprompt policy (`reprompt.ts:35`), verify against `api.verifyMasterPassword`, use a length-hiding 8-bullet mask (`SecretField.tsx:95`, D-15), and route copies through `api.copyToClipboard` so both desktop and web get the 30s auto-clear (`clipboard.ts:16-18`). Port the *appearance* of the prototype's field onto the real component. Never the other way round.

---

## Part C — Fidelity verification

Reference screenshots already exist: `figma-make/screenshots/` — 11 surfaces (login, dashboard, history, health, generator, data hub, trash, reprompt, search-active, add-credential, edit-credential), all dark theme.

1. **Capture the light-theme set.** The export will not run as-is: `figma-make/vite.config.ts` imports `./.figma/make/site.json` and four Figma-internal plugins that exist only in Make's sandbox. Run it with a stripped config (react + tailwind plugins and the `@` alias only), `pnpm install && pnpm dev` (the lockfile is `pnpm-lock.yaml`, not npm). Toggle the theme, screenshot the same 11 surfaces into `figma-make/screenshots/light/`.
2. Port, then screenshot the `"make"` variant at the same window sizes.
3. Diff side by side, surface by surface. Target is pixel-close on spacing, colour and type; deviations are allowed only where real data forces them (longer names, more credentials, empty vaults).

---

## Immediate actions on approval (before Sequencing step 1)

1. `git checkout feature/modern-ui-overhaul` — branch **already exists locally** (verified), so no new branch is created. Working-tree changes (`.gitignore`, `.gsd/`, `docs/`, `figma-make/`) are untracked/unstaged and carry over. No commit.
2. Write the plan into the repo as a real doc: `docs/Plans/Modern_UI_Overhaul_Plan_<YYYY-MM-DD>_<HHMM>.md` (timestamp from `date` at write time, per your `name_date_time.md` convention). Replaces the stale `docs/Plans/Modern_UI_Overhaul_Plan.md` — I will **keep the old file untouched** and mark it superseded in its header line only if you say so.
3. Future plan files follow `name_date_time.md`.

---

## Sequencing

| # | Step | Scope |
|---|---|---|
| 1 | Part C step 1 — get the prototype running, capture the light-theme reference set | this branch |
| 2 | B3-1 + B3-2 — token layer and icon module | this branch |
| 3 | B2 — scaffold the variant; prove an empty `makeui.html` boots via `ui_variant: "make"` and that a missing asset falls back to Classic | this branch |
| 4 | B1-0 — `add_site_columns_if_missing`, with a pre-migration-vault test | GSD phase |
| 5 | B1-1 … B1-3 — favorites, folders, item types | GSD phase |
| 6 | B3-3 … B3-6 — the shell, auth, full-width screens, add/edit drawer, against real data | GSD phase |
| 7 | B1-4 + B1-7 — password history, search over secret content | GSD phase |
| 8 | B1-5 + B1-6 — empty-trash-for-sites, then change-master-password **on its own** | GSD phase |
| 9 | B3-8 + B3-9 — responsive, gap surfaces | GSD phase |
| 10 | Part C steps 2-3 — visual diff pass | — |

Steps 1-3 are this branch's immediate work. Steps 4-9 are large enough to run as real GSD phases under `.planning/phases/`.

---

## Verification

- `npm test` — stays green, **including `src/newui/lib/parity.test.ts`** (untouched Phase 8 contract) and the source-text gate `src/components/Spinner.test.ts`
- `npm run build` — `tsc` type-check plus the now-4-input Rollup build
- `cargo test` — `core/` migration tests, **including opening a pre-migration vault file** and confirming the new `sites` columns are added idempotently on a second open
- `cargo test` — a change-master-password round-trip: change it, then reopen and decrypt every sealed blob type (secret, custom fields, TOTP, SSH key), and confirm PIN and biometric records are cleared
- `npm run tauri:dev` — switch `ui_variant` across all three values; confirm each boots, and that a deliberately missing `makeui.html` falls back to Classic instead of a blank window
- **Offline check** — run with the network disabled; confirm no font request is attempted and text renders in Inter
- Manual — unlock → reveal a secret → confirm the reprompt gate fires, the mask stays fixed-width, and a copy clears from the clipboard after 30s
- Manual — backup, then restore into a build with the new columns; confirm the migration repairs the restored vault
- Final — side-by-side screenshot diff against `figma-make/screenshots/` (dark) and `figma-make/screenshots/light/`

---

## Housekeeping

`figma-make/` is currently not gitignored and would be committed. That is probably right — it is the design contract and worth version-controlling. Two notes: its mock data contains realistic-looking sample passwords (fake, but noisy in a password-manager repo, e.g. `App.tsx:125`), and `node_modules/` must stay ignored if anything is ever installed there.

## Reference

- `.planning/phases/08-new-vault-ui-figma-port/8-UI-SPEC.md` — Phase 8 design contract
- `.planning/phases/08-new-vault-ui-figma-port/8-CONTEXT.md` — the D-01..D-18 decisions
- `docs/research/FIGMA_UI_VS_CURRENT_FEATURES.md`, `docs/research/UI_REDESIGN_CAPABILITY_CHECKLIST.md` — also the two files used to seed the Make prompt (`figma-make/src/imports/`)
