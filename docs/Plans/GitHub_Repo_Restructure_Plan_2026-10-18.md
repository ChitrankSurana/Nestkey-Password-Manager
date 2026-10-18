# GitHub Repo Restructure: Branch Cleanup, Settings, Tags

## Context

Repo `ChitrankSurana/Nestkey-Password-Manager` (private, renamed from `Password_Manager_Local`)
has accumulated merged branches. All 20 PRs are merged, including #20 (`7e0068a` on `main`).
User wants unused branches removed, `feature/modern-ui-overhaul` kept, repo settings tidied, and
tag naming made consistent. `feature/modern-ui-overhaul` is NOT touched. It is 29 behind main,
0 ahead, and left as-is per user choice.

Verified facts (after `git fetch --prune`):
- Every remote branch is merged into `origin/main` (`git branch -r --no-merged origin/main` is empty).
- Local `docs/rename-app-guide` (`eaff874`) is not an ancestor of main, but `git cherry` shows its
  patch is already in main (`-`), via PR #19. Safe to delete.
- Local `old_version_data` = `54b07a7` = tag `pre-modern-ui`. Nothing is lost on delete.
- `m6-responsive-layout` upstream is gone. Its commits are merged (PR #4).
- Branch protection is unavailable (private repo on free plan, HTTP 403), so it is out of scope.

Note: memory rule says plans live in `docs/Plans/`. Plan mode only allows this file. Step 0 copies it there.

## Steps

0. Copy this plan to `docs/Plans/GitHub_Repo_Restructure_Plan_2026-10-18.md` (commit with step 4).

1. **Switch off the branch being deleted, update main**
   - `git checkout main && git pull --ff-only`

2. **Delete remote branches** (all merged)
   - `git push origin --delete chore/housekeeping chore/release-2.9.0 chore/rename-nestkey chore/repo-structure`
   - Keep: `main`, `feature/modern-ui-overhaul`.

3. **Delete local branches**
   - Merged, safe-delete: `git branch -d chore/housekeeping chore/release-2.9.0 chore/rename-nestkey chore/repo-structure m6-responsive-layout old_version_data`
   - Patch-equivalent, needs force: `git branch -D docs/rename-app-guide`
   - `git fetch --prune` and `git remote prune origin`.

4. **Tag hygiene**: rename `pre-modern-ui` to `snapshot/pre-modern-ui`
   - `git tag snapshot/pre-modern-ui pre-modern-ui`
   - `git push origin snapshot/pre-modern-ui`
   - `git push origin --delete refs/tags/pre-modern-ui && git tag -d pre-modern-ui`
   - Keep `snapshot/pre-bitwarden-feature-port` and `v2.9.0` as-is.

5. **Repo settings** via `gh repo edit ChitrankSurana/Nestkey-Password-Manager`
   - `--delete-branch-on-merge` (future merged PR branches auto-delete).
   - `--description "Local-first, zero-knowledge password manager for Windows (Tauri v2 + React + Rust)"`
   - `--add-topic password-manager,tauri,rust,react,zero-knowledge,local-first,windows,sqlcipher`

6. **Commit plan doc** on a short branch `docs/repo-restructure-plan`, then PR to main only if the
   user asks. CI is manual-only. Never dispatch unprompted. Alternative: commit straight to main if the user prefers.

## Irreversibility

Branch and tag deletions on GitHub are outward-facing. Every deleted branch tip is reachable from
`main` or covered by a tag, so content is recoverable. Confirm once before running step 2.

## Verification

- `git branch -a` shows only `main`, `feature/modern-ui-overhaul`, their `origin/` refs, and `origin/HEAD`.
- `git ls-remote --heads origin` lists only those 2 heads.
- `git ls-remote --tags origin` shows `snapshot/pre-modern-ui` (`54b07a7`), `snapshot/pre-bitwarden-feature-port`, `v2.9.0`, and no `pre-modern-ui`.
- `gh repo view --json description,deleteBranchOnMerge,repositoryTopics` reflects step 5.
- `gh pr list --state all` still shows all 20 PRs (deleting branches does not remove PR history).
