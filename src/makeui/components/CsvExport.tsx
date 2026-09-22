import { useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { api, DownloadCancelled, isCommandError, isTauri, type ExportMode } from "../../lib/api";
import { IconDownload, IconAlertTriangle } from "../icons";

const CONFIRM_PHRASE = "EXPORT PLAINTEXT";

// Restyled chrome onto mk-* tokens; export logic (master-password re-auth gate, three export
// modes, desktop-save-dialog vs. web-download branching) unchanged from src/screens/CsvExport.tsx.
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
      if (!target) return;
    }

    setBusy(true);
    setError(null);
    setStatus(null);
    try {
      const exportMode: ExportMode =
        mode === "thisDevice" ? { mode: "thisDevice" } : mode === "withPassword" ? { mode: "withPassword", password } : { mode: "plaintext" };
      const outcome = await api.exportCsv(target, exportMode, mode === "plaintext");
      setStatus(
        isTauri ? `Exported to ${target}` : outcome === "saved" ? "Export saved." : "Export download started - check your browser's downloads.",
      );
    } catch (err) {
      if (err instanceof DownloadCancelled) return;
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="flex-1 overflow-y-auto p-6">
      <div className="max-w-xl mx-auto space-y-4">
        <button onClick={onBack} className="text-xs font-medium text-mk-muted hover:text-mk-fg">
          ← Data Hub
        </button>
        <h2 className="text-sm font-semibold text-mk-fg">Export to CSV</h2>
        <p className="text-xs text-mk-faint">
          CSV only carries site name, URL, notes, username, password, and remarks. Favorites, folders, item type, custom fields, TOTP, and
          password history are not included in any CSV export. For a complete, lossless copy of your vault, use Backup instead.
        </p>

        {error && <p className="text-sm text-mk-red">{error}</p>}
        {status && <p className="text-sm text-mk-green">{status}</p>}

        <div className="rounded-xl border border-mk-border bg-mk-card p-5 space-y-4">
          <label className="block">
            <span className="text-xs font-medium text-mk-muted">Master password (required to export)</span>
            <input
              type="password"
              autoComplete="current-password"
              placeholder="Master password"
              className="mt-1 w-full rounded-lg border px-3 py-1.5 bg-mk-input border-mk-border text-mk-fg text-sm"
              value={masterPassword}
              onChange={(e) => setMasterPassword(e.target.value)}
            />
          </label>

          <fieldset className="space-y-2">
            <label className="flex items-center gap-2 text-sm text-mk-fg2">
              <input type="radio" checked={mode === "thisDevice"} onChange={() => setMode("thisDevice")} />
              Encrypted (this device) — recommended
            </label>
            <label className="flex items-center gap-2 text-sm text-mk-fg2">
              <input type="radio" checked={mode === "withPassword"} onChange={() => setMode("withPassword")} />
              Encrypted (with a password) — portable to another device
            </label>
            <label className="flex items-center gap-2 text-sm text-mk-fg2">
              <input type="radio" checked={mode === "plaintext"} onChange={() => setMode("plaintext")} />
              Plaintext (unencrypted) — anyone with the file can read it
            </label>
          </fieldset>

          {mode === "withPassword" && (
            <div className="space-y-2">
              <p className="text-xs text-mk-faint">
                This is a separate export password, not your master password. Only this app's own file format can open it - it is not a standard
                zip.
              </p>
              <input
                type="password"
                placeholder="Export password"
                className="w-full rounded-lg border px-3 py-1.5 bg-mk-input border-mk-border text-mk-fg text-sm"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
              />
              <input
                type="password"
                placeholder="Confirm export password"
                className="w-full rounded-lg border px-3 py-1.5 bg-mk-input border-mk-border text-mk-fg text-sm"
                value={confirmPassword}
                onChange={(e) => setConfirmPassword(e.target.value)}
              />
            </div>
          )}

          {mode === "plaintext" && (
            <div className="space-y-2">
              <div className="flex items-start gap-2 px-3 py-2.5 rounded-lg" style={{ background: "color-mix(in srgb, var(--mk-red) 8%, transparent)", border: "1px solid color-mix(in srgb, var(--mk-red) 20%, transparent)" }}>
                <IconAlertTriangle size={14} className="text-mk-red flex-shrink-0 mt-0.5" />
                <p className="text-xs text-mk-red">
                  This file will contain every password in your vault, completely unencrypted. Anyone who opens it can read everything.
                </p>
              </div>
              <label className="block text-xs text-mk-muted">
                Type <code className="font-mk-mono text-mk-fg2">{CONFIRM_PHRASE}</code> to confirm:
              </label>
              <input
                className="w-full rounded-lg border px-3 py-1.5 bg-mk-input border-mk-border text-mk-fg text-sm"
                value={typedConfirm}
                onChange={(e) => setTypedConfirm(e.target.value)}
              />
            </div>
          )}

          <button
            onClick={handleExport}
            disabled={!canExport}
            title="Choose where to save the export"
            className="flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-semibold text-white bg-mk-accent hover:bg-mk-accent-hover disabled:opacity-50"
          >
            <IconDownload size={14} /> Export
          </button>
        </div>
      </div>
    </div>
  );
}
