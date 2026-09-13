---
created: 2026-09-13T13:49:40.048Z
title: Resolve M4 Health Dashboard weak-classification doc conflict
area: docs
severity: cosmetic
files:
  - docs/superpowers/specs/2026-07-14-m4-health-nav-theming-design.md:3
  - docs/superpowers/plans/2026-07-14-m4-health-nav-theming.md
  - docs/KNOWN_LIMITATIONS.md:2
  - docs/HOW_TO_USE_MAIN_APP.md
  - docs/testing/HOW_TO_TEST_MAIN_APP.md:6
---

## Problem

`/gsd-ingest-docs` synthesis (MODE=new, retry #3) flagged one unresolved WARNING —
docs disagree on the M4 Health Dashboard's "Weak" password classification:

- `docs/superpowers/specs/2026-07-14-m4-health-nav-theming-design.md` §3 says weak
  detection "reuses the strength scoring already built in M2 (`do_analyze_strength`,
  respecting the user's `strength_algorithm` setting)."
- `docs/superpowers/plans/2026-07-14-m4-health-nav-theming.md` Task 7 documents a
  deliberate implementation-time deviation: this command always classifies "weak"
  using zxcvbn, regardless of the user's Settings toggle — because no Rust port of
  the "simple" algorithm exists.
- This deviation is corroborated as actual shipped/current behavior by
  `docs/KNOWN_LIMITATIONS.md` §2 ("confirmed intentional after review, not a bug"),
  `docs/HOW_TO_USE_MAIN_APP.md`, and `docs/testing/HOW_TO_TEST_MAIN_APP.md` §6.

Both docs are classified `type: SPEC` with identical default precedence, so the
ingest synthesizer's `ADR > SPEC > PRD > DOC` precedence rule can't auto-resolve it.
Both variants preserved verbatim in `.planning/intel/constraints.md` under
"M4 Design — Health Classification Rules" and "M4 Plan — Weak-classification
implementation deviation".

User deferred the decision during `/gsd-ingest-docs` (asked to track it rather than
decide inline) so the ingest could proceed past this WARNING to routing/roadmapping.

## Solution

Decide one of:
1. Formally amend `docs/superpowers/specs/2026-07-14-m4-health-nav-theming-design.md`
   §3 to match shipped behavior (always zxcvbn) — cheapest, matches reality.
2. Change the Health Dashboard to actually respect `strength_algorithm` — requires
   porting the "simple" entropy formula to Rust (real scope of work).

No functional bug either way — shipped behavior is intentional and tested. This is
a docs-vs-code consistency cleanup, not a code fix.
