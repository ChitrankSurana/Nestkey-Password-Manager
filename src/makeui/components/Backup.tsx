import { useEffect, useRef, useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { api, DOWNLOADED, DownloadCancelled, holdLockRedirect, isCommandError, isTauri, releaseLockRedirect } from "../../lib/api";
import { confirmDialog } from "../../lib/confirm";
import { IconDownload, IconUpload } from "../icons";

// Restyled chrome onto mk-* tokens; same restore/lock-redirect logic as src/screens/Backup.tsx
// unchanged (holdLockRedirect/releaseLockRedirect, desktop-path vs. web-upload branching).
export function Backup({ onBack }: { onBack: () => void }) {
  const [status, setStatus] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [restored, setRestored] = useState(false);
  const restoredRef = useRef(false);
  const fileInput = useRef<HTMLInputElement>(null);

  useEffect(() => {
    return () => releaseLockRedirect(restoredRef.current);
  }, []);

  function markRestored() {
    restoredRef.current = true;
    setRestored(true);
    setStatus("Backup restored successfully. Log in with that account's master password.");
  }

  async function handleCreate() {
    let target: string | null = null;
    if (isTauri) {
      target = await save({
        defaultPath: "vault-backup.bak",
        filters: [{ name: "Backup", extensions: ["bak"] }],
      });
      if (!target) return;
    }

    setBusy(true);
    try {
      setError(null);
      setStatus(null);
      const path = await api.createBackup(target);
      setStatus(
        path === DOWNLOADED
          ? "Backup download started - check your browser's downloads."
          : path
            ? `Backup written to ${path}`
            : "Backup saved.",
      );
    } catch (err) {
      if (err instanceof DownloadCancelled) return;
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    } finally {
      setBusy(false);
    }
  }

  async function handleRestore() {
    if (isTauri) {
      const picked = await open({ multiple: false, filters: [{ name: "Backup", extensions: ["bak"] }] });
      if (typeof picked !== "string") return;
      await runRestore(picked);
    } else {
      fileInput.current?.click();
    }
  }

  async function runRestore(picked: string | null, file?: File) {
    setBusy(true);
    setError(null);
    setStatus(null);
    setRestored(false);
    restoredRef.current = false;
    holdLockRedirect();
    try {
      if (file) await api.uploadBackup(file);
      await api.restoreBackup(picked, false);
      markRestored();
    } catch (err) {
      if (isCommandError(err) && err.code === "confirm_required") {
        if (await confirmDialog(err.message)) {
          try {
            await api.restoreBackup(picked, true);
            markRestored();
          } catch (err2) {
            setError(isCommandError(err2) ? err2.message : "Something went wrong. Please try again.");
          }
        }
      } else {
        setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
      }
    } finally {
      setBusy(false);
      if (!restoredRef.current) releaseLockRedirect(false);
    }
  }

  return (
    <div className="flex-1 overflow-y-auto p-6">
      <div className="max-w-xl mx-auto space-y-4">
        <button onClick={onBack} className="text-xs font-medium text-mk-muted hover:text-mk-fg">
          ← Data Hub
        </button>
        <h2 className="text-sm font-semibold text-mk-fg">Backup &amp; Restore</h2>

        {error && <p className="text-sm text-mk-red">{error}</p>}
        {status && <p className="text-sm text-mk-green">{status}</p>}
        {restored && (
          <button onClick={() => releaseLockRedirect(true)} className="px-3 py-1.5 rounded-lg text-xs font-medium bg-mk-accent text-white">
            Go to login
          </button>
        )}

        <input
          ref={fileInput}
          type="file"
          accept=".bak"
          className="hidden"
          onChange={(e) => {
            const f = e.target.files?.[0];
            e.target.value = "";
            if (f) void runRestore(null, f);
          }}
        />

        <div className="rounded-xl border border-mk-border bg-mk-card p-5 space-y-3">
          <p className="text-xs text-mk-muted">
            Your backup will be encrypted with your master password. Keep it safe - without your master password, the backup cannot be opened.
          </p>
          <button
            onClick={handleCreate}
            disabled={busy}
            title="Write an encrypted backup of this vault"
            className="flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-semibold text-white bg-mk-accent hover:bg-mk-accent-hover disabled:opacity-50"
          >
            <IconDownload size={14} /> Create Backup
          </button>
        </div>

        <div className="rounded-xl border border-mk-border bg-mk-card p-5 space-y-3">
          <p className="text-xs text-mk-muted">Restoring a backup replaces your entire current vault. This action cannot be undone.</p>
          <button
            onClick={handleRestore}
            disabled={busy}
            title="Restore from a .bak file (this device or another)"
            className="flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-medium text-mk-fg2 bg-mk-badge hover:bg-mk-badge-active disabled:opacity-50"
          >
            <IconUpload size={14} /> Restore Backup
          </button>
        </div>
      </div>
    </div>
  );
}
