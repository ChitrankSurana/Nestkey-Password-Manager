import { useState, FormEvent } from "react";
import { api, isCommandError, SiteDto } from "../lib/api";
import { Tooltip } from "../components/Tooltip";
import { Icon } from "../components/Icon";

export function SiteEditor({
  existing = null,
  onClose,
  onSaved,
  onDeleted,
}: {
  existing?: SiteDto | null;
  onClose: () => void;
  onSaved: () => void;
  onDeleted?: () => void;
}) {
  const [name, setName] = useState(existing?.name ?? "");
  const [url, setUrl] = useState(existing?.url ?? "");
  const [notes, setNotes] = useState(existing?.notes ?? "");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleSubmit(e: FormEvent) {
    e.preventDefault();
    setSubmitting(true);
    setError(null);
    try {
      if (existing) {
        await api.updateSite(existing.id, name, url || null, notes || null);
      } else {
        await api.addSite(name, url || null, notes || null);
      }
      onSaved();
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    } finally {
      setSubmitting(false);
    }
  }

  async function handleDelete() {
    if (!existing) return;
    if (!window.confirm(`Delete "${existing.name}"? This cannot be undone.`)) return;
    setSubmitting(true);
    setError(null);
    try {
      await api.softDeleteSite(existing.id);
      onDeleted?.();
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="fixed inset-0 z-[60] bg-black/50 flex items-center justify-center p-4">
      <form onSubmit={handleSubmit} className="w-full max-w-80 space-y-3 rounded bg-[var(--surface)] p-4 text-[var(--fg)]">
        <h2 className="text-lg font-semibold">{existing ? "Edit site" : "Add site"}</h2>
        <label className="block">
          <span className="text-sm">Name</span>
          <input
            className="mt-1 w-full rounded border px-2 py-1 bg-[var(--bg)] border-[var(--border)]"
            value={name}
            onChange={(e) => setName(e.target.value)}
            autoFocus
            title="Site name, e.g. GitHub"
          />
        </label>
        <label className="block">
          <span className="text-sm">URL (optional)</span>
          <input
            className="mt-1 w-full rounded border px-2 py-1 bg-[var(--bg)] border-[var(--border)]"
            value={url}
            onChange={(e) => setUrl(e.target.value)}
            title="Site URL"
          />
        </label>
        <label className="block">
          <span className="text-sm">Notes (optional)</span>
          <input
            className="mt-1 w-full rounded border px-2 py-1 bg-[var(--bg)] border-[var(--border)]"
            value={notes}
            onChange={(e) => setNotes(e.target.value)}
            title="Any notes about this site"
          />
        </label>
        {error && <p className="text-sm text-red-400">{error}</p>}
        <div className="flex justify-between items-center">
          {existing ? (
            <Tooltip text="Delete this site (only if it has no credentials)">
              <button
                type="button"
                onClick={handleDelete}
                disabled={submitting}
                className="rounded bg-[var(--danger)] hover:bg-[var(--danger-hover)] px-3 py-1 disabled:opacity-50"
              >
                <Icon name="trash" /> Delete
              </button>
            </Tooltip>
          ) : (
            <span />
          )}
          <div className="flex space-x-2">
            <Tooltip text="Cancel">
              <button type="button" onClick={onClose} className="px-3 py-1">
                <Icon name="x" /> Cancel
              </button>
            </Tooltip>
            <Tooltip text="Save this site">
              <button
                type="submit"
                disabled={submitting || name.trim() === ""}
                className="rounded bg-[var(--accent)] px-3 py-1 disabled:opacity-50"
              >
                <Icon name="check-lg" /> Save
              </button>
            </Tooltip>
          </div>
        </div>
      </form>
    </div>
  );
}
