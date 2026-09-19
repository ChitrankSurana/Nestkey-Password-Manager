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
  const [showPreview, setShowPreview] = useState(false);
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
      setShowPreview(false);
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
    setShowPreview(false);
    setSummary(null);
    setError(null);
    setHeaders([]);
    setMapping({});
    if (source === "generic") {
      await loadHeaders(null);
    }
  }

  async function handleSourceChange(next: "chromium" | "firefox" | "lastPass" | "generic") {
    // Web mode: switching source abandons this upload (SEC-01/D-04) - clear the server-side cache
    // rather than leaving credential bytes resident for a file the user is no longer importing.
    const hadWebUpload = !isTauri && path;
    if (hadWebUpload) {
      try {
        await api.clearUpload();
      } catch {
        // Best-effort cleanup - the session sweep would eventually reclaim it anyway.
      }
    }
    setSource(next);
    setRecords(null);
    setShowPreview(false);
    setSummary(null);
    setError(null);
    setHeaders([]);
    setMapping({});
    if (hadWebUpload) {
      setPath(null); // the cache backing this label is gone - the user must re-upload
    } else if (next === "generic" && path) {
      await loadHeaders(path);
    }
  }

  async function handleBack() {
    // Web mode: leaving the screen abandons any in-flight upload (SEC-01/D-04).
    if (!isTauri && path) {
      try {
        await api.clearUpload();
      } catch {
        // Best-effort cleanup - the session sweep would eventually reclaim it anyway.
      }
    }
    onBack();
  }

  function currentFormat(): ImportFormat {
    if (source === "chromium") return { format: "chromium" };
    if (source === "firefox") return { format: "firefox" };
    if (source === "lastPass") return { format: "lastPass" };
    return { format: "generic", mapping };
  }

  // Desktop needs a real path; web mode only needs "a file has been uploaded", which `path`
  // (the display label set on upload) already signals. Returns null on failure.
  // A newer request supersedes an older one, so rapid mapping changes cannot land out of order.
  const previewSeq = useRef(0);
  async function fetchRecords(format: ImportFormat = currentFormat()): Promise<ImportRecordDto[] | null> {
    if (!path) return null;
    const seq = ++previewSeq.current;
    setBusy(true);
    try {
      setError(null);
      const next = await api.importCsvPreview(isTauri ? path : null, format);
      if (seq === previewSeq.current) setRecords(next);
      return next;
    } catch (err) {
      if (seq === previewSeq.current) {
        setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
      }
      return null;
    } finally {
      if (seq === previewSeq.current) setBusy(false);
    }
  }

  // Toggle: hiding keeps parsed (and user-edited) records so showing again does not re-parse.
  async function handlePreview() {
    if (showPreview) {
      setShowPreview(false);
      return;
    }
    if (records === null && (await fetchRecords()) === null) return;
    setShowPreview(true);
  }

  function updateRecord(index: number, field: keyof ImportRecordDto, value: string) {
    if (!records) return;
    const next = [...records];
    next[index] = { ...next[index], [field]: value };
    setRecords(next);
  }

  async function handleCommit() {
    const toCommit = records ?? (await fetchRecords());
    if (!toCommit) return;
    setBusy(true);
    try {
      setError(null);
      setSummary(await api.importCsvCommit(toCommit));
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    } finally {
      setBusy(false);
    }
  }

  // Only once a file is uploaded and, for "Other", at least one column is mapped to a field.
  const canCommit = !!path && (source !== "generic" || Object.values(mapping).some((v) => v !== ""));

  return (
    <div className="min-h-screen bg-[var(--bg)] text-[var(--fg)] p-4">
      <Tooltip text="Back to vault">
        <button onClick={handleBack} className="mb-4 rounded bg-[var(--accent)] px-3 py-1"><Icon name="arrow-left" /> Back</button>
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
                onChange={(e) => {
                  const nextMapping = { ...mapping, [h]: e.target.value };
                  setMapping(nextMapping);
                  // Preview open: keep it open and re-parse live with the new mapping. Otherwise drop
                  // any stale parsed rows so a later Preview/Commit re-parses.
                  if (showPreview) void fetchRecords({ format: "generic", mapping: nextMapping });
                  else setRecords(null);
                }}
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
          <Icon name={showPreview ? "eye-slash-fill" : "eye-fill"} /> {showPreview ? "Hide Preview" : "Preview"}
        </button>
      </Tooltip>
      {canCommit && (
        <Tooltip text="Insert these records into your vault">
          <button onClick={handleCommit} disabled={busy} className="ml-2 rounded bg-[var(--accent)] px-3 py-1 disabled:opacity-50">
            <Icon name="upload" /> Commit Import
          </button>
        </Tooltip>
      )}

      {showPreview && records && (
        <div className="mt-4 max-h-96 overflow-auto rounded border border-[var(--border)]">
          <table className="w-full text-sm border-collapse">
            <thead className="sticky top-0 bg-[var(--surface)]">
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
        </div>
      )}
    </div>
  );
}
