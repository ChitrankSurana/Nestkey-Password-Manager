# Phase 8: New Vault UI (Figma Port) - Pattern Map

**Mapped:** 2026-09-13
**Files analyzed:** 24 (13 new frontend files + 11 modified backend/build files)
**Analogs found:** 22 / 24

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `newui.html` | config | request-response (static HTML entry) | `index.html` | exact |
| `src/newui-main.tsx` | provider | request-response (React root mount) | `src/main.tsx` | exact |
| `src/newui/NewApp.tsx` | provider | event-driven (auth gate + vault-locked listener) | `src/App.tsx` | exact |
| `src/newui/NewShell.tsx` | component | request-response (layout/breakpoint owner) | `src/screens/AppShell.tsx` | exact |
| `src/newui/components/Sidebar.tsx` | component | request-response | `src/screens/NavRail.tsx` | exact |
| `src/newui/components/SidebarNavItem.tsx` | component | request-response | `src/screens/NavRail.tsx` (destination button block) | role-match |
| `src/newui/components/ItemList.tsx` | component | CRUD (list+search) | `src/screens/Vault.tsx` | exact |
| `src/newui/components/ItemRow.tsx` | component | request-response | `src/screens/Vault.tsx` (`<li>` row) | role-match |
| `src/newui/components/DetailPanel.tsx` | component | CRUD (site+credentials fetch) | `src/screens/SiteDetail.tsx` | exact |
| `src/newui/components/CredentialBlock.tsx` | component | CRUD | `src/screens/SiteDetail.tsx` (credential `<li>` block, lines 71-119) | exact |
| `src/newui/components/DetailField.tsx` | component | transform (display) | `src/screens/SiteDetail.tsx` (custom-fields render block, lines 103-118) | role-match |
| `src/newui/components/CopyButton.tsx` | component | event-driven | `src/components/SecretField.tsx` (copy handler) | role-match |
| `src/newui/components/PaneSurface.tsx` | component | transform (pure wrapper) | none (new visual primitive) | no analog |
| `src/newui/screens/GeneratorPane.tsx` etc. (5 wrappers) | component | request-response (thin wrapper) | `src/screens/AppShell.tsx` render block (lines 123-138) | exact |
| `src/newui/hooks/useVault.ts` | hook | CRUD (lazy-decryption) | `src/screens/Vault.tsx` (`refreshSites`/`runSearch`) + `src/screens/SiteDetail.tsx` (`refresh`, `Promise.all`) | exact (logic-lift, not copy-paste) |
| `core/src/config.rs` (modify) | config | CRUD (field add) | itself — `mobile_nav_style` pattern (already partially applied) | exact |
| `core/src/dto.rs` (modify) | model | transform | `ConfigDto` struct, lines ~121-131 | exact |
| `core/src/commands.rs` (modify) | service | CRUD | `do_update_mobile_nav_style`, lines 510-520; `do_get_config`, lines 203-217 | exact |
| `src-tauri/src/commands.rs` (modify) | controller | request-response | `update_mobile_nav_style` Tauri command, lines 316-319 | exact |
| `src-tauri/src/lib.rs` (modify) | provider | event-driven (window setup) | `ssh_agent_enabled_at_startup` read + `.setup()` closure, lines 15, 23-56 | exact |
| `src-tauri/tauri.conf.json` (modify) | config | — | `app.windows` array, lines 12-19 | exact |
| `web-src-tauri/src/handlers.rs` (modify) | controller | request-response | `MobileNavStyleRequest`/`update_mobile_nav_style`, lines 165-167, 505-512, 628 | exact |
| `src/lib/api.ts` (modify) | utility | request-response (IPC wrapper) | `mobileNavStyle` field (line 71) + `updateMobileNavStyle` (line 225) | exact |
| `src/screens/Settings.tsx` (modify) | component | CRUD (config form) | its own `mobileNavStyle`/`handleMobileNavStyleChange` block, lines 32,62,276-288,545-561 | exact |
| `src/App.css` (modify) | config | — | `@source` glob list, lines 11-16 | exact |
| `vite.config.ts` (modify) | config | — | `build.rollupOptions.input` second-entry pattern (`viewer`), lines 34-41 | exact |

## Pattern Assignments

### `newui.html` (config, static entry)

**Analog:** `index.html` (full file, 15 lines)

Copy verbatim except title/script src. Keep the anti-flash static class:
```html
<html lang="en" class="dark" data-accent="indigo">
  ...
  <div id="root"></div>
  <script type="module" src="/src/newui-main.tsx"></script>
```
D-16 requires this exact static-class pattern be preserved (do not remove `class="dark" data-accent="indigo"` — it prevents the unstyled flash, and the new shell corrects itself via CSS tokens once `getConfig()` resolves, not via `newui.html` itself).

---

### `src/newui-main.tsx` (provider, mount)

**Analog:** `src/main.tsx` (full file, 10 lines)
```typescript
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
```
Copy 1:1, importing `./newui/NewApp` instead of `./App`.

---

### `src/newui/NewApp.tsx` (provider, auth gate + theme bootstrap)

**Analog:** `src/App.tsx` (full file, 78 lines)

**Theme bootstrap pattern** (lines 19-32):
```typescript
useEffect(() => {
  api
    .getConfig()
    .then((c) => {
      document.documentElement.classList.toggle("dark", c.theme === "dark");
      document.documentElement.setAttribute("data-accent", c.accentColor);
    })
    .catch(() => {
      document.documentElement.classList.add("dark");
      document.documentElement.setAttribute("data-accent", "indigo");
    });
}, []);
```

**vault-locked listener pattern** (lines 34-42):
```typescript
useEffect(() => {
  const unlistenPromise = listen("vault-locked", () => {
    setScreen("login");
    setUsername(null);
  });
  return () => {
    unlistenPromise.then((unlisten) => unlisten());
  };
}, []);
```

**Screen-switch + single ToastHost mount** (lines 44-75): copy the `screen`/`username` state machine and the `<>{content}<ToastHost /></>` wrapper verbatim — CONTEXT.md's code_context note flags `ToastHost`/`toast.ts` as module-level, so this 3rd React root needs its own `<ToastHost/>` instance, exactly like `App.tsx`'s. Also copy the `useActivityWatcher(screen === "vault")` call (line 17) — needed here too since `NewApp` is a separate root from the classic one.

Render `<NewShell>` in place of `<AppShell>` for the `"vault"` branch (line 66).

---

### `src/newui/NewShell.tsx` (component, three-pane + breakpoint owner)

**Analog:** `src/screens/AppShell.tsx` (full file, 153 lines)

**Nav destinations array — copy verbatim, do not extend** (lines 58-65):
```typescript
const destinations: NavDestination[] = [
  { key: "vault", label: "Vault", icon: "house", tooltip: "Your sites and credentials", onClick: () => setScreen("vault") },
  { key: "health", label: "Health", icon: "heart-pulse", tooltip: "Password health dashboard", onClick: () => setScreen("health") },
  { key: "generator", label: "Generator", icon: "dice-5", tooltip: "Generate a password", onClick: () => setScreen("generator") },
  { key: "trash", label: "Trash", icon: "trash", tooltip: "View trash", onClick: () => setScreen("trash") },
  { key: "data", label: "Data", icon: "box-seam", tooltip: "Backup, export, and import", onClick: () => setScreen("data") },
  { key: "settings", label: "Settings", icon: "gear", tooltip: "Open settings", onClick: () => setScreen("settings") },
];
```
This is the UI-SPEC's explicit source for the Sidebar's nav item list (D-10 scope cut — do not add Favorites/Folders/Types/Sync/History).

**Config-driven layout state** (lines 28-36):
```typescript
useEffect(() => {
  api
    .getConfig()
    .then((c) => {
      setCollapsed(c.navCollapsed);
      setMobileNavStyle(c.mobileNavStyle);
    })
    .catch(() => {});
}, []);
```

**Mobile-nav-style branching** (lines 83-108) — copy the `mobileNavStyle === "bottom-tabs"`/`"drawer"`/default icon-rail conditional block verbatim; NewShell must reuse `BottomTabBar`/`NavDrawer` unchanged per D-18/D-11.

**Selection-driven detail pane swap** (lines 141-149) — `AppShell` renders `SiteDetail` as an overlay keyed on `selectedSiteId !== null`; `NewShell` instead renders `DetailPanel` inline in the flex-1 column (no overlay), but the `selectedSiteId !== null` gating idea is the direct analog for the <640px single-visible-pane behavior (D-18).

**Toggle-collapsed persistence pattern** (lines 38-47) — reuse for the Sidebar's collapse button:
```typescript
async function toggleCollapsed() {
  const next = !collapsed;
  setCollapsed(next);
  try {
    await api.updateNavCollapsed(next);
  } catch {
    // best-effort persistence
  }
}
```

---

### `src/newui/components/Sidebar.tsx`, `SidebarNavItem.tsx`

**Analog:** `src/screens/NavRail.tsx` (full file, 74 lines)

**Active-state tint pattern** (lines 52-56) — do NOT use a solid fill; UI-SPEC Color section locks this:
```typescript
className={`rounded px-2 py-1 text-left border-l-2 ${
  active
    ? "bg-[var(--accent-subtle-bg)] text-[var(--accent-subtle-fg)] border-[var(--accent)]"
    : "border-transparent hover:bg-[var(--surface)]"
}`}
```

**Icon + Tooltip composition** (lines 49-61) — every nav item wraps in `<Tooltip text={d.tooltip}>` around a `<button>` containing `<Icon name={d.icon} />`; reuse this exact nesting for `SidebarNavItem`.

**Lock button at footer** (lines 64-70) — reuse `NavRail`'s bottom-pinned `Lock` button pattern (`<div className="flex-1" />` spacer then the lock button) for Sidebar's own footer if D-14's Sidebar needs one; UI-SPEC's header-chip note (icon chip + "Vault"/`{username}` two-line block) is new chrome above this list, not present in `NavRail` — no analog for that block, build fresh per UI-SPEC.

---

### `src/newui/components/ItemList.tsx`, `ItemRow.tsx`

**Analog:** `src/screens/Vault.tsx` (full file, 124 lines)

**Search pattern — real-time, no debounce** (lines 33-46, comment is load-bearing, keep the rationale):
```typescript
async function runSearch(q: string) {
  if (q.trim() === "") {
    setResults(null);
    return;
  }
  try {
    setError(null);
    // Real-time search (TRD NFR-3: results update within one UI frame of keystroke) - no debounce,
    // this is a local IPC call against a SQLite index, not a network request.
    setResults(await api.search(q));
  } catch (err) {
    setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
  }
}
```

**refreshSites + mount/query effects** (lines 24-31, 48-61) — copy this shape into `useVault()` per D-09, ItemList then only reads from the hook, it does not own its own fetch effects.

**Add-site CTA + row list** (lines 63-121):
```typescript
<Tooltip text="Add a new site">
  <button onClick={() => setAddingSite(true)} className="rounded bg-[var(--accent)] px-3 py-1">
    <Icon name="plus-lg" /> Add site
  </button>
</Tooltip>
...
<li
  key={s.id}
  className="cursor-pointer rounded border px-2 py-1 border-[var(--border)]"
  onClick={() => onSelectSite(s.id)}
>
  {s.name}
</li>
{sites.length === 0 && <li className="text-sm opacity-70">No sites yet — add one.</li>}
```
`ItemRow` extends this `<li>` with the `globe2` icon chip + url subtitle per UI-SPEC; `SiteEditor` overlay wiring (lines 112-120) is the analog for ItemList's "Add site" modal trigger.

---

### `src/newui/components/DetailPanel.tsx`, `CredentialBlock.tsx`, `DetailField.tsx`

**Analog:** `src/screens/SiteDetail.tsx` (full file, 153 lines)

**Dual-fetch pattern (lazy-decryption invariant, D-09)** (lines 22-35):
```typescript
async function refresh() {
  try {
    setError(null);
    const [sites, creds] = await Promise.all([api.listSites(), api.listCredentialsForSite(siteId)]);
    setSite(sites.find((s) => s.id === siteId) ?? null);
    setCredentials(creds);
  } catch (err) {
    setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
  }
}

useEffect(() => {
  refresh();
}, [siteId]);
```
Lift this logic into `useVault()`; DetailPanel calls it on selection and must clear credential state on deselect and on `vault-locked` (per D-09 — `SiteDetail.tsx` itself doesn't need to clear-on-deselect since it unmounts, but `DetailPanel` stays mounted in the three-pane shell, so `useVault()` needs an explicit clear branch this analog doesn't have).

**Credential row → CredentialBlock body** (lines 70-119) — the exact fields/order to reproduce per credential:
```typescript
<span className="w-32 truncate">{c.username ?? "(no username)"}</span>
<SecretField secret={c.secret} reprompt={c.reprompt} />
...
{c.hasTotp && (
  <div className="pl-2">
    <TotpDisplay credentialId={c.id} />
  </div>
)}
{c.customFields.length > 0 && (
  <div className="pl-2 space-y-1">
    {c.customFields.map((f, i) => (
      <div key={i} className="flex items-center gap-2 text-sm">
        <span className="w-32 truncate opacity-70">{f.label}</span>
        {f.fieldType === "hidden" ? (
          <SecretField secret={f.value} reprompt={c.reprompt} />
        ) : f.fieldType === "boolean" ? (
          <span>{f.value === "true" ? "Yes" : "No"}</span>
        ) : (
          <span className="flex-1 font-mono">{f.value}</span>
        )}
      </div>
    ))}
  </div>
)}
```
This is the exact `DetailField` branching logic (`hidden`→`SecretField`, `boolean`→Yes/No, `text`→plain) the UI-SPEC's Component Inventory describes — copy it into `DetailField.tsx`, do not reinvent. **Both** the main `secret` and any `hidden` custom field route through `SecretField` — SiteDetail already does both (line 75 and line 109), a detail the CONTEXT.md code_context section explicitly warns is "easy to miss one."

**Delete-credential (no confirm) pattern** (lines 81-96) — copy verbatim, D-16/copywriting contract: immediate action on click, no `window.confirm`:
```typescript
onClick={async () => {
  try {
    setError(null);
    await api.softDeleteCredential(c.id);
    refresh();
  } catch (err) {
    setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
  }
}}
```

**Modal overlay wiring for editors** (lines 124-149) — `CredentialEditor`/`SiteEditor` mount pattern (`existing={editing === "new" ? null : editing}`, `onSaved` calls `refresh()`) — DetailPanel reuses this verbatim per D-11.

---

### `src/newui/hooks/useVault.ts`

**Analogs:** `src/screens/Vault.tsx` lines 24-61 (`refreshSites`/`runSearch`, no-debounce rationale) + `src/screens/SiteDetail.tsx` lines 22-35 (`refresh`, `Promise.all`).

Per D-09: sites load eagerly via `api.listSites()` (no secrets in `SiteDto` — confirmed no `secret` field, see `SiteDto` in `src/lib/api.ts` lines 17-22), credentials load only via `api.listCredentialsForSite(siteId)` on selection, and must be cleared:
1. On deselect (`selectedSiteId` set to `null`)
2. On the `vault-locked` event — reuse `NewApp.tsx`'s `listen("vault-locked", ...)` pattern (see `App.tsx` lines 34-42) inside the hook, or expose a `clearCredentials()` the shell calls from its own listener.

No direct analog exists for a *hook* extracting this (today the logic lives inline in screen components) — this is a **logic-lift, not copy-paste**, per CONTEXT.md's explicit instruction.

---

### `src/newui/screens/*Pane.tsx` (5 thin wrappers)

**Analog:** `src/screens/AppShell.tsx` lines 123-138:
```typescript
{screen === "health" && (
  <HealthDashboard
    onBack={goVault}
    onNavigateToSite={(siteId) => {
      setScreen("vault");
      setSelectedSiteId(siteId);
    }}
  />
)}
{screen === "generator" && <Generator onBack={goVault} />}
{screen === "trash" && <Trash onBack={goVault} />}
{screen === "data" && <Data onBack={goVault} />}
{screen === "settings" && <Settings onBack={goVault} username={username} />}
```
Each `*Pane.tsx` wraps one screen 1:1 with the same `{onBack}` prop threading — these screens already render `flex-1 p-4` internally (Data.tsx/Trash.tsx/Generator.tsx/Settings.tsx/HealthDashboard.tsx per CONTEXT.md code_context), so the wrapper is a one-line passthrough, not new layout.

---

### `core/src/config.rs` (modify) — add `ui_variant` field

**Status:** partially applied already (uncommitted, `git diff` confirms `VALID_UI_VARIANTS` const at line 10 and `default_ui_variant()` fn at lines 52-58 exist, but the field is NOT yet on the `Config` struct, `Default` impl, `load()` snap-to-default block, or either test struct literal).

**Analog:** the existing `mobile_nav_style` pattern in the same file — copy this exact 6-step shape:

1. Const (line 9, already has `ui_variant`'s sibling at line 10):
```rust
pub const VALID_MOBILE_NAV_STYLES: [&str; 3] = ["icon-rail", "bottom-tabs", "drawer"];
```
2. Default fn (lines 40-42):
```rust
fn default_mobile_nav_style() -> String {
    "icon-rail".to_string()
}
```
3. Struct field (lines 76-77):
```rust
#[serde(default = "default_mobile_nav_style")]
pub mobile_nav_style: String,
```
4. `Default` impl entry (line 101): `mobile_nav_style: default_mobile_nav_style(),`
5. Snap-to-default block in `load()` (lines 151-155):
```rust
let parsed = if !VALID_MOBILE_NAV_STYLES.contains(&parsed.mobile_nav_style.as_str()) {
    Config { mobile_nav_style: default_mobile_nav_style(), ..parsed }
} else {
    parsed
};
```
6. Both exhaustive test struct literals at lines 193 and 282 (`Config { ..., mobile_nav_style: default_mobile_nav_style(), ... }`) — `ui_variant` needs the same field added to both.

Also copy the 3 analog tests (lines 316-339): `load_defaults_*_to_*`, `save_then_load_roundtrips_*`, `load_rejects_invalid_*_by_falling_back_to_default`.

---

### `core/src/dto.rs` (modify) — add `ui_variant` to `ConfigDto`

**Analog:** `ConfigDto` struct itself (lines ~121-131) — append `pub ui_variant: String,` following the same `#[serde(rename_all = "camelCase")]` convention (no per-field serde attrs needed, only `Config`'s fields get `#[serde(default = ...)]`, `ConfigDto` fields are plain).

---

### `core/src/commands.rs` (modify) — `do_update_ui_variant` + extend `do_get_config`

**Analog A — setter, copy `do_update_mobile_nav_style` verbatim shape** (lines 510-520):
```rust
pub fn do_update_mobile_nav_style(state: &AppState, paths: &AppPaths, style: &str) -> Result<(), CommandError> {
    if !VALID_MOBILE_NAV_STYLES.contains(&style) {
        return Err(crate::error::invalid_input_error("Unknown mobile nav style."));
    }
    let mut config = state.config.lock().unwrap();
    let mut updated = config.clone();
    updated.mobile_nav_style = style.to_string();
    updated.save(&paths.config_path()).map_err(|_| crate::error::internal_error())?;
    *config = updated;
    Ok(())
}
```
Write `do_update_ui_variant(state, paths, variant: &str)` validating against `VALID_UI_VARIANTS`.

**Analog B — extend `do_get_config`** (lines 203-217): add `ui_variant: config.ui_variant.clone(),` to the `ConfigDto { ... }` literal, matching the existing field-per-line convention.

Also copy the 3 analog tests at lines 1373-1393 (`*_persists_and_rejects_invalid_value`, `get_config_includes_*_default`, `*_does_not_mutate_state_when_save_fails`).

---

### `src-tauri/src/commands.rs` (modify) — `update_ui_variant` Tauri command

**Analog:** `update_mobile_nav_style`, lines 316-319:
```rust
#[tauri::command]
pub fn update_mobile_nav_style(state: State<AppState>, paths: State<AppPaths>, style: String) -> Result<(), CommandError> {
    do_update_mobile_nav_style(&state, &paths, &style)
}
```
Also add the import at line 13 (`do_update_mobile_nav_style, do_update_nav_collapsed, ...` → append `do_update_ui_variant`).

---

### `src-tauri/src/lib.rs` (modify) — read `ui_variant` at boot, build window explicitly

**Analog A — boot-time config read** (line 15):
```rust
let ssh_agent_enabled_at_startup = app_state.config.lock().unwrap().ssh_agent_enabled;
```
Add `let ui_variant = app_state.config.lock().unwrap().ui_variant.clone();` right after (per D-03/CONTEXT.md canonical_refs), before `tauri::Builder::default()`.

**Analog B — `.setup()` closure structure** (lines 23-56) — window-building code goes here or independently; existing closure already does async setup (ssh agent start, autolock tick spawn) via `app.handle()`. Use `app.get_webview_window` / `tauri::WebviewWindowBuilder::new(app, "main", WebviewUrl::App(...))` — build `index.html`/`newui.html` path off `ui_variant`, window label MUST stay `"main"` (D-04, no analog needed, this is a hard constraint from `src-tauri/capabilities/default.json`).

**Command registration list pattern** (lines 57-112) — append `commands::update_ui_variant,` following the existing one-per-line style, e.g. right after `commands::update_mobile_nav_style,` (line 100).

---

### `src-tauri/tauri.conf.json` (modify)

**Analog:** its own `app.windows` array, lines 12-19:
```json
"windows": [
  {
    "title": "tauri-app",
    "width": 800,
    "height": 600
  }
]
```
Per D-03, change to `"windows": []` and move title/width/height into the explicit `WebviewWindowBuilder` call in `lib.rs`'s `.setup()`.

---

### `web-src-tauri/src/handlers.rs` (modify) — `update_ui_variant` route

**Analog — request struct + handler + route registration, copy `MobileNavStyleRequest`/`update_mobile_nav_style` verbatim shape:**

Request struct (lines 165-167):
```rust
#[derive(Deserialize)]
struct MobileNavStyleRequest {
    style: String,
}
```

Handler (lines 505-512):
```rust
async fn update_mobile_nav_style(
    Extension(s): Extension<Arc<UserSession>>,
    State(ctx): State<AppContext>,
    Json(b): Json<MobileNavStyleRequest>,
) -> Result<StatusCode, ApiError> {
    do_update_mobile_nav_style(&s.app_state, &ctx.paths, &b.style)?;
    Ok(StatusCode::OK)
}
```

Route registration (line 628):
```rust
.route("/api/update_mobile_nav_style", post(update_mobile_nav_style))
```
Per D-05, register `update_ui_variant` here too even though the web portal doesn't act on it locally — only the desktop's `.setup()` reads it at boot.

---

### `src/lib/api.ts` (modify) — `uiVariant` field + `updateUiVariant` wrapper

**Analog — `mobileNavStyle` field + wrapper, copy shape exactly:**

Field (line 71, inside the `ConfigDto`-mirroring interface): `mobileNavStyle: string;` → add `uiVariant: string;`

Wrapper (line 225):
```typescript
updateMobileNavStyle: (style: string) => call<void>("update_mobile_nav_style", { style }),
```
→ add `updateUiVariant: (variant: string) => call<void>("update_ui_variant", { variant }),`

`call<T>()` already handles the dual transport (`isTauri ? invoke() : httpPost`, per D-12/CONTEXT.md canonical_refs) — no new IPC plumbing needed.

---

### `src/screens/Settings.tsx` (modify) — new "Interface" section

**Analog A — state + boot-value capture, copy `mobileNavStyle` state slice shape** (lines 32, 62):
```typescript
const [mobileNavStyle, setMobileNavStyle] = useState<string | null>(null);
...
setMobileNavStyle(c.mobileNavStyle);
```
Add `uiVariant` state the same way, PLUS a **separate** ref for the boot-value capture (NOT `editedRef` — CONTEXT.md code_context explicitly flags this: the banner's comparison semantics are boot-value-vs-current-saved, not edited-vs-not). E.g. `const bootUiVariantRef = useRef<string | null>(null);` set once inside the `getConfig().then()` callback's first run.

**Analog B — setter handler shape, copy `handleMobileNavStyleChange`** (lines 276-288):
```typescript
async function handleMobileNavStyleChange(value: string) {
  editedRef.current = true;
  setSubmitting(true);
  try {
    setError(null);
    await api.updateMobileNavStyle(value);
    setMobileNavStyle(value);
  } catch (err) {
    setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
  } finally {
    setSubmitting(false);
  }
}
```
Write `handleUiVariantChange` the same way (still sets `editedRef.current = true` — that ref guards against `getConfig()` clobbering an in-flight edit, unrelated to the banner ref).

**Analog C — `<details>` section + `<select>` markup, copy the "Mobile navigation style" control verbatim shape** (lines 545-561):
```typescript
<label className="block">
  <span className="text-sm">Mobile navigation style</span>
  <Tooltip text="How the sidebar is replaced on narrow screens/windows below 640px" className="w-full">
    <select
      className="mt-1 w-full rounded border px-2 py-1 bg-[var(--surface)] border-[var(--border)] disabled:opacity-50"
      value={mobileNavStyle ?? ""}
      onChange={(e) => handleMobileNavStyleChange(e.target.value)}
      disabled={submitting}
    >
      {MOBILE_NAV_STYLE_OPTIONS.map((o) => (
        <option key={o.value} value={o.value}>{o.label}</option>
      ))}
    </select>
  </Tooltip>
</label>
```
New "Interface" `<details open>` section goes after Appearance (lines 509-563), same `max-w-xs mt-4` wrapper pattern (line 509). Options array follows the `MOBILE_NAV_STYLE_OPTIONS`/`ACCENT_OPTIONS` const shape (lines 11-22):
```typescript
const UI_VARIANT_OPTIONS = [
  { value: "classic", label: "Classic" },
  { value: "new", label: "New" },
] as const;
```

**Banner — no existing analog** (this project has no persistent-banner component today; `ToastHost` auto-dismisses in 2000ms per D-07, explicitly NOT reusable here). Build new: a conditional block comparing `bootUiVariantRef.current` to the current `uiVariant` state, rendered above/below the Interface `<details>`, using the `text-sm text-red-400`-style convention seen at every other inline error (`error &&` blocks throughout this file, e.g. line 299) as the closest visual precedent for an inline persistent message, but non-error styling (this is informational, not an error — no exact color token specified by UI-SPEC beyond "persistent banner, not a toast"; Claude's Discretion per CONTEXT.md).

---

### `src/App.css` (modify) — add `@source` line for `newui.html`

**Analog:** existing `@source` list, lines 11-16:
```css
@source "../index.html";
@source "./**/*.js";
@source "./**/*.ts";
@source "./**/*.jsx";
@source "./**/*.tsx";
/* Adding a new source file extension under src/? Add a matching @source line above -
```
Add `@source "../newui.html";` immediately after line 11 — required per CONTEXT.md canonical_refs: `newui.html` sits outside `src/`, so without this line its classes render unstyled with no build error (a silent failure mode, not a compile error — verify visually after adding).

---

### `vite.config.ts` (modify) — 3rd entry point

**Analog:** the existing 2nd-entry-point pattern (`viewer`), lines 34-41:
```typescript
build: {
  rollupOptions: {
    input: {
      main: fileURLToPath(new URL("./index.html", import.meta.url)),
      viewer: fileURLToPath(new URL("./viewer.html", import.meta.url)),
    },
  },
},
```
Add `newui: fileURLToPath(new URL("./newui.html", import.meta.url)),` as a 3rd key, same `fileURLToPath(new URL(...))` shape.

---

## Shared Patterns

### Error handling — every API call site
**Source:** `src/screens/Vault.tsx` lines 28-30, 43-45; `src/screens/SiteDetail.tsx` lines 28-30; `src/screens/Settings.tsx` (every handler, e.g. lines 86-90)
```typescript
} catch (err) {
  setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
}
```
**Apply to:** every new component/hook in `src/newui/**` that calls `api.*` — this is the copywriting-contract-locked generic fallback (UI-SPEC Copywriting Contract table).

### Icon + Tooltip composition
**Source:** `src/screens/NavRail.tsx` lines 49-61, `src/screens/Vault.tsx` lines 67-71
```typescript
<Tooltip text="...">
  <button onClick={...} className="...">
    <Icon name="plus-lg" /> Add site
  </button>
</Tooltip>
```
**Apply to:** every icon-only or ambiguous control in `src/newui/**` (UX-01 requirement in UI-SPEC).

### Config field add (backend, 6-step + tests)
**Source:** `core/src/config.rs` full `mobile_nav_style` trail (const, default fn, struct field, `Default` impl, `load()` snap, 2 test literals, 3 tests) — see `core/src/config.rs` (modify) section above for full excerpts.
**Apply to:** `ui_variant` field add across `core/src/config.rs`, `core/src/dto.rs`, `core/src/commands.rs`, `src-tauri/src/commands.rs`, `web-src-tauri/src/handlers.rs`, `src/lib/api.ts`.

### `{onBack}`-taking secondary screen wrapper
**Source:** `src/screens/AppShell.tsx` lines 123-138 (render dispatch) + the screens themselves already accepting `{ onBack }` (confirmed by CONTEXT.md code_context: `Data.tsx`, `Trash.tsx`, `Generator.tsx`, `Settings.tsx`, `HealthDashboard.tsx` already take this prop and render `flex-1 p-4`).
**Apply to:** all 5 `src/newui/screens/*Pane.tsx` wrappers.

### Modal overlay (`fixed inset-0 z-50`)
**Source:** `src/screens/SiteDetail.tsx` line 38 (`<div className="fixed inset-0 z-50 bg-black/50 flex items-center justify-center p-4">`)
**Apply to:** any place `SiteEditor`/`CredentialEditor` mount inside `DetailPanel` — reused unchanged per D-11, floats above the three-pane shell with no layout conflict (confirmed no z-index collision, `NewShell`'s panes are not `fixed`).

## No Analog Found

| File | Role | Data Flow | Reason |
|---|---|---|---|
| `src/newui/components/PaneSurface.tsx` | component | transform (pure wrapper) | No existing shared card/surface primitive in this codebase — every screen inlines its own `rounded border ... border-[var(--border)]` classes rather than factoring a wrapper component. Build fresh per UI-SPEC's `var(--surface)`/`var(--border)`/`rounded-lg` spec; closest inline precedent is the repeated `className="rounded border px-2 py-1 border-[var(--border)]"` seen throughout `Vault.tsx`/`SiteDetail.tsx`, but no componentized analog exists. |
| Settings "Interface" persistent banner (inline, not a separate file) | component | event-driven | No persistent-banner UI exists today; `ToastHost` (auto-dismiss) is explicitly the wrong analog per D-07. Build new using the inline-error `text-sm` styling convention as the only available visual precedent (see Pattern Assignments > Settings.tsx above). |

## Metadata

**Analog search scope:** `src/`, `src/screens/`, `src/components/`, `src/lib/`, `core/src/`, `src-tauri/src/`, `web-src-tauri/src/`, plus config files (`vite.config.ts`, `tauri.conf.json`, `App.css`) at repo root.
**Files scanned:** 24 existing source/config files read directly (full-file reads for all files ≤ 590 lines; targeted grep+read for `core/src/config.rs`, `core/src/commands.rs`, `core/src/dto.rs`, `src-tauri/src/commands.rs`, `web-src-tauri/src/handlers.rs` which exceed convenient full-read size).
**Pattern extraction date:** 2026-09-13
**Note on `core/src/config.rs` state:** `git status`/`git diff` confirm this file has *uncommitted, partial* work already in place (`VALID_UI_VARIANTS` const + `default_ui_variant()` fn exist; struct field, `Default` impl entry, `load()` snap block, and 2 test struct literals do NOT exist yet). The executor should treat the const/fn as already-done and complete only the remaining 4 steps of the 6-step pattern.
