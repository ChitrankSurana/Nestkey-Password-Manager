# Decisions (from ADR-type documents)

**No entries.**

0 of the 39 classified documents in this ingest carry `type: ADR` or `locked: true`. No ADR-type
documents were found anywhere under `docs/` at ingest time — there is nothing to extract here, and
nothing has been fabricated to fill this file.

This does **not** mean the project has no technical decisions — it means those decisions currently
live as narrative content inside documents classified as PRD, SPEC, or DOC instead of as dedicated,
governance-tracked ADRs. In particular:

- Crypto stack (Argon2id KDF, AES-256-GCM, SQLCipher), data model, and IPC contract choices are
  described in `docs/TRD.md` (classified SPEC) → extracted to `constraints.md`.
- Product-level scope decisions (local-first, zero-knowledge, no cloud sync, milestone sequencing)
  are described in `docs/PRD.md` (classified PRD) → extracted to `requirements.md`.
- Security/threat-boundary decisions (127.0.0.1-only bind, CSRF double-submit, HIBP k-anonymity,
  generic login-failure message) are described in `docs/THREAT_MODEL.md` (classified SPEC) →
  extracted to `constraints.md`.
- Architectural rationale (module boundaries, why the registry DB is unencrypted, why the Viewer
  reuses `core` as a library) is described in `docs/ARCHITECTURE.md` (classified DOC) → extracted
  to `context.md`.

See `INGEST-CONFLICTS.md` → `[INFO] No ADR-type documents in this ingest set` for the full note.
If any of the above should be formally locked as ADRs going forward, that is a decision for the
user/roadmapper to make explicitly — this synthesizer does not promote narrative content to ADR
status on its own authority.
