import { useEffect, useState } from "react";
import { api } from "../lib/api";
import { toast } from "../lib/toast";
import { classifyChar } from "../lib/passwordColor";
import {
  ensureRepromptConfig,
  isRepromptRequired,
  markRepromptSuccess,
  subscribeToRepromptExpiry,
} from "../lib/reprompt";
import { Icon } from "./Icon";
import { Spinner } from "./Spinner";

const CLASS_COLOR: Record<"digit" | "symbol", string> = {
  digit: "text-[var(--pw-digit)]",
  symbol: "text-[var(--pw-symbol)]",
};

// Extracted from ViewerVault's inline reveal/copy logic (the only place this app previously
// exposed password reveal+copy) so SiteDetail can use the same behavior on the main vault view.
// Colorizes digits/symbols on reveal (Bitwarden's bit-color-password idea) - letters inherit the
// default text color, only digit/symbol runs get a wrapping span.
//
// B7: every credential is gated by the master password now. `exempt` (the credential's own
// `repromptExempt` flag, inverted from its pre-B7 meaning) is a per-credential opt-OUT of that
// gate - the exception, not the rule. One correct entry opens a shared grace window (in "grace"
// mode) read by every mounted SecretField; see src/lib/reprompt.ts and src/lib/repromptPolicy.ts.
export function SecretField({ secret, exempt }: { secret: string; exempt?: boolean }) {
  const [revealed, setRevealed] = useState(false);
  const [pendingAction, setPendingAction] = useState<"reveal" | "copy" | null>(null);
  const [password, setPassword] = useState("");
  const [reauthError, setReauthError] = useState<string | null>(null);
  const [verifying, setVerifying] = useState(false);

  useEffect(() => {
    // Loads the reprompt policy well before any click. Memoized in src/lib/reprompt.ts, so this
    // is cheap regardless of how many SecretField instances mount.
    void ensureRepromptConfig();
  }, []);

  useEffect(() => {
    // B7: re-mask this field the instant the shared grace window expires, rather than leaving a
    // revealed secret on screen indefinitely until some unrelated action happens to re-check the
    // gate. Only subscribes while actually revealed - an already-masked field has nothing to do.
    if (!revealed) return;
    return subscribeToRepromptExpiry(() => setRevealed(false));
  }, [revealed]);

  function runAction(action: "reveal" | "copy") {
    if (action === "reveal") {
      setRevealed((r) => !r);
    } else {
      api.copyToClipboard(secret);
      toast("Copied. It will clear from the clipboard in 30s.");
    }
  }

  function requestAction(action: "reveal" | "copy") {
    // B7: hiding an already-revealed value must never itself prompt - only the reveal direction
    // is gated. Previously this ran the same gate for both directions, so hiding a field after
    // the grace window expired would (bizarrely) ask for the master password just to mask it.
    if (action === "reveal" && revealed) {
      setRevealed(false);
      return;
    }
    // Decides through the shared reprompt policy instead of a bare flag check (G-08-2). If the
    // config read hasn't resolved yet, the default policy still gates every reveal - see
    // DEFAULT_REPROMPT_POLICY - so this never fails open on a slow config load. The reprompt gate
    // is a convenience gate over an already-unlocked vault, not a security boundary
    // (SECURITY_CONCERNS.md records that decrypted secrets are in the DOM while unlocked).
    if (isRepromptRequired(exempt === true)) {
      setPendingAction(action);
    } else {
      runAction(action);
    }
  }

  async function confirmReauth() {
    setVerifying(true);
    setReauthError(null);
    try {
      const ok = await api.verifyMasterPassword(password);
      if (!ok) {
        setReauthError("Incorrect master password.");
        return;
      }
      // Only after the `ok` check: a failed attempt must never open the Global-mode grace window.
      markRepromptSuccess();
      const action = pendingAction!;
      setPendingAction(null);
      setPassword("");
      runAction(action);
    } catch {
      setReauthError("Something went wrong. Please try again.");
    } finally {
      setVerifying(false);
    }
  }

  return (
    <>
      <span className="flex-1 font-mono">
        {revealed
          ? [...secret].map((ch, i) => {
              const cls = classifyChar(ch);
              return cls ? (
                <span key={i} className={CLASS_COLOR[cls]}>
                  {ch}
                </span>
              ) : (
                ch
              );
            })
          : "••••••••"}
      </span>
      <button
        type="button"
        title={revealed ? "Hide" : "Show"}
        onClick={() => requestAction("reveal")}
        className="rounded px-2 py-0.5 border border-[var(--border)]"
      >
        <Icon name={revealed ? "eye-slash-fill" : "eye-fill"} />
      </button>
      <button
        type="button"
        title="Copy"
        onClick={() => requestAction("copy")}
        className="rounded px-2 py-0.5 border border-[var(--border)]"
      >
        <Icon name="clipboard" /> Copy
      </button>

      {pendingAction !== null && (
        <div className="fixed inset-0 z-[70] bg-black/50 flex items-center justify-center p-4">
          <form
            onSubmit={(e) => {
              e.preventDefault();
              confirmReauth();
            }}
            className="w-full max-w-72 space-y-3 rounded bg-[var(--surface)] p-4 text-[var(--fg)]"
          >
            <h2 className="text-sm font-semibold">Re-enter your master password</h2>
            <input
              type="password"
              autoFocus
              className="w-full rounded border px-2 py-1 bg-[var(--bg)] border-[var(--border)]"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
            />
            {reauthError && <p className="text-sm text-red-400">{reauthError}</p>}
            <div className="flex justify-end gap-2">
              <button
                type="button"
                onClick={() => {
                  setPendingAction(null);
                  setPassword("");
                  setReauthError(null);
                }}
                className="px-3 py-1 text-sm"
              >
                Cancel
              </button>
              <button
                type="submit"
                disabled={verifying || password === ""}
                className="rounded bg-[var(--accent)] px-3 py-1 text-sm disabled:opacity-50"
              >
                {verifying ? (
                  <>
                    <Spinner className="mr-1.5 inline-block" />
                    Checking…
                  </>
                ) : (
                  "Confirm"
                )}
              </button>
            </div>
          </form>
        </div>
      )}
    </>
  );
}
