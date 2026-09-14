---
phase: 08-new-vault-ui-figma-port
reviewed: 2026-09-14T00:00:00Z
depth: standard
files_reviewed: 35
files_reviewed_list:
  - core/src/commands.rs
  - core/src/config.rs
  - core/src/dto.rs
  - newui.html
  - src-tauri/src/commands.rs
  - src-tauri/src/lib.rs
  - src-tauri/tauri.conf.json
  - src/App.css
  - src/lib/api.ts
  - src/newui-main.tsx
  - src/newui/NewApp.tsx
  - src/newui/NewShell.tsx
  - src/newui/components/CopyButton.tsx
  - src/newui/components/CredentialBlock.tsx
  - src/newui/components/DetailField.tsx
  - src/newui/components/DetailPanel.tsx
  - src/newui/components/ItemList.tsx
  - src/newui/components/ItemRow.tsx
  - src/newui/components/PaneSurface.tsx
  - src/newui/components/Sidebar.tsx
  - src/newui/components/SidebarNavItem.tsx
  - src/newui/hooks/useVault.ts
  - src/newui/lib/credentialLayout.test.ts
  - src/newui/lib/credentialLayout.ts
  - src/newui/lib/parity.test.ts
  - src/newui/lib/vaultSelectors.test.ts
  - src/newui/lib/vaultSelectors.ts
  - src/newui/screens/DataPane.tsx
  - src/newui/screens/GeneratorPane.tsx
  - src/newui/screens/HealthPane.tsx
  - src/newui/screens/SettingsPane.tsx
  - src/newui/screens/TrashPane.tsx
  - src/screens/Settings.tsx
  - vite.config.ts
  - web-src-tauri/src/handlers.rs
findings:
  critical: 2
  warning: 2
  info: 2
  total: 6
status: issues_found
---

# Phase 8: Code Review Report

**Reviewed:** 2026-09-14T00:00:00Z
**Depth:** standard
**Files Reviewed:** 35
**Status:** issues_found

## Summary

This phase ports a Figma-designed vault UI (`src/newui/`) that runs alongside the existing
Classic UI, plumbing a new `ui_variant` config field end-to-end (core → Tauri commands → web
handlers → frontend) and adding a `newui.html` Vite entry point selected at window-build time.

The four named security invariants for this phase hold up well and are backed by a standing
`parity.test.ts` gate that greps the New UI's own sources on every test run:
- **No direct Tauri core IPC import** under `src/newui/` — confirmed; the only
  `@tauri-apps/api/*` import under that directory is the allowed `@tauri-apps/api/event` listener
  module. `src/newui-main.tsx` does import `@tauri-apps/api/core` directly, but it lives outside
  `src/newui/` and is a documented, deliberate exception (the fallback must not depend on code
  that might itself be broken).
- **No direct clipboard writes** — every copy path in the reviewed files (`CopyButton.tsx`,
  `DetailField.tsx` via `SecretField`) routes through `api.copyToClipboard`, never
  `navigator.clipboard` directly.
- **Lazy credential decryption** — `listCredentialsForSite` is called exactly once, in
  `src/newui/hooks/useVault.ts`; no other file under `src/newui/` calls it.
- **Reprompt gate on secret reveal/copy, including hidden custom fields** —
  `DetailField.tsx` branches on `fieldType === "hidden"` and delegates to `SecretField` with the
  credential's `reprompt` flag, exactly mirroring the Classic UI's `SiteDetail.tsx`.

Backend plumbing for `ui_variant` (`core/src/config.rs`, `core/src/commands.rs`,
`src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`, `web-src-tauri/src/handlers.rs`) is
consistent and well-tested: the value is validated against a fixed allowlist server-side before
ever being used to select a compiled-in window entry (`match ui_variant.as_str() { "new" => ...,
_ => "index.html" }`), so there is no path-interpolation risk from a hand-edited config file.

However, two BLOCKER-level correctness bugs were found in the new React code, both with
security-relevant consequences for a password manager (wrong-site credential display, and a
trivial recoverable error nuking the whole vault UI), plus two lower-severity issues.

## Critical Issues

### CR-01: Switching sites can display the previous site's decrypted credentials under the new site's header

**File:** `src/newui/hooks/useVault.ts:85-91`
**Issue:**

```ts
useEffect(() => {
  if (selectedSiteId === null) {
    clearCredentials();
    return;
  }
  refreshCredentials(selectedSiteId);
}, [selectedSiteId, refreshCredentials, clearCredentials]);
```

When `selectedSiteId` changes from one non-null site to another non-null site (the normal case of
clicking a different row in `ItemList`), `credentials` is **not** cleared before the new fetch
starts. `DetailPanel.renderBody()` only shows a loading state when `credentials === null`:

```ts
if (credentialsLoading && credentials === null) {
  return <p className="text-sm opacity-70">Loading credentials…</p>;
}
```

Since `credentials` still holds the *previous* site's array while the new site's
`api.listCredentialsForSite` call is in flight, `DetailPanel` renders the **previously selected
site's plaintext username/password/custom fields under the newly selected site's name and URL**
for the duration of the round trip. There is also no request sequencing/cancellation
(`AbortController` or a "latest requested site" guard), so if two requests resolve out of order
(e.g. the user clicks site A then quickly clicks site B, and A's response lands after B's), the
mismatch is not self-correcting — the UI can settle on site A's credentials permanently displayed
under site B's header. In a password manager, a user could reasonably copy or trust a secret they
believe belongs to the currently displayed site when it actually belongs to a different one.

**Fix:** Clear credentials synchronously before starting the new fetch, and guard against
out-of-order responses by only committing a response if it still matches the currently selected
site:

```ts
const latestRequestedSiteId = useRef<number | null>(null);

const refreshCredentials = useCallback(async (siteId: number) => {
  latestRequestedSiteId.current = siteId;
  try {
    setError(null);
    setCredentialsLoading(true);
    const creds = await api.listCredentialsForSite(siteId);
    if (latestRequestedSiteId.current === siteId) {
      setCredentials(creds);
    }
  } catch (err) {
    if (latestRequestedSiteId.current === siteId) {
      setError(isCommandError(err) ? err.message : FALLBACK_ERROR);
    }
  } finally {
    if (latestRequestedSiteId.current === siteId) {
      setCredentialsLoading(false);
    }
  }
}, []);

useEffect(() => {
  if (selectedSiteId === null) {
    clearCredentials();
    return;
  }
  clearCredentials(); // never show a stale site's credentials under the new site's header
  refreshCredentials(selectedSiteId);
}, [selectedSiteId, refreshCredentials, clearCredentials]);
```

### CR-02: Global `unhandledrejection`/`error` listeners stay armed for the whole session and will tear down the entire vault UI on any unrelated, recoverable runtime error

**File:** `src/newui-main.tsx:68-74`, `src/newui/components/CopyButton.tsx:14-19`
**Issue:** `newui-main.tsx` registers permanent, page-lifetime listeners:

```ts
window.addEventListener("error", (e) => {
  showClassicFallback(e.message || "An unexpected error occurred.");
});
window.addEventListener("unhandledrejection", (e) => {
  const reason = e.reason;
  showClassicFallback(reason instanceof Error ? reason.message : "An unexpected error occurred.");
});
```

The surrounding comment frames this as a startup/import/first-render safety net ("it also catches
errors thrown while importing NewApp.tsx... not just errors during React's render/commit
phases"), but the listeners are never removed after a successful mount — they remain active for
the entire session. Any later, unrelated, non-fatal error anywhere in the running app — including
an ordinary unhandled promise rejection — will call `showClassicFallback()`, which wipes
`#root` and replaces the whole vault UI with a "Something went wrong" screen and a forced
"Switch to Classic UI" button, discarding all in-memory state (open site, unsaved edits, etc.).

This combines badly with several fire-and-forget clipboard calls that have no `.catch()`,
including one added by this phase:

```ts
// src/newui/components/CopyButton.tsx
onClick={() => {
  api.copyToClipboard(text);           // returns a Promise, not awaited, no .catch
  toast("Copied. It will clear from the clipboard in 30s.");
}}
```

(`src/components/SecretField.tsx`'s reused copy handler and `src/components/TotpDisplay.tsx` have
the same unawaited pattern.) `api.copyToClipboard` can reject for mundane, transient reasons
(OS clipboard temporarily locked by another process, a denied clipboard permission in web mode,
an IPC hiccup) that are not fatal to the app in any way under the Classic UI (`src/main.tsx` has
no such global listeners at all). Under the New UI, the exact same transient failure now
force-crashes the whole vault view and pushes the user toward abandoning the New UI entirely —
a severe reliability regression introduced specifically by this phase's bootstrap file.

**Fix:** Scope the safety net to genuine bootstrap failures instead of the whole session, e.g.
track whether the initial mount succeeded and ignore later events, and/or use a React error
boundary (which naturally only catches render/commit-phase errors within its subtree) for
post-mount issues:

```ts
let bootstrapped = false;

window.addEventListener("error", (e) => {
  if (bootstrapped) return;
  showClassicFallback(e.message || "An unexpected error occurred.");
});
window.addEventListener("unhandledrejection", (e) => {
  if (bootstrapped) return;
  const reason = e.reason;
  showClassicFallback(reason instanceof Error ? reason.message : "An unexpected error occurred.");
});

try {
  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(/* ... */);
  bootstrapped = true;
} catch (e) {
  showClassicFallback(e instanceof Error ? e.message : "An unexpected error occurred.");
}
```

Separately, `CopyButton.tsx` (and ideally the reused `SecretField.tsx`/`TotpDisplay.tsx`) should
handle the rejection explicitly instead of leaving it unhandled:

```ts
onClick={() => {
  api.copyToClipboard(text)
    .then(() => toast("Copied. It will clear from the clipboard in 30s."))
    .catch(() => toast("Couldn't copy to the clipboard. Please try again."));
}}
```

## Warnings

### WR-01: Search results have no request sequencing and can render out of order

**File:** `src/newui/hooks/useVault.ts:36-49`
**Issue:**

```ts
const runSearch = useCallback(async (q: string) => {
  if (q.trim() === "") {
    setResults(null);
    return;
  }
  try {
    setError(null);
    setResults(await api.search(q));
  } catch (err) {
    setError(isCommandError(err) ? err.message : FALLBACK_ERROR);
  }
}, []);

useEffect(() => {
  runSearch(query);
}, [query, runSearch]);
```

Every keystroke fires a new `api.search(q)` call with no cancellation and no sequence guard. If
an earlier query's response arrives after a later query's response (plausible for an IPC/HTTP
call under load, or in the web-portal transport), `results` will briefly or permanently reflect a
stale query instead of the text currently in the search box. Lower severity than CR-01 (no
secrets involved, only the site list), but the same missing-guard pattern.

**Fix:** Apply the same "ignore stale responses" guard as suggested for CR-01, keyed on the query
string or a monotonic request counter, e.g. only call `setResults` if `q === latestQuery.current`
at resolution time.

## Info

### IN-01: `DetailPanel`'s `error` prop is permanently hard-coded to `null`, making its error branch dead code

**File:** `src/newui/NewShell.tsx:187`, `src/newui/components/DetailPanel.tsx:184`
**Issue:** `DetailPanel` accepts and renders an `error: string | null` prop
(`{error && <p className="mb-4 text-sm text-red-400">{error}</p>}`), but its only call site
passes a literal `null`:

```tsx
<DetailPanel
  site={vault.selectedSite}
  credentials={vault.credentials}
  credentialsLoading={vault.credentialsLoading}
  error={null}
  ...
```

`useVault`'s `error` state (set when `refreshCredentials` fails) is never threaded through, so
this branch can never render. The user still sees credential-fetch errors because `ItemList`
independently renders the same shared `error` state, but that means a failure that happened while
loading a specific site's credentials surfaces in the site-list pane rather than the detail pane
that actually failed to load — and `DetailPanel`'s documented `error` prop is unreachable dead
code as written.

**Fix:** Either wire `vault.error` through when a site is selected (`error={vault.selectedSiteId !== null ? vault.error : null}`), or drop the unused prop from `DetailPanel`'s contract if the
list-pane error banner is considered sufficient.

### IN-02: `DetailField`'s `copyable` prop is declared but never passed, making its `CopyButton` path unreachable

**File:** `src/newui/components/DetailField.tsx:17,23,37`
**Issue:**

```tsx
export function DetailField({ label, value, fieldType, reprompt, copyable }: {
  ...
  copyable?: boolean;
}) {
  ...
  <span className="flex-1 break-all font-mono">{value}</span>
  {copyable && <CopyButton text={value} tooltip={`Copy ${label}`} />}
```

`CredentialBlock.tsx` (the sole caller) never passes `copyable`, so it is always `undefined` and
the `CopyButton` branch for plain-text custom fields is unreachable in the shipped UI. This
matches the Classic UI's `SiteDetail.tsx`, which also renders plain-text custom fields with no
copy affordance, so this isn't a functional regression — but it is a speculative, unused prop
that adds API surface and reader confusion for no current behavior.

**Fix:** Either remove the unused `copyable` prop until a caller needs it, or intentionally wire
it up (`<DetailField ... copyable />`) for text-type custom fields in `CredentialBlock.tsx` to
give parity with the copy affordance already offered for the primary username/URL fields.

---

_Reviewed: 2026-09-14T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
