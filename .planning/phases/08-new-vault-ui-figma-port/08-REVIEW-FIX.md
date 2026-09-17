---
phase: 08-new-vault-ui-figma-port
fixed_at: 2026-09-14T17:10:00Z
review_path: .planning/phases/08-new-vault-ui-figma-port/08-REVIEW.md
iteration: 1
findings_in_scope: 3
fixed: 3
skipped: 0
status: all_fixed
---

# Phase 8: Code Review Fix Report

**Fixed at:** 2026-09-14T17:10:00Z
**Source review:** .planning/phases/08-new-vault-ui-figma-port/08-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 3 (fix_scope: critical_warning — 2 critical, 1 warning)
- Fixed: 3
- Skipped: 0

**Note on scope count:** REVIEW.md's YAML frontmatter reports `warning: 2`, but the document body
contains exactly one `### WR-` heading (WR-01). Findings were parsed from the document body per
the finding_parser rules (headings are the source of truth), so only WR-01 was in scope for the
warning tier. Info-level findings (IN-01, IN-02) were intentionally left out of scope per
`fix_scope: critical_warning`.

## Fixed Issues

### CR-01: Switching sites can display the previous site's decrypted credentials under the new site's header

**Files modified:** `src/newui/hooks/useVault.ts`
**Commit:** b79213b
**Applied fix:** Added a `latestRequestedSiteId` ref that `refreshCredentials` sets on entry and
checks before committing `setCredentials`/`setError`/`setCredentialsLoading(false)`, so a
resolved response for a site that is no longer selected is discarded. Also added an unconditional
`clearCredentials()` call in the site-selection effect before `refreshCredentials(selectedSiteId)`
so the previous site's plaintext credentials are never rendered under the newly selected site's
header while the new fetch is in flight.

### CR-02: Global `unhandledrejection`/`error` listeners stay armed for the whole session and will tear down the entire vault UI on any unrelated, recoverable runtime error

**Files modified:** `src/newui-main.tsx`, `src/newui/components/CopyButton.tsx`
**Commit:** e0e1f2d
**Applied fix:** Added a `bootstrapped` flag in `newui-main.tsx` that starts `false` and flips to
`true` immediately after `ReactDOM.createRoot(...).render(...)` succeeds. Both the `error` and
`unhandledrejection` window listeners now return early (`if (bootstrapped) return;`) once
bootstrap has completed, so they only ever fire for genuine import/first-mount failures and no
longer tear down the whole vault UI on a later, unrelated, recoverable runtime error. Separately,
`CopyButton.tsx`'s fire-and-forget `api.copyToClipboard(text)` call (a likely source of exactly
such an unrelated rejection) now has an explicit `.then()`/`.catch()` chain that toasts a
recoverable "Couldn't copy to the clipboard" message instead of throwing an unhandled rejection.

### WR-01: Search results have no request sequencing and can render out of order

**Files modified:** `src/newui/hooks/useVault.ts`
**Commit:** 970a975
**Applied fix:** Added a `latestQuery` ref that `runSearch` sets on entry; the resolved
`api.search(q)` response (and any error from it) is only committed via `setResults`/`setError` if
`latestQuery.current === q` still holds at resolution time, so a stale, out-of-order response can
no longer overwrite results for the query currently in the search box.

## Skipped Issues

None — all in-scope findings were fixed.

---

_Fixed: 2026-09-14T17:10:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
