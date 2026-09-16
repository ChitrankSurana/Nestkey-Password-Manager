---
schema_version: 1
open_count: 1
waived_count: 0
fixed_count: 0
total_count: 1
last_updated: 2026-09-16T17:06:22.899Z
---

# Broken Windows Ledger

> Cross-phase defect register. With `workflow.windows_enforce` enabled, `/gsd-ship` blocks while `open_count > 0`.
> Waive with `gsd-tools windows waive <id> "<reason>"` (reason required).
> Mark fixed with `gsd-tools windows fixed <id>`.

| id | phase | kind | file | line | description | status | reason | recorded_at | resolved_at |
|----|-------|------|------|------|-------------|--------|--------|-------------|-------------|
| 1 | 08 | unrun-verify | core/src/vault.rs |  | Manual UAT re-pass needed: in the New UI, add a custom field of each of the three types (text/hidden/boolean) and confirm save succeeds without error, including the hidden-field reprompt path from UAT Test 4 | open |  | 2026-09-16T17:06:22.899Z |  |

````json
[
  {
    "id": 1,
    "kind": "unrun-verify",
    "phase": "08",
    "file": "core/src/vault.rs",
    "line": null,
    "description": "Manual UAT re-pass needed: in the New UI, add a custom field of each of the three types (text/hidden/boolean) and confirm save succeeds without error, including the hidden-field reprompt path from UAT Test 4",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-09-16T17:06:22.899Z",
    "resolved_at": null,
    "milestone": "v2.0"
  }
]
````
