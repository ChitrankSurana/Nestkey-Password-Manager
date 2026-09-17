---
phase: "08"
slug: "new-vault-ui-figma-port"
status: verified
threats_open: 0
asvs_level: 1
block_on: high
register_authored_at_plan_time: true
created: "2026-09-17"
---

# Phase 08 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| `config.json` (on disk) → running app | A hand-edited or corrupted config value feeds `ui_variant`, `reprompt_mode`, `reprompt_grace_seconds` | UI-variant selection, reprompt policy — all snapped to an allowlist on load |
| Tauri IPC / HTTP portal → core commands | Frontend calls into `core::commands` via either the desktop IPC bridge or the localhost web portal | Config setters, vault CRUD, backup destination path |
| Web portal session → authenticated command surface | Browser tab holding a session cookie reaches the same command surface as the desktop app | Full vault read/write once authenticated (pre-existing `SECURITY_CONCERNS.md` H1/H2/M3 trust model, unchanged by this phase) |
| Vault lock event → in-memory UI state | `vault-locked` Tauri/DOM event must clear all decrypted state held by long-lived React components (`useVault`, `reprompt.ts`) | Decrypted credentials, reprompt grace timestamp |
| Trash → live vault | Soft-deleted sites/credentials are recoverable; a cascade delete must not permanently destroy data before a restore path exists | Cascade-deleted credential rows |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-08-01 | Tampering | `src-tauri/src/lib.rs` window-entry resolution from `config.ui_variant` | high | mitigate | Config value never interpolated into a path; snapped to `classic` on load; builder maps through a `match` with fixed literals | closed |
| T-08-02 | Elevation of Privilege | Tauri window label vs `capabilities/default.json` | high | mitigate | Label is the literal `"main"`; gated by grep + human-check | closed |
| T-08-03 | Denial of Service | Runtime window construction | medium | mitigate | `.build()` failure propagates via `?`, loud not silent | closed |
| T-08-04 | Information Disclosure | `newui.html` served by localhost portal | low | accept | Same session/CSRF layers as `index.html`; no new surface | closed |
| T-08-05 | Tampering | `update_ui_variant` (IPC + HTTP) | high | mitigate | Single validation point (`do_update_ui_variant`) against `VALID_UI_VARIANTS`; both transports pass through it | closed |
| T-08-06 | Spoofing | `/api/update_ui_variant` route registration | high | mitigate | Registered in the same router chain — inherits session + CSRF requirement | closed |
| T-08-07 | Elevation of Privilege | Read-only viewer binary | medium | mitigate | Viewer crate excluded from `files_modified`; lacks `verify_master_password` | closed |
| T-08-08 | Information Disclosure | Interface banner copy | low | accept | Names only interface label + local path, no vault data | closed |
| T-08-09 | Information Disclosure | `useVault.ts` credential fetch surface | high | mitigate | Lazy-decryption invariant enforced structurally — one call site, gated by grep | closed |
| T-08-10 | Information Disclosure | In-memory credentials surviving a vault lock | high | mitigate | `useVault`'s own `vault-locked` listener nulls credentials + selection | closed |
| T-08-11 | Information Disclosure | Site names/URLs in list pane | low | accept | `SiteDto` carries no secret field; unchanged exposure | closed |
| T-08-12 | Denial of Service | Un-debounced search | low | accept | Deliberate, pre-existing; local IPC against SQLite index | closed |
| T-08-13 | Information Disclosure | Hidden-type custom fields in `DetailField.tsx` | high | mitigate | Delegates to `SecretField` carrying `reprompt` flag; gated by grep + human-check | closed |
| T-08-14 | Information Disclosure | Secret masking in `CredentialBlock.tsx` | high | mitigate | Only `SecretField`'s fixed-width mask used; no length-proportional mask | closed |
| T-08-15 | Information Disclosure | Plaintext retained after vault lock | high | mitigate | `vault-locked` listener nulls state; collapsed credentials unmounted | closed |
| T-08-16 | Information Disclosure | Clipboard writes from `CopyButton.tsx` | high | mitigate | All copies route through `api.copyToClipboard`'s 30s auto-clear seam; zero-match grep for direct clipboard call | closed |
| T-08-17 | Spoofing | Per-item breach/reuse status | medium | mitigate | Neither rendered; no per-item command exists; grep-gated | closed |
| T-08-18 | Information Disclosure | Site url as outbound link | low | mitigate | `rel="noreferrer"` + `target="_blank"` | closed |
| T-08-19 | Tampering | Destructive confirmations in new UI | high | mitigate | Reused `SiteEditor`/`Trash` confirm copy verbatim, not re-authored; zero-match grep | closed |
| T-08-20 | Elevation of Privilege | `SettingsPane` in web portal | high | mitigate | `Settings` reused byte-for-byte — `isTauri` gating intact; diff check enforces no edit | closed |
| T-08-21 | Repudiation | Credential deletion with no confirmation | low | accept | Soft delete, restorable from Trash; deliberate parity | closed |
| T-08-22 | Spoofing | Per-item health presentation | medium | mitigate | Links to real vault-wide Health Dashboard instead of a synthesized per-item verdict | closed |
| T-08-23 | Information Disclosure | Editor overlays above three-pane shell | low | accept | Existing `fixed inset-0 z-50` overlay covers panes; unchanged stacking | closed |
| T-08-24 | Information Disclosure | Silent regression of clipboard/reprompt/lazy-decryption invariants | high | mitigate | `parity.test.ts` asserts all three against actual sources on every `npm test`, comments stripped | closed |
| T-08-25 | Spoofing | Reintroduction of D-10 scope cuts as non-functional UI | medium | mitigate | Gate fails on any of five cut nav concepts appearing under new UI | closed |
| T-08-26 | Tampering | Making the parity gate pass by weakening it | medium | mitigate | Diff check fails the task if `package.json`/`vitest.config.ts` changed | closed |
| T-08-27 | Information Disclosure | Full-viewport detail pane below 640px | low | accept | Reveal-off-by-default, reprompt gate, clipboard auto-clear all unchanged by width | closed |
| T-08-28 | Tampering | Narrow tier overwriting saved collapse preference | low | mitigate | Mid/narrow tiers never call `api.updateNavCollapsed` | closed |
| T-08-07-01 | Denial of Service | `CustomField` field rename in `core/src/vault.rs` | high | mitigate | `#[serde(alias = "field_type")]` keeps legacy blobs readable; `custom_field_still_deserializes_legacy_snake_case_key` test passes | closed |
| T-08-07-02 | Information Disclosure | `CustomField` `Hidden` values | low | accept | No `Debug` derive added; hidden values stay out of logs | closed |
| T-08-08-01 | Information Disclosure | `Spinner` rendered next to secret-bearing controls | low | accept | Static glyph/label, no credential/secret prop | closed |
| T-08-08-02 | Denial of Service | Argon2id cost in `core/src/crypto.rs` | medium | accept | Deliberate brute-force resistance, already reviewed in `SECURITY_CONCERNS.md`; UAT-locked decision to defer any change | closed |
| T-08-09-01 | Denial of Service | `core::vault::soft_delete_site` cascade | high | mitigate | Soft delete only, single transaction, cascade to Trash, restore path revives site — `soft_delete_site_cascades_to_its_live_credentials` + `restore_credential_revives_its_cascade_deleted_site` tests pass | closed |
| T-08-09-02 | Elevation of Privilege | `dialog:allow-confirm` capability grant | medium | mitigate | Scoped to `main` window only, matching existing dialog grants; `confirm` is display-only | closed |
| T-08-09-03 | Spoofing | `confirmDialog` web branch (browser-native modal) | low | accept | Attacker with script execution already has full authenticated command surface (`SECURITY_CONCERNS.md` H2/M3) | closed |
| T-08-09-04 | Repudiation | Cascade delete, no audit log | low | accept | No audit log by design (local-first, single user); Trash + restore is the recovery mechanism | closed |
| T-08-10-01 | Tampering | `/api/create_backup` destination path (web route) | **critical** | mitigate | Task 2 passes `None` unconditionally on the web transport, adds no request field — verified live by `create_backup_route_ignores_a_client_supplied_destination` (HTTP integration test, confirmed passing this audit) | closed |
| T-08-10-02 | Information Disclosure | `do_create_backup` with user-chosen `Some(path)` | medium | accept | Bundle is encrypted under the master password; path comes only from an OS dialog; same trust model already accepted for CSV export | closed |
| T-08-10-03 | Denial of Service | `do_create_backup` empty-destination branch | low | mitigate | Rejected with `invalid_input` before any write — `create_backup_rejects_an_empty_destination` test | closed |
| T-08-10-04 | Spoofing | `defaultPath` filename no longer embeds username | low | accept | Slightly less exposure than the prior default, not more | closed |
| T-08-11-01 | Tampering | `Config::load` snapping (reprompt fields) | medium | mitigate | Out-of-allowlist values snap to default on load; grace window is an allowlisted enum, not a free integer | closed |
| T-08-11-02 | Elevation of Privilege | `/api/update_reprompt_mode` | medium | mitigate | Same `Extension<Arc<UserSession>>` extractor as siblings; validated setters accept only allowlisted values | closed |
| T-08-11-03 | Information Disclosure | `ConfigDto` `#[derive(Debug)]` | low | accept | Neither new field is secret (enum label + small integer) | closed |
| T-08-11-04 | Denial of Service | Setters' save-then-commit ordering | low | mitigate | Clone-save-then-assign order preserved; "does not mutate state when save fails" test | closed |
| T-08-12-01 | Elevation of Privilege | Grace window in `src/lib/reprompt.ts` | medium | mitigate | Allowlisted max 300s (Rust-enforced), `clearRepromptGrace()` on `vault-locked` in both shells, in-memory-only state | closed |
| T-08-12-02 | Spoofing | `markRepromptSuccess` placement in `confirmReauth` | high | mitigate | Verified live during this UAT pass: call sits strictly after the `ok === true` check and the wrong-password early return, in the success path only | closed |
| T-08-12-03 | Tampering | `isRepromptRequired` default before config loads | low | accept | Fails open to Per-credential default (today's behavior); deliberate, documented trade-off | closed |
| T-08-12-04 | Information Disclosure | `src/lib/reprompt.ts` module state | low | mitigate | Holds only a policy enum, an integer, and a timestamp — never a secret | closed |
| T-08-SC (×9, one per plan) | Tampering | npm / cargo / pip package installs | low / high | accept | No plan in this phase installed a new package; every construct used was already a direct dependency. No package-legitimacy checkpoint required for any of the 12 plans. | closed |

*Status: open · closed · open — below `high` threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above `high` (`workflow.security_block_on`) count toward `threats_open`*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

**Depth note (ASVS L1):** register authored at plan time for all 12 plans (`register_authored_at_plan_time: true`); every threat carries a plan-authored disposition and rationale, so this audit ran at L1 grep-depth per the standard short-circuit rather than spawning a full auditor. The two highest-stakes entries — T-08-10-01 (critical, web-route arbitrary-path write) and T-08-09-01/T-08-12-02 (cascade-delete transactionality, reprompt-success-gate placement) — were additionally spot-verified against live code and passing tests during this same session (backend `cargo test --workspace`: 356/356 passed; the specific named tests cited above are among them).

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-08-01 | T-08-04, T-08-08, T-08-11, T-08-12, T-08-23, T-08-27, T-08-07-02, T-08-08-01, T-08-09-03, T-08-09-04, T-08-10-02, T-08-10-04, T-08-11-03, T-08-12-03 | Low-severity, deliberate, documented trade-offs (unchanged exposure, existing trust model reused, or accepted UX-vs-security balance) — see each row's Mitigation column for the specific rationale | Phase 08 plan authors | 2026-09-14 to 2026-09-17 |
| AR-08-02 | T-08-08-02 | Argon2id KDF cost is intentional brute-force resistance; UAT (G-08-7) explicitly locked the decision to add a loading spinner instead of reducing cost, deferring any KDF change to a future release-build measurement | User (UAT session) | 2026-09-15 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-09-17 | 58 | 58 | 0 | verify-work (secure-phase, L1 short-circuit + spot verification) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-09-17
