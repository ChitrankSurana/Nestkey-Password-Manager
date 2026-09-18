import { useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api, isCommandError, isTauri, ImportFormat, ImportRecordDto, ImportSummaryDto } from "../lib/api";
import { Tooltip } from "../components/Tooltip";
import { Icon } from "../components/Icon";

const TARGET_FIELDS = ["site_name", "url", "notes", "username", "secret", "remarks"] as const;

export function CsvImport({ onBack }: { onBack: () => void }) {
  const [path, setPath] = useState<string | null>(null);
  const [source, setSource] = useState<"chromium" | "firefox" | "lastPass" | "generic">("chromium");
  const [headers, setHeaders] = useState<string[]>([]);
  const [mapping, setMapping] = useState<Record<string, string>>({});
  const [records, setRecords] = useState<ImportRecordDto[] | null>(null);
  const [summary, setSummary] = useState<ImportSummaryDto | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);

  async function loadHeaders(filePath: string | null) {
    setBusy(true);
    try {
      setError(null);
      setHeaders(await api.readCsvHeaders(filePath));
    } catch (err) {
      setHeaders([]);
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    } finally {
      setBusy(false);
    }
  }

  async function handlePickFile() {
    if (isTauri) {
      const picked = await open({ multiple: false, filters: [{ name: "CSV", extensions: ["csv"] }] });
      if (typeof picked !== "string") return;
      setPath(picked);
      setRecords(null);
      setSummary(null);
      setError(null);
      setHeaders([]);
      setMapping({});
      if (source === "generic") {
        await loadHeaders(picked);
      }
      return;
    }
    // Web mode: no native open dialog - drive a hidden <input type="file"> instead (SEC-01/D-01).
    fileInputRef.current?.click();
  }

  async function handleFileSelected(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    e.target.value = ""; // allow re-selecting the same file later
    if (!file) return;

    setBusy(true);
    try {
      setError(null);
      await api.uploadCsv(file);
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
      setBusy(false);
      return;
    }
    setBusy(false);

    setPath(file.name); // display label only - never sent to the server
    setRecords(null);
    setSummary(null);
    setError(null);
    setHeaders([]);
    setMapping({});
    if (source === "generic") {
      await loadHeaders(null);
    }
  }

  async function handleSourceChange(next: "chromium" | "firefox" | "lastPass" | "generic") {
    setSource(next);
    setRecords(null);
    setSummary(null);
    setError(null);
    setHeaders([]);
    setMapping({});
    if (next === "generic" && path) {
      await loadHeaders(isTauri ? path : null);
    }
  }

  function currentFormat(): ImportFormat {
    if (source === "chromium") return { format: "chromium" };
    if (source === "firefox") return { format: "firefox" };
    if (source === "lastPass") return { format: "lastPass" };
    return { format: "generic", mapping };
  }

  async function handlePreview() {
    if (!path) return;
    setBusy(true);
    try {
      setError(null);
      setRecords(await api.importCsvPreview(path, currentFormat()));
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    } finally {
      setBusy(false);
    }
  }

  function updateRecord(index: number, field: keyof ImportRecordDto, value: string) {
    if (!records) return;
    const next = [...records];
    next[index] = { ...next[index], [field]: value };
    setRecords(next);
  }

  async function handleCommit() {
    if (!records) return;
    setBusy(true);
    try {
      setError(null);
      setSummary(await api.importCsvCommit(records));
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="min-h-screen bg-[var(--bg)] text-[var(--fg)] p-4">
      <Tooltip text="Back to vault">
        <button onClick={onBack} className="mb-4 underline text-sm"><Icon name="arrow-left" /> Back</button>
      </Tooltip>
      <h1 className="text-xl font-semibold mb-4">Import from CSV</h1>
      {error && <p className="text-sm text-red-400 mb-4">{error}</p>}
      {summary && (
        <p className="text-sm text-green-400 mb-4">
          Imported {summary.imported}, skipped {summary.duplicates} duplicate(s).
        </p>
      )}

      <div className="flex gap-2 items-center mb-2">
        <select
          className="rounded border px-2 py-1 bg-[var(--surface)] border-[var(--border)]"
          value={source}
          disabled={busy}
          onChange={(e) => handleSourceChange(e.target.value as typeof source)}
        >
          <option value="chromium">Chrome / Chromium export</option>
          <option value="firefox">Firefox export</option>
          <option value="lastPass">LastPass export</option>
          <option value="generic">Other (map columns myself)</option>
        </select>
        <Tooltip text="Choose the CSV file to import">
          <button onClick={handlePickFile} disabled={busy} className="rounded bg-[var(--surface)] border border-[var(--border)] px-3 py-1 disabled:opacity-50">
            <Icon name="folder2-open" /> Choose File…
          </button>
        </Tooltip>
        {!isTauri && (
          <input
            ref={fileInputRef}
            type="file"
            accept=".csv,text/csv"
            className="hidden"
            onChange={handleFileSelected}
          />
        )}
        {path && <span className="text-xs opacity-70">{path}</span>}
      </div>

      {source === "generic" && headers.length > 0 && (
        <div className="mb-4 space-y-1">
          <p className="text-sm">Map each column you want to import:</p>
          {headers.map((h) => (
            <label key={h} className="flex items-center gap-2 text-sm">
              <span className="w-40 truncate">{h}</span>
              <select
                className="rounded border px-2 py-1 bg-[var(--surface)] border-[var(--border)]"
                value={mapping[h] ?? ""}
                onChange={(e) =>
                  setMapping((m) => ({ ...m, [h]: e.target.value }))
                }
              >
                <option value="">(ignore)</option>
                {TARGET_FIELDS.map((f) => (
                  <option key={f} value={f}>{f}</option>
                ))}
              </select>
            </label>
          ))}
        </div>
      )}

      <Tooltip text="Parse the file without saving anything yet">
        <button onClick={handlePreview} disabled={!path || busy} className="rounded bg-[var(--accent)] px-3 py-1 disabled:opacity-50">
          <Icon name="eye-fill" /> Preview
        </button>
      </Tooltip>

      {records && (
        <div className="mt-4 overflow-x-auto">
          <table className="w-full text-sm border-collapse">
            <thead>
              <tr>
                {["Site", "URL", "Username", "Secret", "Remarks"].map((h) => (
                  <th key={h} className="text-left border-b px-2 py-1 border-[var(--border)]">{h}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {records.map((r, i) => (
                <tr key={i}>
                  <td><input className="w-full bg-transparent" value={r.siteName} onChange={(e) => updateRecord(i, "siteName", e.target.value)} /></td>
                  <td><input className="w-full bg-transparent" value={r.url ?? ""} onChange={(e) => updateRecord(i, "url", e.target.value)} /></td>
                  <td><input className="w-full bg-transparent" value={r.username ?? ""} onChange={(e) => updateRecord(i, "username", e.target.value)} /></td>
                  <td><input className="w-full bg-transparent" value={r.secret} onChange={(e) => updateRecord(i, "secret", e.target.value)} /></td>
                  <td><input className="w-full bg-transparent" value={r.remarks ?? ""} onChange={(e) => updateRecord(i, "remarks", e.target.value)} /></td>
                </tr>
              ))}
            </tbody>
          </table>
          <Tooltip text="Insert these records into your vault">
            <button onClick={handleCommit} disabled={busy} className="mt-3 rounded bg-[var(--accent)] px-3 py-1 disabled:opacity-50">
              <Icon name="upload" /> Commit Import
            </button>
          </Tooltip>
        </div>
      )}
    </div>
  );
}
