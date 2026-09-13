# Decisions (from ADRs)

No ADR-type documents were found in this ingest set (39 classifications; 0 typed `ADR`). There are
therefore no locked or proposed architectural decisions to record from this pass.

Several documents contain design rationale and architecture commitments that *read* like decisions
(e.g. `docs/ARCHITECTURE.md`'s module-boundary rationale, the M4/M5/M6 design specs' "Architecture —
module placement" sections), but per the doc-classifier these were typed `DOC` or `SPEC`, not `ADR`.
Their content is captured in `constraints.md` (SPECs) and `context.md` (DOCs) instead, per the
type-routing rules — it is not duplicated here.

If any of these should be formalized as locked decisions (e.g. "split data-encryption key from
password-derived key," proposed but not yet built — see `context.md` topic "Bitwarden Comparison
Research"), they should be written up as ADRs and re-ingested.
