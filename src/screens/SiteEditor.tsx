import { useState, FormEvent } from "react";
import { api, isCommandError } from "../lib/api";
import { Tooltip } from "../components/Tooltip";

export function SiteEditor({ onClose, onSaved }: { onClose: () => void; onSaved: () => void }) {
  const [name, setName] = useState("");
  const [url, setUrl] = useState("");
  const [notes, setNotes] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleSubmit(e: FormEvent) {
    e.preventDefault();
    setSubmitting(true);
    setError(null);
    try {
      await api.addSite(name, url || null, notes || null);
      onSaved();
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center">
      <form onSubmit={handleSubmit} className="w-80 space-y-3 rounded bg-slate-800 p-4">
        <h2 className="text-lg font-semibold">Add site</h2>
        <label className="block">
          <span className="text-sm">Name</span>
          <input
            className="mt-1 w-full rounded border px-2 py-1 dark:bg-slate-900 dark:border-slate-600"
            value={name}
            onChange={(e) => setName(e.target.value)}
            autoFocus
            title="Site name, e.g. GitHub"
          />
        </label>
        <label className="block">
          <span className="text-sm">URL (optional)</span>
          <input
            className="mt-1 w-full rounded border px-2 py-1 dark:bg-slate-900 dark:border-slate-600"
            value={url}
            onChange={(e) => setUrl(e.target.value)}
            title="Site URL"
          />
        </label>
        <label className="block">
          <span className="text-sm">Notes (optional)</span>
          <input
            className="mt-1 w-full rounded border px-2 py-1 dark:bg-slate-900 dark:border-slate-600"
            value={notes}
            onChange={(e) => setNotes(e.target.value)}
            title="Any notes about this site"
          />
        </label>
        {error && <p className="text-sm text-red-400">{error}</p>}
        <div className="flex justify-end space-x-2">
          <Tooltip text="Cancel">
            <button type="button" onClick={onClose} className="px-3 py-1">
              Cancel
            </button>
          </Tooltip>
          <Tooltip text="Save this site">
            <button
              type="submit"
              disabled={submitting || name.trim() === ""}
              className="rounded bg-blue-600 px-3 py-1 disabled:opacity-50"
            >
              Save
            </button>
          </Tooltip>
        </div>
      </form>
    </div>
  );
}
