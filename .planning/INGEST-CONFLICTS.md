## Conflict Detection Report

### BLOCKERS (2)

[BLOCKER] Cross-reference cycle among "How to Use" guides
  Found: docs/HOW_TO_USE_MAIN_APP.md cross-references docs/HOW_TO_USE_VIEWER_APP.md and docs/HOW_TO_USE_WEB_PORTAL.md; docs/HOW_TO_USE_VIEWER_APP.md cross-references docs/HOW_TO_USE_MAIN_APP.md and docs/HOW_TO_USE_WEB_PORTAL.md; docs/HOW_TO_USE_WEB_PORTAL.md cross-references docs/HOW_TO_USE_MAIN_APP.md and docs/HOW_TO_USE_VIEWER_APP.md — a mutual strongly-connected cycle (A→B→A, A→C→A, B→C→B) in the cross_refs graph.
  Expected: An acyclic reference graph so per-doc extraction can proceed independently without ambiguity about read order.
  → These 3 documents were excluded from `.planning/intel/context.md` extraction. If these are simply "See also" navigational links (likely, given they are parallel per-app user guides), re-run ingest with `--manifest` marking these cross_refs as non-dependency navigational links, or accept them into context.md manually.

[BLOCKER] Cross-reference cycle among "How to Test" guides and Known Limitations
  Found: docs/KNOWN_LIMITATIONS.md cross-references docs/testing/HOW_TO_TEST_VIEWER_APP.md and docs/testing/HOW_TO_TEST_WEB_PORTAL.md; docs/testing/HOW_TO_TEST_MAIN_APP.md cross-references docs/testing/HOW_TO_TEST_VIEWER_APP.md, docs/testing/HOW_TO_TEST_WEB_PORTAL.md, and docs/KNOWN_LIMITATIONS.md; docs/testing/HOW_TO_TEST_VIEWER_APP.md cross-references docs/testing/HOW_TO_TEST_MAIN_APP.md, docs/testing/HOW_TO_TEST_WEB_PORTAL.md, and docs/KNOWN_LIMITATIONS.md; docs/testing/HOW_TO_TEST_WEB_PORTAL.md cross-references docs/testing/HOW_TO_TEST_MAIN_APP.md, docs/KNOWN_LIMITATIONS.md, and THREAT_MODEL.md — a 4-node mutual cycle.
  Expected: An acyclic reference graph so per-doc extraction can proceed independently without ambiguity about read order.
  → These 4 documents were excluded from `.planning/intel/context.md` extraction. `docs/KNOWN_LIMITATIONS.md` in particular is high-value content (deferred/testing-gap catalog) — recommend re-running ingest with `--manifest` marking these cross_refs as non-dependency navigational links so it can be included.

### WARNINGS (1)

[WARNING] Competing guidance on vault item-type expansion (Card/Secure Note/Identity types)
  Found: docs/FIGMA_UI_VS_CURRENT_FEATURES.md marks "Item types" (Login/Secure Note/Payment Card/Identity) as "Closing — scheduled for implementation as part of the new-UI port."
  Found: docs/bitwarden_compare_ideas/02-vault-and-cli.md (§3, item 5) and docs/bitwarden_compare_ideas/00-summary.md ("Explicitly flagged, not recommended") both recommend skipping item-type variety, reasoning that it exists in Bitwarden purely to power autofill, which is an explicit PRD non-goal.
  Impact: Two DOC-tier sources give directly opposing direction on whether to build multi-item-type support. docs/PRD.md itself is silent on item types specifically — only "browser extension / autofill" is a named non-goal (§4) — so neither source is overridden by PRD precedence. Roadmapper cannot safely schedule or skip this work without a decision.
  → User should decide whether item-type expansion is in scope for a future milestone (e.g. as part of the Figma-driven UI port), or mark one source as superseded before routing to roadmapping.

### INFO (5)

[INFO] No ADRs present in this ingestion batch
  Note: Of 39 classified documents (31 DOC, 7 SPEC, 1 PRD, 0 ADR, 0 UNKNOWN), none were type ADR and none were `locked`. `.planning/intel/decisions.md` is therefore empty of decisions. LOCKED-vs-LOCKED and LOCKED-vs-non-LOCKED precedence checks were not applicable (nothing to check against). See decisions.md for candidate content that could become future ADRs.

[INFO] Duplicate classification of docs/FRESH_MACHINE_SETUP.md
  Note: Two classification JSON files (FRESH-MACHINE-SETUP-8c4d5f7e.json and fresh-machine-setup-b8c2f1e3.json) point at the identical source_path docs/FRESH_MACHINE_SETUP.md with matching content (minor title/scope wording differences only, no factual conflict). Collapsed to a single entry in context.md.

[INFO] Redundant planning documents for the same npm audit fix, no contradiction
  Note: docs/superpowers/plans/2026-07-19-npm-audit-fix.md and docs/Plans/npm_vulnerability_fix_plan_19-07-2026.md independently document the identical decision (upgrade vitest 2.1.9 → 4.1.10 to resolve 5 nested npm audit findings via GHSA-67mh-4wv8-2f99). Both retained as separate context.md entries for provenance; no precedence conflict since content agrees.

[INFO] docs/PRD.md and docs/TRD.md appear stale relative to later-shipped features
  Note: PRD.md/TRD.md are both dated "Last updated: 2026-07-01." Several features documented only in DOC-tier sources — Windows Hello biometric unlock (docs/HOW_TO_TEST_BIOMETRIC_UNLOCK.md), SSH agent over a named pipe (docs/HOW_TO_TEST_SSH_AGENT.md), PIN quick-unlock, TOTP live display, custom fields, and reprompt-on-reveal (all per docs/FIGMA_UI_VS_CURRENT_FEATURES.md) — are not mentioned in PRD.md's §6 user stories or §7 milestone table. No contradiction (PRD does not forbid them), but the documented scope has drifted from the shipped surface — roadmapper may want to reconcile PRD.md before using it as the sole requirements source.

[INFO] M4 milestone is covered by two separately-classified SPEC documents, cross-checked and consistent
  Note: docs/superpowers/specs/2026-07-14-m4-health-nav-theming-design.md (the design) and docs/superpowers/plans/2026-07-14-m4-health-nav-theming.md (the plan, also classified SPEC due to prescriptive language) were compared for the HIBP k-anonymity flow, the 5-accent-color set, and app-crate-only `reqwest`/`sha1` placement — content matches with no contradiction. The plan operationalizes the design; both are captured as separate entries in constraints.md for provenance.
