import { useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { api, DownloadCancelled, isCommandError, isTauri, ExportMode } from "../lib/api";
import { Tooltip } from "../components/Tooltip";
import { Icon } from "../components/Icon";

const CONFIRM_PHRASE = "EXPORT PLAINTEXT";

export function CsvExport({ onBack }: { onBack: () => void }) {
  const [mode, setMode] = useState<"thisDevice" | "withPassword" | "plaintext">("thisDevice");
  const [masterPassword, setMasterPassword] = useState("");
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [typedConfirm, setTypedConfirm] = useState("");
  const [status, setStatus] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const passwordModeReady = mode !== "withPassword" || (password.length > 0 && password === confirmPassword);
  const plaintextModeReady = mode !== "plaintext" || typedConfirm === CONFIRM_PHRASE;
  const canExport = masterPassword.length > 0 && passwordModeReady && plaintextModeReady && !busy;

  async function handleExport() {
    // Re-auth gate: exporting hands out every secret, so the master password is required in every
    // mode, on desktop and web alike.
    setError(null);
    setStatus(null);
    try {
      if (!(await api.verifyMasterPassword(masterPassword))) {
        setError("Incorrect master password.");
        return;
      }
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
      return;
    }
    setMasterPassword("");

    let target: string | null = null;
    if (isTauri) {
      const defaultName = mode === "plaintext" ? "vault-export.csv" : "vault-export.pwenc";
      target = await save({ defaultPath: defaultName });
      if (!target) return; // cancelled
    }

    setBusy(true);
    setError(null);
    setStatus(null);
    try {
      const exportMode: ExportMode =
        mode === "thisDevice" ? { mode: "thisDevice" } :
        mode === "withPassword" ? { mode: "withPassword", password } :
        { mode: "plaintext" };
      const outcome = await api.exportCsv(target, exportMode, mode === "plaintext");
      setStatus(
        isTauri
          ? `Exported to ${target}`
          : outcome === "saved"
            ? "Export saved."
            : "Export download started - check your browser's downloads.",
      );
    } catch (err) {
      if (err instanceof DownloadCancelled) return; // user closed the save dialog
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="min-h-screen bg-[var(--bg)] text-[var(--fg)] p-4">
      <Tooltip text="Back to vault">
        <button onClick={onBack} className="mb-4 rounded bg-[var(--accent)] px-3 py-1"><Icon name="arrow-left" /> Back</button>
      </Tooltip>
      <h1 className="text-xl font-semibold mb-4">Export to CSV</h1>
      {error && <p className="text-sm text-red-400 mb-4">{error}</p>}
      {status && <p className="text-sm text-green-400 mb-4">{status}</p>}

      <div className="mb-4 max-w-xs">
        <label className="block text-sm mb-1">Master password (required to export)</label>
        <input
          type="password"
          autoComplete="current-password"
          placeholder="Master password"
          className="w-full rounded border px-2 py-1 bg-[var(--surface)] border-[var(--border)]"
          value={masterPassword}
          onChange={(e) => setMasterPassword(e.target.value)}
        />
      </div>

      <fieldset className="space-y-2 max-w-md">
        <label className="flex items-center gap-2">
          <input type="radio" checked={mode === "thisDevice"} onChange={() => setMode("thisDevice")} />
          Encrypted (this device) — recommended
        </label>
        <label className="flex items-center gap-2">
          <input type="radio" checked={mode === "withPassword"} onChange={() => setMode("withPassword")} />
          Encrypted (with a password) — portable to another device
        </label>
        <label className="flex items-center gap-2">
          <input type="radio" checked={mode === "plaintext"} onChange={() => setMode("plaintext")} />
          Plaintext (unencrypted) — anyone with the file can read it
        </label>
      </fieldset>

      {mode === "withPassword" && (
        <div className="mt-3 max-w-xs space-y-2">
          <p className="text-xs opacity-70">
            This is a separate export password, not your master password. Only this app's own file
            format can open it — it is not a standard zip.
          </p>
          <input
            type="password"
            placeholder="Export password"
            className="w-full rounded border px-2 py-1 bg-[var(--surface)] border-[var(--border)]"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
          />
          <input
            type="password"
            placeholder="Confirm export password"
            className="w-full rounded border px-2 py-1 bg-[var(--surface)] border-[var(--border)]"
            value={confirmPassword}
            onChange={(e) => setConfirmPassword(e.target.value)}
          />
        </div>
      )}

      {mode === "plaintext" && (
        <div className="mt-3 max-w-xs space-y-2">
          <p className="text-sm text-red-400">
            This file will contain every password in your vault, completely unencrypted. Anyone who
            opens it can read everything.
          </p>
          <label className="block text-xs">
            Type <code>{CONFIRM_PHRASE}</code> to confirm:
          </label>
          <input
            className="w-full rounded border px-2 py-1 bg-[var(--surface)] border-[var(--border)]"
            value={typedConfirm}
            onChange={(e) => setTypedConfirm(e.target.value)}
          />
        </div>
      )}

      <Tooltip text="Choose where to save the export">
        <button
          onClick={handleExport}
          disabled={!canExport}
          className="mt-4 rounded bg-[var(--accent)] px-3 py-1 disabled:opacity-50"
        >
          <Icon name="download" /> Export
        </button>
      </Tooltip>
    </div>
  );
}
