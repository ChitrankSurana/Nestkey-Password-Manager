import { useEffect, useState } from "react";
import { api, isCommandError, TrashedCredentialDto } from "../lib/api";
import { Tooltip } from "../components/Tooltip";

export function Trash({ onBack }: { onBack: () => void }) {
  const [items, setItems] = useState<TrashedCredentialDto[]>([]);
  const [error, setError] = useState<string | null>(null);

  async function refresh() {
    try {
      setError(null);
      setItems(await api.listTrash());
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    }
  }

  useEffect(() => {
    refresh();
  }, []);

  return (
    <div className="min-h-screen dark:bg-slate-900 dark:text-slate-100 p-4">
      <Tooltip text="Back to vault">
        <button onClick={onBack} className="mb-4 underline text-sm">
          ← Back
        </button>
      </Tooltip>
      <div className="flex justify-between items-center mb-4">
        <h1 className="text-xl font-semibold">Trash</h1>
        <Tooltip text="Permanently delete everything in Trash">
          <button
            onClick={async () => {
              if (!window.confirm(`Permanently delete ${items.length} item(s)? This cannot be undone.`)) {
                return;
              }
              try {
                setError(null);
                await api.purgeTrash();
                refresh();
              } catch (err) {
                setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
              }
            }}
            className="rounded bg-red-700 px-3 py-1"
            disabled={items.length === 0}
          >
            Empty Trash
          </button>
        </Tooltip>
      </div>

      {error && <p className="text-sm text-red-400 mb-4">{error}</p>}

      <ul className="space-y-2">
        {items.map((t) => (
          <li key={t.credentialId} className="rounded border px-2 py-1 dark:border-slate-700 flex justify-between">
            <span>
              {t.siteName} — {t.username ?? "(no username)"}
            </span>
            <Tooltip text="Restore this credential">
              <button
                onClick={async () => {
                  try {
                    setError(null);
                    await api.restoreCredential(t.credentialId);
                    refresh();
                  } catch (err) {
                    setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
                  }
                }}
                className="underline text-sm"
              >
                Restore
              </button>
            </Tooltip>
          </li>
        ))}
        {items.length === 0 && <li className="text-sm opacity-70">Trash is empty.</li>}
      </ul>
    </div>
  );
}
