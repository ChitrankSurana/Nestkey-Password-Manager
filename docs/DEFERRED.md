# Deferred Items

Cross-milestone follow-ups that were explicitly considered and postponed, tracked here so they
don't get lost. Not a backlog of every idea — only things raised and deliberately deferred during
real design/review work, with a reason.

## Automated viewport/screenshot testing (raised during M6 responsive-layout design)

**What:** Add browser-automation tests (e.g. Playwright) that render the app at a few key
viewport widths and assert layout correctness / catch visual regressions automatically.

**Why deferred:** This project has no browser/UI test tooling today (only Node-env `vitest` unit
tests). Every prior milestone (M4, M5, M6 web-mode) treated visual/frontend verification as manual
QA via a deferred walkthrough rather than introducing browser automation. Responsive layout is
exactly the kind of feature automated viewport tests would catch regressions in fastest, but adding
a new test framework + CI surface was judged out of scope for a first pass — ship the manual-QA'd
version first, revisit if breakpoint regressions actually start happening in practice.

**Revisit when:** Either (a) a real regression slips through manual QA that automated tests would
have caught, or (b) responsive layout code accumulates enough screens/breakpoints that manual
re-verification on every future frontend change becomes the bottleneck.

**Status (2026-09-30):** Still deferred. The Make UI overhaul added three UI variants, which
raises the cost of manual re-verification; the UAT remediation effort (`docs/testing/UAT_*.md`)
relies on manual and live-app runs. Revisit condition (b) is arguably approaching.
