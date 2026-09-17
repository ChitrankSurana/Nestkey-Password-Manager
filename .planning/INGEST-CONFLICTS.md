## Conflict Detection Report

### BLOCKERS (0)

None. No LOCKED-vs-LOCKED ADR contradictions (no ADR-type documents exist in this ingest set at
all), no `merge`-mode existing-locked-decision contradictions (`MODE: new`, no existing context to
contradict), no `UNKNOWN`/low-confidence classifications requiring re-tagging, and no cross-reference
cycles (see the INFO entry below — the graph was walked and is clean).

### WARNINGS (1)

[WARNING] Competing constraint variants for "Weak" password classification scope (Health Dashboard)
  Found: docs/superpowers/specs/2026-07-14-m4-health-nav-theming-design.md §3 states weak detection
  "reuses the strength scoring already built in M2 (`do_analyze_strength`, respecting the user's
  `strength_algorithm` setting)."
  Found: docs/superpowers/plans/2026-07-14-m4-health-nav-theming.md Task 7 documents a deliberate
  implementation-time deviation: "This command always classifies 'weak' using zxcvbn, regardless of
  the user's Settings toggle" — because no Rust port of the "simple" algorithm exists. This deviation
  is corroborated as actual shipped/current behavior by docs/KNOWN_LIMITATIONS.md §2 ("confirmed
  intentional after review, not a bug"), docs/HOW_TO_USE_MAIN_APP.md ("does not change how the Health
  Dashboard classifies 'Weak'"), and docs/testing/HOW_TO_TEST_MAIN_APP.md §6.
  Impact: Both the design doc and the plan doc are classified `type: SPEC` with identical (default,
  null) precedence, so synthesis cannot silently prefer one without risk: picking the design doc's
  wording would bake stale intent into downstream planning; picking the plan's wording would silently
  promote an undocumented-at-the-design-level deviation to canonical status without the design doc
  ever being amended. Three corroborating DOC-type sources describe the plan's version as actual
  current behavior, but DOC is lower default precedence than SPEC, so precedence rules alone cannot
  resolve this either.
  → Decide whether `docs/superpowers/specs/2026-07-14-m4-health-nav-theming-design.md` §3 should be
  formally amended to match shipped behavior, or whether the "Weak" classification should be changed
  to actually respect `strength_algorithm` (would require porting the "simple" entropy formula to
  Rust). Both constraint variants are preserved verbatim in `.planning/intel/constraints.md` under
  "M4 Design — Health Classification Rules" and "M4 Plan — Weak-classification implementation
  deviation" pending that decision.

### INFO (6)

[INFO] Duplicate classification collapsed: FRESH_MACHINE_SETUP.md
  Note: Two classification JSONs (`FRESH-MACHINE-SETUP-8c4d5f7e.json` and
  `fresh-machine-setup-b8c2f1e3.json`) both point to the same `source_path`,
  `docs/FRESH_MACHINE_SETUP.md`. Treated as one document in `context.md`; the two classifications'
  summaries and cross_refs were consistent with each other, so no content was lost and there is no
  divergence to flag.

[INFO] No ADR-type documents in this ingest set
  Note: 0 of 39 classified documents carry `type: ADR` or `locked: true`. All technical/architectural
  decisions found in this project (crypto stack, DB schema, bind-address policy, session-token
  design, etc.) currently live as narrative content inside `docs/PRD.md` (PRD), `docs/TRD.md` /
  `docs/THREAT_MODEL.md` / the four M4-M6 `*-design.md` files (SPEC), or plain DOC files — not as
  dedicated ADRs. `.planning/intel/decisions.md` is therefore empty by design, not by omission.

[INFO] PRD.md predates several already-shipped or in-progress features
  Note: `docs/PRD.md` is "Draft v1.0" last updated 2026-07-01, and its Feature Set (§7) only covers
  M1-M6 plus a "Later" row (Android/macOS/Linux). `docs/HOW_TO_TEST_BIOMETRIC_UNLOCK.md` and
  `docs/HOW_TO_TEST_SSH_AGENT.md` (both "added on `feature/bitwarden-port-phase1`") describe Windows
  Hello biometric unlock and an SSH agent over a named pipe — neither appears anywhere in PRD.md's
  user stories or feature table. `docs/FIGMA_UI_VS_CURRENT_FEATURES.md` additionally documents a
  "Closing"-status new-UI port introducing item types (Secure Note/Payment Card/Identity), favorites,
  folders, per-item health, PIN quick-unlock, and custom fields — none of which are in PRD.md either.
  Nothing here contradicts PRD.md (it doesn't forbid any of these), but the PRD is stale relative to
  the actual current/planned feature surface — worth a PRD refresh before downstream roadmap planning
  treats PRD.md §7 as the complete feature list.

[INFO] Cross-reference cycle check: clean
  Note: the `cross_refs` graph across all 39 classified documents was walked (DFS, three-color
  marking, max depth 50) and no cycles were found. The 7 documents previously hand-cleared of cyclic
  `cross_refs` in a prior synthesis attempt (`HOW_TO_USE_MAIN_APP.md`, `HOW_TO_USE_VIEWER_APP.md`,
  `HOW_TO_USE_WEB_PORTAL.md`, `HOW_TO_TEST_MAIN_APP.md`, `HOW_TO_TEST_VIEWER_APP.md`,
  `HOW_TO_TEST_WEB_PORTAL.md`, `KNOWN_LIMITATIONS.md`) remain clean; no new cycles were introduced.

[INFO] Redundant (non-conflicting) coverage: npm/vitest audit fix
  Note: `docs/superpowers/plans/2026-07-19-npm-audit-fix.md` and
  `docs/Plans/npm_vulnerability_fix_plan_19-07-2026.md` both describe the identical decision (upgrade
  `vitest` 2.1.9 → 4.1.10 to resolve GHSA-67mh-4wv8-2f99) with consistent facts (same advisory, same
  root cause, same fix, same verification steps). No divergence between them — captured once in
  `context.md` with both sources attributed.

[INFO] Unlock-attempt throttling is an acknowledged open gap, not a conflict
  Note: `docs/THREAT_MODEL.md` documents no brute-force/lockout algorithm beyond Argon2id's inherent
  cost. `docs/bitwarden_compare_ideas/00-summary.md` and `03-server-security-notes.md` independently
  flag this as a genuine open gap (shared with Bitwarden itself, per their public whitepaper) and
  recommend writing an explicit decision into `THREAT_MODEL.md`. No source claims throttling already
  exists, so this is a documented gap/recommendation for a future decision, not a contradiction
  between sources.
