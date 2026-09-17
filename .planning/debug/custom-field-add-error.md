---
status: diagnosed
trigger: "UAT gap G-08-3 (Phase 08 New Vault UI): Adding a custom field to a credential in the New UI shows a 'Something went wrong. Please Try Again' error. Blocked verifying the hidden-custom-field reprompt path since a hidden field can't be added to test against. Severity: blocker."
created: 2026-09-16T00:00:00Z
updated: 2026-09-16T00:00:00Z
---

## Current Focus

hypothesis: CONFIRMED — `pwmgr_core::vault::CustomField` (core/src/vault.rs) is missing `#[serde(rename_all = "camelCase")]`, so its `field_type` member deserializes expecting the literal JSON key `field_type`, but every frontend caller (src/lib/api.ts's `CustomField` TS interface, populated by src/screens/CredentialEditor.tsx) sends `fieldType` (camelCase). Any `add_credential`/`update_credential` call whose `customFields` array contains at least one entry with a non-empty label fails IPC/HTTP arg deserialization before it ever reaches business logic, in BOTH Tauri desktop IPC (src-tauri/src/commands.rs) and web/Axum JSON (web-src-tauri/src/handlers.rs) — and in BOTH the Classic UI and the New UI, since both UIs render the exact same shared `CredentialEditor.tsx` component and call the same `api.addCredential`/`api.updateCredential`.
test: Traced full call chain from UI to backend struct definitions; compared serde attributes across all layers that touch `CustomField`.
expecting: N/A — diagnosis complete, goal is find_root_cause_only (no fix applied).
next_action: Return ROOT CAUSE FOUND to caller. Do not implement a fix in this session.

## Symptoms

expected: "Adding a custom field to a credential in the New UI succeeds without error" (UAT Test 4, gap G-08-3 truth statement).
actual: User reported: "Adding a custom field shows a 'Something went wrong. Please Try Again' error." Blocked verifying the hidden-custom-field reprompt path since a hidden field can't be added to test against.
errors: "Something went wrong. Please Try Again" — a generic catch-all fallback string, not a specific backend error message.
reproduction: New UI → select a site → expand a credential → click Edit (or Add credential) → click "+ Add" under Custom fields → fill in a Label → click Save. (Also reproducible identically via Classic UI's SiteDetail → Edit/Add credential → same custom-field UI, since it's the same shared component.)
started: Discovered during UAT of Phase 08 (New Vault UI, Figma port). The underlying defect itself was introduced much earlier — commit `aa0dcad feat(vault): add custom fields and master-password re-prompt to items` (pre-Phase-08) — and was never exercised end-to-end through the UI/IPC boundary by any prior test.

## Eliminated

- hypothesis: "New UI's custom-field add flow uses a different/divergent component or API call than Classic UI, and the divergence is the bug."
  evidence: src/newui/NewShell.tsx imports and renders the exact same `src/screens/CredentialEditor.tsx` component used by Classic UI's `src/screens/SiteDetail.tsx` (identical props: siteId, existing, onClose, onSaved). Both call the identical `api.addCredential` / `api.updateCredential` functions in src/lib/api.ts with an identical `CustomField[]` payload shape. There is no New-UI-specific code path for adding a custom field — it is 100% shared code.
  timestamp: 2026-09-16T00:00:00Z

- hypothesis: "The error is a React render crash caught by an error boundary, not an API/command failure."
  evidence: Searched the entire src/ tree for ErrorBoundary/componentDidCatch/getDerivedStateFromError — none exist. The only global catch-all is src/newui-main.tsx's window 'error'/'unhandledrejection' listeners, which are explicitly disabled once `bootstrapped = true` (i.e., after the initial successful mount) via an early `if (bootstrapped) return;` guard. Since the bug reproduces well after the app has mounted and the user is deep in an interactive flow, that listener cannot be the source of the displayed message. The message must come from one of the many `catch (err) { setError(isCommandError(err) ? err.message : "Something went wrong. Please try again."); }` blocks — specifically the one in CredentialEditor.tsx's handleSubmit, which fires exactly on Save after adding a custom field.
  timestamp: 2026-09-16T00:00:00Z

## Evidence

- timestamp: 2026-09-16T00:00:00Z
  checked: src/newui/NewShell.tsx (lines 8, 246-256) and src/screens/SiteDetail.tsx (lines 3, 124-134)
  found: Both New UI and Classic UI mount the identical `<CredentialEditor siteId=... existing=... onClose=... onSaved=... />` component from src/screens/CredentialEditor.tsx. No fork/divergence exists between the two UIs for this flow.
  implication: The bug cannot be a New-UI-vs-Classic-UI integration divergence; it must live in the shared component or the backend contract it talks to.

- timestamp: 2026-09-16T00:00:00Z
  checked: src/screens/CredentialEditor.tsx (handleSubmit, lines 49-75; addField, lines 33-35)
  found: Clicking "+ Add" (addField) only mutates local React state — cannot throw. The actual API calls (`api.addCredential` / `api.updateCredential`) happen only in handleSubmit on Save, wrapped in try/catch that falls back to "Something went wrong. Please try again." whenever `isCommandError(err)` is false.
  implication: The observed error fires on Save, and specifically when the thrown error is NOT shaped like the app's own `{code, message}` CommandError — i.e., a raw/foreign error such as an IPC argument-deserialization failure, not a structured backend business-logic error.

- timestamp: 2026-09-16T00:00:00Z
  checked: src/lib/api.ts (CustomField interface, lines 26-30; addCredential/updateCredential, lines 193-208)
  found: Frontend TS `CustomField` is `{ label: string; value: string; fieldType: "text" | "hidden" | "boolean" }` — camelCase `fieldType`. This exact shape is sent as part of the `customFields` array arg to both `add_credential` and `update_credential`.
  implication: Confirms exact wire payload shape the backend must deserialize: `{"label": ..., "value": ..., "fieldType": ...}`.

- timestamp: 2026-09-16T00:00:00Z
  checked: core/src/vault.rs (lines 111-126)
  found: `pub enum CustomFieldType { Text, Hidden, Boolean }` is annotated `#[serde(rename_all = "lowercase")]` (correct — matches "text"/"hidden"/"boolean" values). But `pub struct CustomField { pub label: String, pub value: String, pub field_type: CustomFieldType }` has NO `#[serde(rename_all = "camelCase")]` (or any per-field rename/alias) at all — it derives only `Serialize, Deserialize` with default (snake_case-preserving) naming.
  implication: When this struct is deserialized directly from JSON (as it is at every IPC/HTTP entry point), serde requires the literal key `field_type`, not `fieldType`. Since `field_type` is a required (non-Option) field, receiving `fieldType` instead means the required field is absent → deserialization error.

- timestamp: 2026-09-16T00:00:00Z
  checked: src-tauri/src/commands.rs (lines 1-29, 195-217) — the `#[tauri::command] add_credential`/`update_credential` desktop entry points
  found: `custom_fields: Vec<CustomField>` where `CustomField` is imported directly as `pwmgr_core::vault::CustomField` (line 25) — the same struct examined above, not a separate camelCase DTO. Tauri's `#[tauri::command]` macro camelCases the top-level *argument names* (`custom_fields` param name -> `customFields` on the wire) but does not, and cannot, alter the serde behavior of a nested struct type used as that argument's element type.
  implication: The desktop IPC path deserializes each array element straight into `vault::CustomField`, hitting the same snake_case-only `field_type` requirement. Any non-empty `customFields` array sent from the frontend fails here, before `do_add_credential`/`do_update_credential` business logic ever runs.

- timestamp: 2026-09-16T00:00:00Z
  checked: web-src-tauri/src/handlers.rs (lines 16, 56-97, 359-380)
  found: `AddCredentialRequest`/`UpdateCredentialRequest` wrapper structs are correctly annotated `#[serde(rename_all = "camelCase")]` for their OWN fields (`site_id`->`siteId`, `custom_fields`->`customFields`), and `custom_fields: Vec<CustomField>` again uses the same unmodified `pwmgr_core::vault::CustomField`. The wrapper's `rename_all` does not propagate into the nested type.
  implication: The web/HTTP path (`/api/add_credential`, `/api/update_credential`) has the identical defect — this is not a Tauri-IPC-specific issue but a structural gap in the shared `CustomField` DTO definition itself, reachable from every consumer (desktop Classic, desktop New UI, web Classic, web New UI).

- timestamp: 2026-09-16T00:00:00Z
  checked: `git log --oneline -- core/src/vault.rs`
  found: `CustomField`/`CustomFieldType` were introduced in commit `aa0dcad feat(vault): add custom fields and master-password re-prompt to items`, well before Phase 08 (New Vault UI Figma port) began.
  implication: This is a pre-existing latent defect, not something the Phase-08 port introduced. It was never caught previously because no automated test exercises the JSON/IPC serialization boundary for `CustomField` (Rust-side tests call `do_add_credential`/`vault::add_credential` directly with native `CustomField` Rust values, bypassing serde JSON entirely) and no manual UAT had previously exercised "add a credential with a custom field" through the actual UI until Phase 08's Test 4.

- timestamp: 2026-09-16T00:00:00Z
  checked: src/lib/api.ts `isCommandError` (lines 13-15) and `httpPost` (lines 137-155)
  found: `isCommandError` only accepts objects with BOTH `code` and `message` keys. A Tauri IPC arg-deserialization failure surfaces as a plain error/string (e.g. "invalid args `customFields` for command `add_credential`: ... missing field `field_type`"), not the app's structured `{code, message}` shape — so `isCommandError` returns false and the UI falls back to the generic "Something went wrong. Please try again." text, matching the user-observed message exactly (aside from likely casing/paraphrase in the user's verbal report).
  implication: This directly explains why the error the user saw was generic/unhelpful rather than a specific validation message — it's a raw transport/deserialization failure, never reaching the app's own error-formatting layer at all.

## Resolution

root_cause: "`pwmgr_core::vault::CustomField` (core/src/vault.rs:121-126) lacks `#[serde(rename_all = \"camelCase\")]` (or an explicit `#[serde(rename = \"fieldType\")]` on its `field_type` member), so it only deserializes JSON with a literal `field_type` key. Every frontend call site sends `fieldType` (camelCase) per src/lib/api.ts's `CustomField` TS interface. This mismatch causes deserialization of the `customFields` array argument to fail at both the Tauri desktop IPC boundary (src-tauri/src/commands.rs `add_credential`/`update_credential`) and the web/Axum JSON boundary (web-src-tauri/src/handlers.rs) whenever at least one custom field with a non-empty label is submitted — before any business logic runs. Because the resulting error is a raw transport-level deserialization failure (not the app's `{code, message}` CommandError shape), `isCommandError()` in src/lib/api.ts returns false and the UI displays the generic fallback \"Something went wrong. Please try again.\" This is a pre-existing defect (introduced in commit aa0dcad, before Phase 08) affecting BOTH Classic and New UI identically, since both render the same shared src/screens/CredentialEditor.tsx and call the same src/lib/api.ts functions — it was simply never previously exercised end-to-end through the real UI/IPC path until Phase 08's UAT Test 4."
fix: ""
verification: ""
files_changed: []
