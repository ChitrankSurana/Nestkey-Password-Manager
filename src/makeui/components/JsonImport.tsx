import { useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api, isCommandError, isTauri, type ImportSummaryDto, type JsonImportPreviewDto } from "../../lib/api";
import { IconFolder, IconEye, IconUpload } from "../icons";

// Full-fidelity JSON vault import (Wave 4 follow-up) - unlike CsvImport.tsx there is no per-row
// editable table: an export can carry an arbitrary number of sites/credentials/custom fields/
// history entries, and editing that nested shape in a flat table isn't worth the complexity this
// pass budgeted for. Preview instead shows a count summary (sites, credentials, duplicates) so the
// user can see the outcome before committing, same intent as CSV's preview, smaller UI.
export function JsonImport({ onBack }: { onBack: () => void }) {
  const [path, setPath] = useState<string | null>(null);
  const [password, setPassword] = useState("");
  const [preview, setPreview] = useState<JsonImportPreviewDto | null>(null);
  const [summary, setSummary] = useState<ImportSummaryDto | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);

  function resetForNewFile() {
    setPreview(null);
    setSummary(null);
    setError(null);
  }

  async function handlePickFile() {
    if (isTauri) {
      const picked = await open({ multiple: false, filters: [{ name: "Vault export", extensions: ["json"] }] });
      if (typeof picked !== "string") return;
      setPath(picked);
      resetForNewFile();
      return;
    }
    fileInputRef.current?.click();
  }

  async function handleFileSelected(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    e.target.value = "";
    if (!file) return;
    setBusy(true);
    try {
      setError(null);
      await api.uploadJsonExport(file);
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
      setBusy(false);
      return;
    }
    setBusy(false);
    setPath(file.name);
    resetForNewFile();
  }

  async function handleBack() {
    if (!isTauri && path) {
      try {
        await api.clearUpload();
      } catch {
        // Best-effort cleanup - the session sweep would eventually reclaim it anyway.
      }
    }
    onBack();
  }

  async function handlePreview() {
    if (!path) return;
    setBusy(true);
    try {
      setError(null);
      setPreview(await api.importJsonPreview(isTauri ? path : null, password || null));
    } catch (err) {
      setPreview(null);
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    } finally {
      setBusy(false);
    }
  }

  async function handleCommit() {
    if (!path) return;
    setBusy(true);
    try {
      setError(null);
      setSummary(await api.importJsonCommit(isTauri ? path : null, password || null));
      setPreview(null);
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="flex-1 overflow-y-auto p-6">
      <div className="max-w-xl mx-auto space-y-4">
        <button onClick={handleBack} className="text-xs font-medium text-mk-muted hover:text-mk-fg">
          ← Data Hub
        </button>
        <h2 className="text-sm font-semibold text-mk-fg">Import from JSON (full fidelity)</h2>
        <p className="text-xs text-mk-faint">
          Only files exported from this app's own "Export to JSON" - not a standard format. Folders and sites already in this vault are matched by
          name and merged into, never overwritten; a credential already present (same site + username) is skipped, not duplicated.
        </p>

        {error && <p className="text-sm text-mk-red">{error}</p>}
        {summary && (
          <p className="text-sm text-mk-green">
            Imported {summary.imported}, skipped {summary.duplicates} duplicate(s).
          </p>
        )}

        <div className="rounded-xl border border-mk-border bg-mk-card p-5 space-y-4">
          <div className="flex flex-wrap gap-2 items-center">
            <button
              onClick={handlePickFile}
              disabled={busy}
              title="Choose the exported vault file"
              className="flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-medium text-mk-fg2 bg-mk-badge hover:bg-mk-badge-active disabled:opacity-50"
            >
              <IconFolder size={13} /> Choose File…
            </button>
            {!isTauri && <input ref={fileInputRef} type="file" accept=".json" className="hidden" onChange={handleFileSelected} />}
            {path && <span className="text-xs text-mk-faint">{path}</span>}
          </div>

          <label className="block">
            <span className="text-xs font-medium text-mk-muted">Export password (only needed for a password-protected export)</span>
            <input
              type="password"
              placeholder="Leave blank for a this-device or plaintext export"
              className="mt-1 w-full rounded-lg border px-3 py-1.5 bg-mk-input border-mk-border text-mk-fg text-sm"
              value={password}
              onChange={(e) => {
                setPassword(e.target.value);
                setPreview(null);
              }}
            />
          </label>

          <div className="flex gap-2">
            <button
              onClick={handlePreview}
              disabled={!path || busy}
              title="Check the file without importing anything yet"
              className="flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-semibold text-white bg-mk-accent hover:bg-mk-accent-hover disabled:opacity-50"
            >
              <IconEye size={14} /> Preview
            </button>
            {preview && (
              <button
                onClick={handleCommit}
                disabled={busy}
                title="Merge these records into your vault"
                className="flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-semibold text-white bg-mk-accent hover:bg-mk-accent-hover disabled:opacity-50"
              >
                <IconUpload size={14} /> Commit Import
              </button>
            )}
          </div>

          {preview && (
            <div className="text-sm text-mk-fg2 space-y-0.5">
              <p>{preview.folderCount} folder(s), {preview.siteCount} site(s), {preview.credentialCount} credential(s) in this file.</p>
              {preview.duplicateCredentialCount > 0 && (
                <p className="text-mk-faint">{preview.duplicateCredentialCount} credential(s) already exist and will be skipped.</p>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
