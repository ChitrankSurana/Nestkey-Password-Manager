import type { RepromptMode } from "./api";

// Pure gate decision for the master-password reprompt (G-08-2). Two modes, deliberately never
// sharing a code path so a future edit to one cannot leak into the other:
//
// - "perCredential" (today's behavior, and the default): only credentials whose own `reprompt`
//   flag is set ask for the master password, and they ask EVERY time - there is no grace window
//   in this mode, by the UAT's explicit locked decision.
// - "global": every credential's reveal/copy asks for the master password, regardless of its own
//   flag - UNLESS a prior successful entry opened a grace window (see `graceSeconds`) that is
//   still open, in which case the gate is skipped for any credential, flagged or not.
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
// `src/lib/reprompt.ts`), the app must behave exactly as it does today.
export const DEFAULT_REPROMPT_POLICY: RepromptPolicy = { mode: "perCredential", graceSeconds: 30 };

export function repromptRequired(
  policy: RepromptPolicy,
  perCredentialFlag: boolean,
  lastSuccessAt: number | null,
  now: number,
): boolean {
  if (policy.mode === "global") {
    // No successful entry yet this session (or since the grace window last expired/was
    // cleared) - always prompt.
    if (lastSuccessAt === null) {
      return true;
    }
    const elapsedMs = now - lastSuccessAt;
    // A negative elapsed time means the clock moved backwards since the last success. That
    // cannot be attacker-induced in this local-only single-user app, so treat it the same as
    // "well inside the window" rather than leaving the behavior undefined.
    if (elapsedMs < 0) {
      return false;
    }
    // Exact-boundary elapsed time expires the window (>=, not >) - do not leave the boundary
    // ambiguous.
    return elapsedMs >= policy.graceSeconds * 1000;
  }

  // Per-credential mode: the grace window never applies here, even if `lastSuccessAt` is set
  // (e.g. from a prior Global-mode session before the user switched modes). Only the
  // credential's own flag matters, and a flagged credential prompts every single time.
  return perCredentialFlag === true;
}
