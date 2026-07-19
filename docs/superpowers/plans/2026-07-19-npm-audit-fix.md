# npm audit fix (vitest 2 → 4) — Implementation Plan

**Goal:** Resolve the `npm audit` esbuild advisory (GHSA-67mh-4wv8-2f99) by upgrading `vitest`
2.1.9 → 4.1.10, without breaking the test suite or the production build pipeline.

**Background:** the vulnerable `esbuild` is nested entirely inside `vitest`'s own bundled `vite@5.4.21`
(`vite-node`, `@vitest/mocker`) — the top-level `vite@7.3.6` used by `npm run build`/`npm run dev`
already resolves to a non-vulnerable `esbuild@0.28.1` and is unaffected. Real exposure is low (this
project's `test` script is `vitest run`, headless, never opens the vulnerable watch/UI dev server),
but the fix is still worth doing cleanly rather than leaving it. Full analysis: see conversation
history 2026-07-19, "npm audit report" discussion.

**Deliberately NOT in this plan:** `@vitejs/plugin-react` 4.7.0→6.0.3, `typescript` 5.8.3→7.0.2,
`vite` 7.3.6→8.1.5 — separate `npm outdated` items, unrelated to this advisory, not bundled in here.

---

## Before you start

- PowerShell for `npm`/`cargo`. Prefix every such command:
  ```powershell
  $env:PATH = "$env:USERPROFILE\.cargo\bin;C:\Strawberry\perl\bin;C:\Strawberry\c\bin;" + $env:PATH
  ```
- Confirm a clean working tree before touching the lockfile:
  ```powershell
  git status
  ```
  If not clean, `git stash -u` first — this plan's diff should be isolated to the dependency bump.

---

## Task 1 — Apply the fix

- [ ] `npm audit fix --force` (rewrites `package.json`'s `vitest` entry to `^4.1.10`, regenerates
      `package-lock.json`)
- [ ] `git diff package.json package-lock.json` — confirm only `vitest` (and its own transitive
      deps) changed, nothing unrelated got dragged along

## Task 2 — Check for breaking-change fallout

Known v2→v4 risk areas in this repo, check both explicitly:

- [ ] Read `vitest.config.ts` — v3 changed default test-environment resolution and pool options; v4
      may warn/error on anything using a removed default
- [ ] Read `src/lib/clipboard.test.ts` (only test file in the repo) — check for any removed/renamed
      globals or changed matcher/mock defaults
- [ ] Watch `npm audit fix`'s own output for a peer-dependency warning on `@vitejs/plugin-react`
      (currently pinned 4.7.0) — vitest 4 may want a newer peer

## Task 3 — Verify

- [ ] `npm test` — must pass, 0 failures
- [ ] `npm run build` — `tsc` + `vite build` must still succeed cleanly (confirms the upgrade didn't
      leak into the production build pipeline, which stays on the unrelated top-level `vite@7.3.6`)
- [ ] `cargo test --workspace` — unrelated to this change, cheap sanity check that nothing else
      regressed in the same session

## Task 4 — Confirm the advisory is resolved

- [ ] `npm audit` — expect `found 0 vulnerabilities`

## Task 5 — Commit

- [ ] `git add package.json package-lock.json` (plus any test/config file touched in Task 2)
- [ ] Conventional-commit message, e.g.:
  ```
  fix(deps): upgrade vitest 2.1.9 -> 4.1.10, resolve esbuild dev-server advisory

  GHSA-67mh-4wv8-2f99 was nested in vitest's own bundled vite-node/@vitest/mocker
  esbuild, unrelated to the top-level vite used by the production build.
  ```

---

## Rollback (if Task 2/3 turns up a break that isn't a quick fix)

```powershell
git checkout -- package.json package-lock.json
npm install
```

Back to vitest 2.1.9. The vulnerability is dev-tooling-only and low real exposure (per Background
above) — reverting is safe to leave in place until the breaking change is worth chasing down
separately, not an emergency.
