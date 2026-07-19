# Repo vs. Local Folder Comparison — 2026-07-19

Snapshot taken after pushing branch `docs/security-and-setup-audit` (PR #7). Compares this local
clone against `origin` (`github-account-1:ChitrankSurana/Password_Manager_Local.git`).

## 1. Current branch — in sync

`docs/security-and-setup-audit` (local commit `3c52e35`) matches `origin/docs/security-and-setup-audit`
exactly. Working tree clean, nothing untracked, nothing staged.

## 2. Difference: PR #7 not yet merged into `main`

The 4 files just pushed exist **only on `docs/security-and-setup-audit`**, not yet on `origin/main`:
- `SECURITY_CONCERNS.md`
- `docs/PATH_AUDIT.md`
- `docs/FRESH_MACHINE_SETUP.md`
- `scripts/setup-dependencies.ps1`

Local `main` branch is up to date with `origin/main` (`20cac43`) — but `origin/main` itself won't
have these files until PR #7 is merged. This is expected PR-flow state, not a bug — flagging it since
"repo vs local" comparisons often assume `main` is the reference point, and right now `main` (both
local and origin) is one PR behind what's visible on GitHub as a whole.

## 3. Difference: `m6-responsive-layout` worktree branch is behind origin

```
m6-responsive-layout f96b888 (.worktrees/m6-responsive-layout) [origin/m6-responsive-layout: behind 2]
```

Local copy is **missing 2 commits** that exist on `origin/m6-responsive-layout`:
- `31fc865` — docs: pause session - M6 responsive-layout spec+plan done, execution not started
- `becc761` — Merge branch 'main' into m6-responsive-layout

This branch lives in the `.worktrees/m6-responsive-layout` worktree and was never pulled after those
2 commits landed on the remote. Fix: `git -C .worktrees/m6-responsive-layout pull` (or `fetch` +
fast-forward) if that worktree is still active work.

## 4. Remote-only branches (no local tracking branch, not a problem)

These exist on `origin` with no corresponding local branch — normal for already-merged milestone
branches nobody deleted remotely:
- `origin/m4-health-nav-theming`
- `origin/m5-readonly-viewer`

## 5. Local-only content never pushed (correctly gitignored, not a discrepancy)

Directories present on disk but intentionally untracked (`git status --ignored`):

| Path | What it is |
|---|---|
| `target/`, `core/target/` | Rust build output |
| `node_modules/` | npm packages |
| `dist/` | Vite production build output |
| `src-tauri/gen/`, `viewer-src-tauri/gen/` | Tauri-generated platform scaffolding |
| `.worktrees/` | git worktree checkouts (e.g. the m6-responsive-layout one above) |
| `.claude/`, `.superpowers/` | local Claude Code / plugin state |

None of these belong in the repo — listed here only because the request was for every difference
between the local folder and the repo, including expected/correct ones.

## Summary

| Item | Status |
|---|---|
| Current branch vs its origin counterpart | In sync |
| `main` (local vs origin) | In sync with each other, but `origin/main` doesn't yet have PR #7's files |
| `m6-responsive-layout` worktree | Local behind origin by 2 commits — needs a pull |
| `m4-health-nav-theming`, `m5-readonly-viewer` | Remote-only, no local branch, not actionable |
| Build/tool artifacts on disk | Correctly gitignored, not tracked anywhere |
