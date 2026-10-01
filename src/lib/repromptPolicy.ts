import type { RepromptMode } from "./api";

// Pure gate decision for the master-password reprompt (G-08-2, reworked for B7). Every credential
// is gated by the master password by default now - there is no more "unflagged credentials never
// prompt" case. Two knobs:
//
// - The vault-wide policy picks how the SHARED grace window behaves: "grace" opens a window after
//   one correct entry, during which no credential (flagged or not) re-prompts; "always" never
//   opens one, so every reveal prompts regardless of any prior success.
// - `exempt` is a per-credential override (the credential's own `reprompt` flag, inverted from its
//   pre-B7 meaning - see core/src/vault.rs) that skips the gate entirely for that one credential,
//   regardless of policy or grace state. This is the escape hatch for a throwaway/non-sensitive
//   entry the user doesn't want friction on; it is not a way to add MORE friction, since the
//   baseline is now already "gated".
//
// `now` and `lastSuccessAt` are epoch milliseconds passed in by the caller, never read from the
// clock inside this module - that is what makes the window logic testable with no fake timers.
// This module must stay pure: no `Date.now()`, no module-level mutable state, no import of
// anything beyond the `RepromptMode` type. The stateful shell (the actual grace-window store) is
// `src/lib/reprompt.ts`.
export interface RepromptPolicy {
  mode: RepromptMode;
  graceSeconds: number;
}

// Load-bearing default: until the real config arrives (see `ensureRepromptConfig` in
// `src/lib/reprompt.ts`), the app must still gate every reveal, never reveal-by-default.
export const DEFAULT_REPROMPT_POLICY: RepromptPolicy = { mode: "grace", graceSeconds: 30 };

export function repromptRequired(
  policy: RepromptPolicy,
  exempt: boolean,
  lastSuccessAt: number | null,
  now: number,
): boolean {
  // Per-credential override: this one credential never gates, independent of everything else.
  if (exempt) {
    return false;
  }

  if (policy.mode === "always") {
    return true;
  }

  // "grace" mode.
  // No successful entry yet this session (or since the grace window last expired/was cleared) -
  // always prompt.
  if (lastSuccessAt === null) {
    return true;
  }
  const elapsedMs = now - lastSuccessAt;
  // A negative elapsed time means the clock moved backwards since the last success. That cannot
  // be attacker-induced in this local-only single-user app, so treat it the same as "well inside
  // the window" rather than leaving the behavior undefined.
  if (elapsedMs < 0) {
    return false;
  }
  // Exact-boundary elapsed time expires the window (>=, not >) - do not leave the boundary
  // ambiguous.
  return elapsedMs >= policy.graceSeconds * 1000;
}
