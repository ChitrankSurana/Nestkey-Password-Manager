# Deferred items (09-04)

- Registering an already-taken username returns HTTP 500 (internal_error), not 409 username_taken:
  `core/src/auth.rs::register` calls `vault::open_vault` with a freshly derived key BEFORE the registry
  insert, so it fails against the existing vault file. Pre-existing; out of scope for SEC-02.
