import { useEffect, useState } from "react";
import { api, isCommandError, TrashedCredentialDto } from "../lib/api";
import { Tooltip } from "../components/Tooltip";
import { Icon } from "../components/Icon";

export function Trash({
  onBack,
  onRestored,
}: {
  onBack: () => void;
  // Fired after a successful restore so a parent holding its own separately-fetched
  // site/credential list (the New UI's NewShell keeps `useVault()` mounted across every pane
  // switch, unlike Classic's AppShell which remounts Vault.tsx on return) can refresh — without
  // this, restoring a cascade-deleted site's credential updates the DB correctly but the vault
  // view stays stale until the app restarts, reading as "restore did nothing" even though the
  // data was never lost.
  onRestored?: () => void;
}) {
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
    <div className="flex-1 bg-[var(--bg)] text-[var(--fg)] p-4">
      <Tooltip text="Back to vault">
        <button onClick={onBack} className="mb-4 rounded bg-[var(--accent)] px-3 py-1">
          <Icon name="arrow-left" /> Back
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
            className="rounded bg-[var(--danger)] hover:bg-[var(--danger-hover)] px-3 py-1"
            disabled={items.length === 0}
          >
            <Icon name="trash" /> Empty Trash
          </button>
        </Tooltip>
      </div>

      {error && <p className="text-sm text-red-400 mb-4">{error}</p>}

      <ul className="space-y-2">
        {items.map((t) => (
          <li key={t.credentialId} className="rounded border px-2 py-1 border-[var(--border)] flex justify-between">
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
                    onRestored?.();
                  } catch (err) {
                    setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
                  }
                }}
                className="underline text-sm"
              >
                <Icon name="arrow-counterclockwise" /> Restore
              </button>
            </Tooltip>
          </li>
        ))}
        {items.length === 0 && <li className="text-sm opacity-70">Trash is empty.</li>}
      </ul>
    </div>
  );
}
