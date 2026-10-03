# Repo Reorg — Finish Tidy (files only)

> On approval, step 0: copy this plan to `docs/Plans/Repo_Reorg_Tidy_Plan_2026-10-02.md` and delete this C-drive copy (per user rule: plans live in `docs/Plans/`).

## Context

PR #14 (`5dd0e77`) grouped `docs/` into `guides/`, `design/` and `testing/`, but some clutter remains:
- `Bitwarden_Feature_copy/` sits at the repo root.
- The design-input and research docs are still loose in `docs/` next to the front-door docs.
- `.gsd/dispatch-isolation-sentinel.json` is a GSD runtime file that got committed.

The user chose the **files-only** scope:
- Do not edit source code, build config, CI, or any of the Tauri, Vite or Cargo files.
- `figma-make/` stays exactly as it is.
- The crate directories, `src/` layout and root HTML entry points stay unchanged.

Rule for choosing what moves: move a file only if no source, build or CI file references it. Files referenced from code comments stay where they are. That keeps the "no code edits" promise:
- `SECURITY_CONCERNS.md` is referenced from `SecretField.tsx`, `handlers.rs` and `http_integration.rs`.
- `KNOWN_LIMITATIONS.md` is referenced from `ci.yml` and `http_integration.rs`.
- `FRESH_MACHINE_SETUP.md` is referenced from `setup-dependencies.ps1`.

## Target layout

```
docs/
  README.md, USAGE_GUIDE.md                 (front door — unchanged)
  FRESH_MACHINE_SETUP.md, PATH_AUDIT.md     (unchanged; code refs)
  KNOWN_LIMITATIONS.md, DEFERRED.md, RESUME_HERE.md  (unchanged; code/.planning refs)
  guides/  design/  testing/  Plans/  superpowers/   (unchanged)
  research/                                  NEW
    FIGMA_DESIGN_BRIEF.md                    <- docs/
    FIGMA_UI_VS_CURRENT_FEATURES.md          <- docs/
    UI_REDESIGN_CAPABILITY_CHECKLIST.md      <- docs/
    REPO_VS_LOCAL_COMPARISON_2026-07-19.md   <- docs/
    make-ui-capture/                         <- docs/make-ui-capture/
    bitwarden/
      compare-ideas/                         <- docs/bitwarden_compare_ideas/
      gui-survey/                            <- Bitwarden_Feature_copy/  (4 .md)
```
Repo root loses `Bitwarden_Feature_copy/` and `.gsd/`. `SECURITY_CONCERNS.md` stays at the root.

## Steps

The branch is `chore/housekeeping`. It already has unrelated staged changes in `Cargo.toml` and `core/src/session.rs`. Keep them out of this commit. Either commit them first on their own, or commit only the reorg paths.

1. **Moves**: use `git mv` so history is kept.
   - `git mv docs/FIGMA_DESIGN_BRIEF.md docs/FIGMA_UI_VS_CURRENT_FEATURES.md docs/UI_REDESIGN_CAPABILITY_CHECKLIST.md docs/REPO_VS_LOCAL_COMPARISON_2026-07-19.md docs/make-ui-capture docs/research/`
   - `git mv docs/bitwarden_compare_ideas docs/research/bitwarden/compare-ideas`
   - `git mv Bitwarden_Feature_copy docs/research/bitwarden/gui-survey`
2. **Untrack `.gsd/`**: run `git rm -r --cached .gsd` and add `.gsd/` to `.gitignore`.
3. **Fix links (markdown only).** Find them with `git grep -n -E 'FIGMA_DESIGN_BRIEF|FIGMA_UI_VS|UI_REDESIGN_CAPABILITY|REPO_VS_LOCAL|make-ui-capture|bitwarden_compare_ideas|Bitwarden_Feature_copy' -- docs README.md`. Known hits:
   - `docs/README.md`: rewrite the "Design inputs and research" section to point at `research/`. Drop the `Bitwarden_Feature_copy/` line under "Elsewhere".
   - `docs/Plans/Modern_UI_Overhaul_Plan*.md`, `Modern_UI_Overhaul_Progress_2026-09-22_0040.md`, `UAT_Modern_UI_Overhaul_Plan_2026-09-23.md`
   - `docs/testing/UAT_FINDINGS_2026-09-23.md`, `docs/testing/UAT_MODERN_UI_OVERHAUL.md`
   - Relative links inside the moved files themselves, e.g. `FIGMA_DESIGN_BRIEF.md` points to its siblings and to `../` paths, and `REPO_VS_LOCAL` points to `PATH_AUDIT`, `FRESH_MACHINE_SETUP` and `SECURITY_CONCERNS`. Add one `../` level where needed.
   - Do **not** edit `figma-make/src/imports/*` (user: keep figma-make as-is). Do not edit `.planning/` (history; it has zero refs to the moved paths).
4. **README.md layout table**: add a `docs/research/` mention if the table lists docs subfolders. Otherwise no change.
5. Commit as `chore: move research docs under docs/research, untrack .gsd`.

## Verification

- `git grep -n -E 'Bitwarden_Feature_copy|docs/make-ui-capture|bitwarden_compare_ideas|docs/FIGMA_|docs/UI_REDESIGN|docs/REPO_VS_LOCAL' -- . ':!.planning'` should return nothing.
- Link check: list the relative `.md` link targets under `docs/` and confirm each one exists. A small PowerShell or bash loop over `git grep -oE '\]\([^)#]+\.md'` is enough. No new tooling.
- `git status`: renames should show as `R` (history kept), and `.gsd/` should be deleted from the index while the file stays on disk.
- `git diff --stat HEAD -- '*.rs' '*.ts' '*.tsx' '*.json' '*.toml' '*.yml'` should be empty except for `.gitignore`, which proves there are no code or build edits.
- Optional sanity check: `npm run build` and `cargo test --workspace` should be unaffected.
