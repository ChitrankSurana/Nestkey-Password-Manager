import { FormEvent, useState } from "react";
import { api } from "../lib/api";
import { Spinner } from "./Spinner";

// Modal re-auth gate for sensitive setting changes. Verifies the master password itself so a wrong
// entry keeps the dialog open; `onResult(true)` only after a correct one, `onResult(false)` on cancel.
export function MasterPasswordPrompt({
  title,
  onResult,
}: {
  title: string;
  onResult: (ok: boolean) => void;
}) {
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [verifying, setVerifying] = useState(false);

  async function handleSubmit(e: FormEvent) {
    e.preventDefault();
    setVerifying(true);
    setError(null);
    try {
      if (await api.verifyMasterPassword(password)) {
        onResult(true);
      } else {
        setError("Incorrect master password.");
      }
    } catch {
      setError("Something went wrong. Please try again.");
    } finally {
      setVerifying(false);
    }
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4">
      <form
        onSubmit={handleSubmit}
        className="w-full max-w-sm space-y-3 rounded bg-[var(--surface)] p-4 text-[var(--fg)]"
      >
        <h2 className="text-lg font-semibold">{title}</h2>
        <p className="text-sm opacity-70">Enter your master password to continue.</p>
        <input
          type="password"
          autoComplete="current-password"
          autoFocus
          className="w-full rounded border px-2 py-1 bg-[var(--bg)] border-[var(--border)]"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          placeholder="Master password"
        />
        {error && <p className="text-sm text-red-400">{error}</p>}
        <div className="flex justify-end gap-2">
          <button
            type="button"
            onClick={() => onResult(false)}
            className="rounded border px-3 py-1 text-sm border-[var(--border)]"
          >
            Cancel
          </button>
          <button
            type="submit"
            disabled={verifying || password === ""}
            className="rounded bg-[var(--accent)] px-3 py-1 text-sm disabled:opacity-50"
          >
            {verifying ? <Spinner /> : "Confirm"}
          </button>
        </div>
      </form>
    </div>
  );
}
