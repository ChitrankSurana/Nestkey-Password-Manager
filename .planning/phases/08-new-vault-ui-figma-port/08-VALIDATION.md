---
phase: "08"
slug: "new-vault-ui-figma-port"
status: validated
nyquist_compliant: true
wave_0_complete: true
created: "2026-09-17"
---

# Phase 08 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | vitest 4.x (frontend) + cargo test (Rust workspace: pwmgr-core, pwmgr-web) |
| **Config file** | `vitest.config.ts` (frontend); `Cargo.toml` workspace (backend) |
| **Quick run command** | `npx vitest run <file>` / `cargo test -p pwmgr-core <name>` |
| **Full suite command** | `npm test && npm run build && cargo test --workspace` |
| **Estimated runtime** | ~2 min (frontend: ~1s tests + ~1s build; backend: ~90s pwmgr-core + ~15s pwmgr-web/end_to_end) |

---

## Sampling Rate

- **After every task commit:** Run the task's own `<automated>` command(s)
- **After every plan wave:** Run `npm test && npm run build` (frontend) and `cargo test --workspace` (backend)
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** ~90s (bounded by the pwmgr-core Rust suite, the slowest single command)

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 08-01-01 | 01 | 1 | VAULT-03 | integration | `npm run build`, `cargo test -p pwmgr-core` | ✅ | ✅ green |
| 08-01-03 | 01 | 1 | VAULT-03 | build | `npm run build` | ✅ | ✅ green |
| 08-02-01 | 02 | 2 | VAULT-03 | integration | `cargo test --workspace` | ✅ | ✅ green |
| 08-02-02 | 02 | 2 | VAULT-03 | build | `npm run build` | ✅ | ✅ green |
| 08-03-01 | 03 | 2 | VAULT-03/04/05 | unit | `npm test` (vaultSelectors.test.ts) | ✅ | ✅ green |
| 08-03-02 | 03 | 2 | VAULT-03/04/05 | build | `npm run build` | ✅ | ✅ green |
| 08-03-03 | 03 | 2 | VAULT-03/04/05 | build | `npm run build` | ✅ | ✅ green |
| 08-04-01 | 04 | 3 | HEALTH-02, VAULT-03/04/05 | unit | `npx vitest run src/newui/lib/credentialLayout.test.ts` | ✅ | ✅ green |
| 08-04-02 | 04 | 3 | HEALTH-02, VAULT-03/04/05 | build | `npm run build` | ✅ | ✅ green |
| 08-04-03 | 04 | 3 | HEALTH-02, VAULT-03/04/05 | build | `npm run build` | ✅ | ✅ green |
| 08-05-01 | 05 | 4 | VAULT-03 | build | `npm run build` | ✅ | ✅ green |
| 08-05-02 | 05 | 4 | VAULT-03 | build | `npm run build` | ✅ | ✅ green |
| 08-05-03 | 05 | 4 | VAULT-03 | build | `npm run build` | ✅ | ✅ green |
| 08-06-01 | 06 | 5 | VAULT-03/04/05, HEALTH-02 | build | `npm run build` | ✅ | ✅ green |
| 08-06-02 | 06 | 5 | VAULT-03/04/05, HEALTH-02 | build | `npm run build` | ✅ | ✅ green |
| 08-06-03 | 06 | 5 | VAULT-03/04/05, HEALTH-02 | unit | `npx vitest run src/newui/lib/parity.test.ts` | ✅ | ✅ green |
| 08-07-01 | 07 | 1 | VAULT-01 | unit | `cargo test -p pwmgr-core custom_field`, `grep -c 'rename_all = "camelCase"' core/src/vault.rs` | ✅ | ✅ green |
| 08-07-02 | 07 | 1 | VAULT-01 | integration | `cargo test -p pwmgr-core credential_with_hidden_custom_field_round_trips_through_vault`, `cargo test --workspace` | ✅ | ✅ green |
| 08-08-01 | 08 | 1 | UX-01 | unit | `grep -c 'animate-spin' src/components/Spinner.tsx`, `npx vitest run src/components/Spinner.test.ts` | ✅ | ✅ green |
| 08-08-02 | 08 | 1 | UX-01 | integration | `npm test`, `npm run build` | ✅ | ✅ green |
| 08-09-01 | 09 | 2 | VAULT-01 | decision | N/A — product-decision task, no code change | N/A | ✅ decided |
| 08-09-02 | 09 | 2 | VAULT-01 | unit | `cargo test -p pwmgr-core soft_delete_site`, `cargo test -p pwmgr-core restore_credential` | ✅ | ✅ green |
| 08-09-03 | 09 | 2 | VAULT-01 | integration | `! grep -q 'window\.confirm(' src/screens/SiteEditor.tsx src/screens/Backup.tsx`, `npm run build` | ✅ | ✅ green |
| 08-09-04 | 09 | 2 | VAULT-01 | integration | `npm run build`, `npm test` | ✅ | ✅ green |
| 08-10-01 | 10 | 3 | DATA-01 | unit | `cargo test -p pwmgr-core create_backup` | ✅ | ✅ green |
| 08-10-02 | 10 | 3 | DATA-01 | integration | `cargo test -p pwmgr-core`, `cargo build -p pwmgr-app`, `cargo test -p pwmgr-web` | ✅ | ✅ green |
| 08-10-03 | 10 | 3 | DATA-01 | integration | `npm run build`, `npm test` | ✅ | ✅ green |
| 08-11-01 | 11 | 4 | AUTH-01 | unit | `cargo test -p pwmgr-core reprompt` | ✅ | ✅ green |
| 08-11-02 | 11 | 4 | AUTH-01 | integration | `cargo test -p pwmgr-core`, `cargo build -p pwmgr-app`, `cargo test -p pwmgr-web reprompt` | ✅ | ✅ green |
| 08-11-03 | 11 | 4 | AUTH-01 | integration | `npm run build`, `npm test` | ✅ | ✅ green |
| 08-12-01 | 12 | 5 | AUTH-01 | unit | `npx vitest run src/lib/repromptPolicy.test.ts`, `npm test` | ✅ | ✅ green |
| 08-12-02 | 12 | 5 | AUTH-01 | integration | `npm test`, `npm run build` | ✅ | ✅ green |
| 08-12-03 | 12 | 5 | AUTH-01 | integration | `npm test`, `npm run build` | ✅ | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

**Full-suite confirmation (this validation pass, 2026-09-17):** `npm test` → 40/40 passed. `npm run build` → clean (tsc + vite). `cargo test --workspace` → pwmgr-core 329 passed + 2 end-to-end passed, pwmgr-web 17+9 passed, 0 failed across the entire workspace.

---

## Wave 0 Requirements

Existing infrastructure (vitest + cargo test, already wired before Phase 08 started) covers all phase requirements. No new framework install or scaffolding was needed.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Full responsive-tier walkthrough (>=1024px / 640-1023px / <640px) and the 18-flow capability checklist parity between New UI and Classic | VAULT-03/04/05 | Live visual/interaction judgment across breakpoints and both UIs — no automated visual-regression harness in this project | See `08-UAT.md` Test 6 (deferred by user to a later session) and `docs/UI_REDESIGN_CAPABILITY_CHECKLIST.md` §1/§4 |
| Live grace-window timing UX (Global reprompt mode, cross-credential) and auto-lock interaction | AUTH-01 | End-to-end timing/UX property spanning multiple mounted components in a real running app | See `08-12-SUMMARY.md` coverage D2 rationale; confirmed live in `08-UAT.md` Test 1 |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references (none — no gaps found)
- [x] No watch-mode flags
- [x] Feedback latency < 90s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-09-17
