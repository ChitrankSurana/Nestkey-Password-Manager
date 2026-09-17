---
phase: 08-new-vault-ui-figma-port
plan: 07
subsystem: vault
tags: [serde, rust, custom-fields, tauri-ipc, axum]

# Dependency graph
requires:
  - phase: 08-new-vault-ui-figma-port (plans 01-06)
    provides: New Vault UI shell, detail panel, and credential editing surfaces that call add_credential/update_credential with custom fields
provides:
  - CustomField's JSON wire names are camelCase (fieldType), matching src/lib/api.ts on both the Tauri IPC and web HTTP transports
  - Backward-compatible deserialization of legacy snake_case (field_type) blobs already persisted in existing vaults
  - Regression coverage proving both the pure-serde contract and the real seal/encrypt/decrypt/open round trip
affects: [08-new-vault-ui-figma-port UAT re-pass, hidden-custom-field reprompt path (UAT Test 4)]

actuals:
  tokens: 780
  tasks: 2
  commits: 2

tech-stack:
  added: []
  patterns:
    - "serde rename_all=camelCase + field-level alias for backward-compatible wire-format migration of an encrypted, persisted blob"

key-files:
  created: []
  modified:
    - core/src/vault.rs

key-decisions:
  - "Used #[serde(alias = \"field_type\")] rather than a one-time data migration - the encrypted blob is opaque at rest, so accepting both spellings on read is simpler and safer than trying to migrate ciphertext"

patterns-established:
  - "Wire-format renames on any struct serialized into an encrypted-at-rest blob must add a serde alias for the old key name, never just rename - the plan's threat model made this a 'mitigate' disposition (T-08-07-01), not optional"

requirements-completed: [VAULT-01]

coverage:
  - id: D1
    description: "CustomField deserializes camelCase fieldType from JSON (fixes the Tauri IPC and web HTTP transport failure)"
    requirement: "VAULT-01"
    verification:
      - kind: unit
        ref: "core/src/vault.rs#vault::tests::custom_field_deserializes_camel_case_field_type"
        status: pass
      - kind: unit
        ref: "core/src/vault.rs#vault::tests::custom_field_serializes_field_type_as_camel_case"
        status: pass
    human_judgment: false
  - id: D2
    description: "Legacy encrypted custom-fields blobs (snake_case field_type, written before this fix) still decrypt and parse"
    requirement: "VAULT-01"
    verification:
      - kind: unit
        ref: "core/src/vault.rs#vault::tests::custom_field_still_deserializes_legacy_snake_case_key"
        status: pass
    human_judgment: false
  - id: D3
    description: "A hidden custom field round-trips through the real vault (add_credential -> seal -> encrypt -> decrypt -> open -> list_credentials_for_site)"
    requirement: "VAULT-01"
    verification:
      - kind: unit
        ref: "core/src/vault.rs#vault::tests::credential_with_hidden_custom_field_round_trips_through_vault"
        status: pass
      - kind: unit
        ref: "cargo test --workspace (312 pwmgr-core + 2 end_to_end + 17 pwmgr_web + 5 http_integration, all pass)"
        status: pass
    human_judgment: false
  - id: D4
    description: "Manual confirmation that the New UI can add a custom field of each of the three types without error"
    verification: []
    human_judgment: true
    rationale: "Requires driving the actual UI (desktop or web) interactively; deferred to the next UAT pass per this plan's <verification> section - out of scope for an automated unit-test gate."

duration: ~15min
completed: 2026-09-16
status: complete
---

# Phase 08 Plan 07: CustomField camelCase Wire-Name Fix Summary

**Fixed `pwmgr_core::vault::CustomField` missing `#[serde(rename_all = "camelCase")]`, which made every custom-field add/edit fail deserialization on both the Tauri IPC and web HTTP transports, while keeping pre-existing encrypted custom-field blobs readable via a field-level alias.**

## Performance

- **Duration:** ~15 min
- **Tasks:** 2
- **Files modified:** 1 (`core/src/vault.rs`)

## Accomplishments

- `CustomField` now serializes/deserializes with `fieldType` on the wire, matching `src/lib/api.ts`'s `CustomField.fieldType` on both `src-tauri` (IPC) and `web-src-tauri` (axum `Json`)
- `#[serde(alias = "field_type")]` on `field_type` keeps every vault that already holds a custom field readable — `open_custom_fields` decrypts and parses the same as before the rename
- Four new regression tests: two pure-serde tests proving the camelCase contract, one proving legacy snake_case still deserializes, and one proving the real encrypted round trip through `add_credential` / `list_credentials_for_site`
- Confirmed `cargo test --workspace` green after the change (312 `pwmgr-core` unit tests, 2 end-to-end, 17 `pwmgr-web` unit tests, 5 `http_integration` tests — no regressions)
- Unblocks G-08-3 and the hidden-custom-field reprompt sub-check that UAT Test 4 could not previously exercise

## Task Commits

Each task was committed atomically, following TDD (tests written first, confirmed RED, then the fix applied and confirmed GREEN):

1. **Task 1: Make CustomField's JSON wire names camelCase, backward-compatibly** - `4d23d92` (fix)
   - Added `#[serde(rename_all = "camelCase")]` and `#[serde(alias = "field_type")]`
   - Added `custom_field_deserializes_camel_case_field_type`, `custom_field_serializes_field_type_as_camel_case`, `custom_field_still_deserializes_legacy_snake_case_key`
   - Confirmed RED (2 of 3 tests failed) before the struct annotation was applied, then GREEN after
2. **Task 2: Prove the fix end-to-end through the vault, and confirm no existing test regressed** - `7067d53` (test)
   - Added `credential_with_hidden_custom_field_round_trips_through_vault`
   - Ran `cargo test --workspace` — all 336 tests pass, no regressions

**Plan metadata:** (this commit)

## Files Created/Modified

- `core/src/vault.rs` - `CustomField` struct gained `#[serde(rename_all = "camelCase")]` and a `#[serde(alias = "field_type")]` on `field_type`; test module gained 4 new tests

## Decisions Made

- Used `#[serde(alias = "field_type")]` instead of a data migration for existing encrypted blobs — the blob is opaque ciphertext at rest, so a read-time alias is simpler and cannot silently lose data, unlike a one-time migration that would need to touch every vault file.

## Deviations from Plan

None - plan executed exactly as written. TDD sequencing was followed literally: tests were written against the pre-fix struct, confirmed to fail (RED) for the two camelCase-specific assertions, then the fix was applied and re-verified (GREEN).

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- G-08-3 is closed. Custom-field add/edit is unblocked on both desktop (Tauri IPC) and web (axum HTTP) transports.
- The hidden-custom-field reprompt sub-check from UAT Test 4 is now testable and should be re-run in the next UAT pass, per this plan's `<verification>` section (manual confirmation deferred: open a site, edit a credential, add a custom field of each of the three types, save without error).
- No existing vault becomes unreadable — a credential saved with a custom field before this change still decrypts and lists correctly.

---
*Phase: 08-new-vault-ui-figma-port*
*Completed: 2026-09-16*

## Self-Check: PASSED

- FOUND: core/src/vault.rs
- FOUND: commit 4d23d92
- FOUND: commit 7067d53
- FOUND: .planning/phases/08-new-vault-ui-figma-port/08-07-SUMMARY.md
