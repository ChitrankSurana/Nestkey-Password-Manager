import { api } from "./api";
import { DEFAULT_REPROMPT_POLICY, repromptRequired, type RepromptPolicy } from "./repromptPolicy";

// Thin stateful shell over the pure decision in `repromptPolicy.ts`. This state is deliberately
// module-scoped and in-memory only: it must never be persisted, and a page reload resetting it is
// correct, not a bug. It holds only a policy enum, an integer, and a timestamp - never the master
// password, never a secret.
let policy: RepromptPolicy = DEFAULT_REPROMPT_POLICY;
let lastSuccessAt: number | null = null;
let loadPromise: Promise<void> | null = null;

// Memoized so N mounted `SecretField`s cause exactly one `api.getConfig()` call. On failure the
// default policy stays in place and `loadPromise` is cleared so a later mount can retry.
export function ensureRepromptConfig(): Promise<void> {
  if (loadPromise === null) {
    loadPromise = api
      .getConfig()
      .then((c) => {
        policy = { mode: c.repromptMode, graceSeconds: c.repromptGraceSeconds };
      })
      .catch(() => {
        loadPromise = null;
      });
  }
  return loadPromise;
}

// Clears the memoization and reloads, so a Settings change takes effect in the current session
// without a restart.
export function refreshRepromptConfig(): Promise<void> {
  loadPromise = null;
  return ensureRepromptConfig();
}

export function isRepromptRequired(perCredentialFlag: boolean): boolean {
  return repromptRequired(policy, perCredentialFlag, lastSuccessAt, Date.now());
}

// Only records a success while in Global mode. Recording it in Per-credential mode would be
// harmless today (the pure function ignores `lastSuccessAt` in that mode) but would silently
// open a window the instant the user switched to Global, which is the opposite of what they
// asked for - Global mode should start cold.
export function markRepromptSuccess(): void {
  if (policy.mode === "global") {
    lastSuccessAt = Date.now();
  }
}

// Called on the `vault-locked` event in both app shells - without this the grace window would
// outlive the lock, and the first reveal after unlocking would skip the prompt.
export function clearRepromptGrace(): void {
  lastSuccessAt = null;
}
