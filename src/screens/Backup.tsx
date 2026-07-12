import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api, isCommandError } from "../lib/api";
import { Tooltip } from "../components/Tooltip";

export function Backup({ onBack }: { onBack: () => void }) {
  const [status, setStatus] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function handleCreate() {
    setBusy(true);
    try {
      setError(null);
      setStatus(null);
      const path = await api.createBackup();
      setStatus(`Backup written to ${path}`);
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    } finally {
      setBusy(false);
    }
  }

  async function handleRestore() {
    const picked = await open({ multiple: false, filters: [{ name: "Backup", extensions: ["bak"] }] });
    if (typeof picked !== "string") return; // cancelled

    setBusy(true);
    setError(null);
    setStatus(null);
    try {
      await api.restoreBackup(picked, false);
      setStatus("Backup restored. Log in with that account's master password.");
    } catch (err) {
      if (isCommandError(err) && err.code === "confirm_required") {
        if (window.confirm(err.message)) {
          try {
            await api.restoreBackup(picked, true);
            setStatus("Backup restored. Log in with that account's master password.");
          } catch (err2) {
            setError(isCommandError(err2) ? err2.message : "Something went wrong. Please try again.");
          }
        }
      } else {
        setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
      }
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="min-h-screen dark:bg-slate-900 dark:text-slate-100 p-4">
      <Tooltip text="Back to vault">
        <button onClick={onBack} className="mb-4 underline text-sm">← Back</button>
      </Tooltip>
      <h1 className="text-xl font-semibold mb-4">Backup</h1>
      {error && <p className="text-sm text-red-400 mb-4">{error}</p>}
      {status && <p className="text-sm text-green-400 mb-4">{status}</p>}
      <div className="space-x-2">
        <Tooltip text="Write an encrypted backup of this vault to ~/.pwmgr/backups">
          <button onClick={handleCreate} disabled={busy} className="rounded bg-blue-600 px-3 py-1 disabled:opacity-50">
            Create Backup
          </button>
        </Tooltip>
        <Tooltip text="Restore from a .bak file (this device or another)">
          <button onClick={handleRestore} disabled={busy} className="rounded bg-slate-700 px-3 py-1 disabled:opacity-50">
            Restore Backup
          </button>
        </Tooltip>
      </div>
    </div>
  );
}
