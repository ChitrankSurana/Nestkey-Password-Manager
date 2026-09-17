import { useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { api, isCommandError, isTauri } from "../lib/api";
import { confirmDialog } from "../lib/confirm";
import { Tooltip } from "../components/Tooltip";
import { Icon } from "../components/Icon";

export function Backup({ onBack }: { onBack: () => void }) {
  const [status, setStatus] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function handleCreate() {
    let target: string | null = null;
    if (isTauri) {
      target = await save({
        defaultPath: "vault-backup.bak",
        filters: [{ name: "Backup", extensions: ["bak"] }],
      });
      if (!target) return; // cancelled - no busy state, no error, no status change
    }

    setBusy(true);
    try {
      setError(null);
      setStatus(null);
      const path = await api.createBackup(target);
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
        if (await confirmDialog(err.message)) {
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
    <div className="min-h-screen bg-[var(--bg)] text-[var(--fg)] p-4">
      <Tooltip text="Back to vault">
        <button onClick={onBack} className="mb-4 underline text-sm"><Icon name="arrow-left" /> Back</button>
      </Tooltip>
      <h1 className="text-xl font-semibold mb-4">Backup</h1>
      {error && <p className="text-sm text-red-400 mb-4">{error}</p>}
      {status && <p className="text-sm text-green-400 mb-4">{status}</p>}
      <div className="space-x-2">
        <Tooltip text="Write an encrypted backup of this vault - you'll be asked where to save it">
          <button onClick={handleCreate} disabled={busy} className="rounded bg-[var(--accent)] px-3 py-1 disabled:opacity-50">
            <Icon name="floppy2-fill" /> Create Backup
          </button>
        </Tooltip>
        <Tooltip text="Restore from a .bak file (this device or another)">
          <button onClick={handleRestore} disabled={busy} className="rounded bg-[var(--surface)] border border-[var(--border)] px-3 py-1 disabled:opacity-50">
            <Icon name="arrow-counterclockwise" /> Restore Backup
          </button>
        </Tooltip>
      </div>
    </div>
  );
}
