import { api } from "./api";
import { DEFAULT_REPROMPT_POLICY, repromptRequired, type RepromptPolicy } from "./repromptPolicy";

// Thin stateful shell over the pure decision in `repromptPolicy.ts`. This state is deliberately
// module-scoped and in-memory only: it must never be persisted, and a page reload resetting it is
// correct, not a bug. It holds only a policy enum, an integer, a timestamp, and re-mask callbacks
// - never the master password, never a secret.
let policy: RepromptPolicy = DEFAULT_REPROMPT_POLICY;
let lastSuccessAt: number | null = null;
let loadPromise: Promise<void> | null = null;
let expiryTimer: ReturnType<typeof setTimeout> | null = null;

// B7: every `SecretField` that reveals a value under the grace window registers a callback here so
// it can be told to re-mask when that window expires, without each field running its own timer
// (which would drift out of sync with each other and with the shared `lastSuccessAt`). One shared
// timer, scheduled from `markRepromptSuccess`, fires all of them together.
const onExpireCallbacks = new Set<() => void>();

export function subscribeToRepromptExpiry(callback: () => void): () => void {
  onExpireCallbacks.add(callback);
  return () => {
    onExpireCallbacks.delete(callback);
  };
}

function clearExpiryTimer(): void {
  if (expiryTimer !== null) {
    clearTimeout(expiryTimer);
    expiryTimer = null;
  }
}

function scheduleExpiryTimer(): void {
  clearExpiryTimer();
  if (policy.mode !== "grace" || lastSuccessAt === null) return;
  const remainingMs = policy.graceSeconds * 1000 - (Date.now() - lastSuccessAt);
  // setTimeout(fn, <=0) fires on the next tick, which is what we want if the window is already
  // expired (e.g. the config's grace window shrank while one was already open).
  expiryTimer = setTimeout(() => {
    expiryTimer = null;
    // Re-mask every currently-revealed field together, THEN drop lastSuccessAt - a callback that
    // re-runs isRepromptRequired while re-masking should already see the window as closed.
    lastSuccessAt = null;
    for (const cb of onExpireCallbacks) cb();
  }, Math.max(0, remainingMs));
}

// Memoized so N mounted `SecretField`s cause exactly one `api.getConfig()` call. On failure the
// default policy stays in place and `loadPromise` is cleared so a later mount can retry.
export function ensureRepromptConfig(): Promise<void> {
  if (loadPromise === null) {
    loadPromise = api
      .getConfig()
      .then((c) => {
        policy = { mode: c.repromptMode, graceSeconds: c.repromptGraceSeconds };
        scheduleExpiryTimer();
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

export function isRepromptRequired(exempt: boolean): boolean {
  return repromptRequired(policy, exempt, lastSuccessAt, Date.now());
}

// Only records a success while in "grace" mode. Recording it in "always" mode would be harmless
// today (the pure function ignores `lastSuccessAt` in that mode) but would silently open a window
// the instant the user switched to "grace", which is the opposite of what they asked for - a mode
// switch should start cold.
export function markRepromptSuccess(): void {
  if (policy.mode === "grace") {
    lastSuccessAt = Date.now();
    scheduleExpiryTimer();
  }
}

// Called on the `vault-locked` event in both app shells - without this the grace window would
// outlive the lock, and the first reveal after unlocking would skip the prompt.
export function clearRepromptGrace(): void {
  lastSuccessAt = null;
  clearExpiryTimer();
}
