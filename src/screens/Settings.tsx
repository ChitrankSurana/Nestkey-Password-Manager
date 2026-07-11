import { useEffect, useRef, useState } from "react";
import { api, isCommandError } from "../lib/api";
import { Tooltip } from "../components/Tooltip";

const OPTIONS = [1, 5, 15] as const;

export function Settings({ onBack }: { onBack: () => void }) {
  const [autoLockMinutes, setAutoLockMinutes] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const editedRef = useRef(false);

  useEffect(() => {
    api
      .getConfig()
      .then((c) => {
        if (!editedRef.current) setAutoLockMinutes(c.autoLockMinutes);
      })
      .catch((err) => {
        setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
      });
  }, []);

  async function handleChange(minutes: number) {
    editedRef.current = true;
    setSubmitting(true);
    try {
      setError(null);
      await api.updateAutoLockMinutes(minutes);
      setAutoLockMinutes(minutes);
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="min-h-screen dark:bg-slate-900 dark:text-slate-100 p-4">
      <Tooltip text="Back to vault">
        <button onClick={onBack} className="mb-4 underline text-sm">
          ← Back
        </button>
      </Tooltip>
      <h1 className="text-xl font-semibold mb-4">Settings</h1>

      {error && <p className="text-sm text-red-400 mb-4">{error}</p>}

      <label className="block max-w-xs">
        <span className="text-sm">Auto-lock after idle for</span>
        <Tooltip text="How long to wait before auto-locking the vault" className="w-full">
          <select
            className="mt-1 w-full rounded border px-2 py-1 dark:bg-slate-800 dark:border-slate-600 disabled:opacity-50"
            value={autoLockMinutes ?? ""}
            onChange={(e) => handleChange(Number(e.target.value))}
            disabled={submitting}
          >
            {OPTIONS.map((m) => (
              <option key={m} value={m}>
                {m} minute{m === 1 ? "" : "s"}
              </option>
            ))}
          </select>
        </Tooltip>
      </label>
    </div>
  );
}
