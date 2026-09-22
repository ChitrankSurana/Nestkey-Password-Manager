import { useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api, isCommandError, isTauri, type ImportFormat, type ImportRecordDto, type ImportSummaryDto } from "../../lib/api";
import { IconFolder, IconEye, IconEyeOff, IconUpload } from "../icons";

const TARGET_FIELDS = ["site_name", "url", "notes", "username", "secret", "remarks"] as const;

// Restyled chrome onto mk-* tokens; import logic (source-switch cache clearing, generic-mapping
// preview, commit) unchanged from src/screens/CsvImport.tsx.
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
    fileInputRef.current?.click();
  }

  async function handleFileSelected(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    e.target.value = "";
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

    setPath(file.name);
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
      setPath(null);
    } else if (next === "generic" && path) {
      await loadHeaders(path);
    }
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

  function currentFormat(): ImportFormat {
    if (source === "chromium") return { format: "chromium" };
    if (source === "firefox") return { format: "firefox" };
    if (source === "lastPass") return { format: "lastPass" };
    return { format: "generic", mapping };
  }

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

  const canCommit = !!path && (source !== "generic" || Object.values(mapping).some((v) => v !== ""));

  return (
    <div className="flex-1 overflow-y-auto p-6">
      <div className="max-w-3xl mx-auto space-y-4">
        <button onClick={handleBack} className="text-xs font-medium text-mk-muted hover:text-mk-fg">
          ← Data Hub
        </button>
        <h2 className="text-sm font-semibold text-mk-fg">Import from CSV</h2>

        {error && <p className="text-sm text-mk-red">{error}</p>}
        {summary && (
          <p className="text-sm text-mk-green">
            Imported {summary.imported}, skipped {summary.duplicates} duplicate(s).
          </p>
        )}

        <div className="rounded-xl border border-mk-border bg-mk-card p-5 space-y-4">
          <div className="flex flex-wrap gap-2 items-center">
            <select
              className="rounded-lg border px-2 py-1.5 bg-mk-input border-mk-border text-mk-fg text-sm"
              value={source}
              disabled={busy}
              onChange={(e) => handleSourceChange(e.target.value as typeof source)}
            >
              <option value="chromium">Chrome / Chromium export</option>
              <option value="firefox">Firefox export</option>
              <option value="lastPass">LastPass export</option>
              <option value="generic">Other (map columns myself)</option>
            </select>
            <button
              onClick={handlePickFile}
              disabled={busy}
              title="Choose the CSV file to import"
              className="flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-medium text-mk-fg2 bg-mk-badge hover:bg-mk-badge-active disabled:opacity-50"
            >
              <IconFolder size={13} /> Choose File…
            </button>
            {!isTauri && <input ref={fileInputRef} type="file" accept=".csv,text/csv" className="hidden" onChange={handleFileSelected} />}
            {path && <span className="text-xs text-mk-faint">{path}</span>}
          </div>

          {source === "generic" && headers.length > 0 && (
            <div className="space-y-1.5">
              <p className="text-sm text-mk-fg2">Map each column you want to import:</p>
              {headers.map((h) => (
                <label key={h} className="flex items-center gap-2 text-sm text-mk-fg2">
                  <span className="w-40 truncate">{h}</span>
                  <select
                    className="rounded-lg border px-2 py-1 bg-mk-input border-mk-border text-mk-fg text-sm"
                    value={mapping[h] ?? ""}
                    onChange={(e) => {
                      const nextMapping = { ...mapping, [h]: e.target.value };
                      setMapping(nextMapping);
                      if (showPreview) void fetchRecords({ format: "generic", mapping: nextMapping });
                      else setRecords(null);
                    }}
                  >
                    <option value="">(ignore)</option>
                    {TARGET_FIELDS.map((f) => (
                      <option key={f} value={f}>
                        {f}
                      </option>
                    ))}
                  </select>
                </label>
              ))}
            </div>
          )}

          <div className="flex gap-2">
            <button
              onClick={handlePreview}
              disabled={!path || busy}
              title="Parse the file without saving anything yet"
              className="flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-semibold text-white bg-mk-accent hover:bg-mk-accent-hover disabled:opacity-50"
            >
              {showPreview ? <IconEyeOff size={14} /> : <IconEye size={14} />} {showPreview ? "Hide Preview" : "Preview"}
            </button>
            {canCommit && (
              <button
                onClick={handleCommit}
                disabled={busy}
                title="Insert these records into your vault"
                className="flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-semibold text-white bg-mk-accent hover:bg-mk-accent-hover disabled:opacity-50"
              >
                <IconUpload size={14} /> Commit Import
              </button>
            )}
          </div>
        </div>

        {showPreview && records && (
          <div className="max-h-96 overflow-auto rounded-xl border border-mk-border bg-mk-card">
            <table className="w-full text-sm border-collapse">
              <thead className="sticky top-0 bg-mk-panel">
                <tr>
                  {["Site", "URL", "Username", "Secret", "Remarks"].map((h) => (
                    <th key={h} className="text-left border-b px-3 py-2 border-mk-border text-mk-muted font-medium">
                      {h}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {records.map((r, i) => (
                  <tr key={i} className="border-b border-mk-border last:border-b-0">
                    <td className="px-3 py-1.5">
                      <input className="w-full bg-transparent text-mk-fg2" value={r.siteName} onChange={(e) => updateRecord(i, "siteName", e.target.value)} />
                    </td>
                    <td className="px-3 py-1.5">
                      <input className="w-full bg-transparent text-mk-fg2" value={r.url ?? ""} onChange={(e) => updateRecord(i, "url", e.target.value)} />
                    </td>
                    <td className="px-3 py-1.5">
                      <input className="w-full bg-transparent text-mk-fg2" value={r.username ?? ""} onChange={(e) => updateRecord(i, "username", e.target.value)} />
                    </td>
                    <td className="px-3 py-1.5">
                      <input className="w-full bg-transparent text-mk-fg2" value={r.secret} onChange={(e) => updateRecord(i, "secret", e.target.value)} />
                    </td>
                    <td className="px-3 py-1.5">
                      <input className="w-full bg-transparent text-mk-fg2" value={r.remarks ?? ""} onChange={(e) => updateRecord(i, "remarks", e.target.value)} />
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
}
