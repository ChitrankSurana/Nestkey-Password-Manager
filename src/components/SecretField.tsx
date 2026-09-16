import { useState } from "react";
import { api } from "../lib/api";
import { toast } from "../lib/toast";
import { classifyChar } from "../lib/passwordColor";
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
// `reprompt` gates reveal/copy behind a master-password re-check (per-item "require master
// password to view" flag) even though the vault itself is already unlocked.
export function SecretField({ secret, reprompt }: { secret: string; reprompt?: boolean }) {
  const [revealed, setRevealed] = useState(false);
  const [pendingAction, setPendingAction] = useState<"reveal" | "copy" | null>(null);
  const [password, setPassword] = useState("");
  const [reauthError, setReauthError] = useState<string | null>(null);
  const [verifying, setVerifying] = useState(false);

  function runAction(action: "reveal" | "copy") {
    if (action === "reveal") {
      setRevealed((r) => !r);
    } else {
      api.copyToClipboard(secret);
      toast("Copied. It will clear from the clipboard in 30s.");
    }
  }

  function requestAction(action: "reveal" | "copy") {
    if (reprompt) {
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
