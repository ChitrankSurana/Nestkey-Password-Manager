import { useState, useEffect, FormEvent } from "react";
import { api, isCommandError, CredentialDto } from "../lib/api";
import { Tooltip } from "../components/Tooltip";
import { PasswordGeneratorPanel } from "../components/PasswordGeneratorPanel";

export function CredentialEditor({
  siteId,
  existing,
  onClose,
  onSaved,
}: {
  siteId: number;
  existing: CredentialDto | null;
  onClose: () => void;
  onSaved: () => void;
}) {
  const [username, setUsername] = useState(existing?.username ?? "");
  const [secret, setSecret] = useState(existing?.secret ?? "");
  const [remarks, setRemarks] = useState(existing?.remarks ?? "");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [algorithm, setAlgorithm] = useState("simple");

  useEffect(() => {
    api.getConfig().then((c) => setAlgorithm(c.strengthAlgorithm)).catch(() => {});
  }, []);

  async function handleSubmit(e: FormEvent) {
    e.preventDefault();
    setSubmitting(true);
    setError(null);
    try {
      if (existing) {
        await api.updateCredential(existing.id, username || null, secret, remarks || null);
      } else {
        await api.addCredential(siteId, username || null, secret, remarks || null);
      }
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
        <h2 className="text-lg font-semibold">{existing ? "Edit credential" : "Add credential"}</h2>
        <label className="block">
          <span className="text-sm">Username (optional)</span>
          <input
            className="mt-1 w-full rounded border px-2 py-1 dark:bg-slate-900 dark:border-slate-600"
            value={username}
            onChange={(e) => setUsername(e.target.value)}
            autoFocus
            title="Username or email for this login"
          />
        </label>
        <label className="block">
          <span className="text-sm">Password</span>
          <input
            className="mt-1 w-full rounded border px-2 py-1 dark:bg-slate-900 dark:border-slate-600"
            value={secret}
            onChange={(e) => setSecret(e.target.value)}
            title="The password to store, encrypted"
          />
        </label>

        <PasswordGeneratorPanel value={secret} onChange={setSecret} algorithm={algorithm} disabled={submitting} />

        <label className="block">
          <span className="text-sm">Remarks (optional)</span>
          <input
            className="mt-1 w-full rounded border px-2 py-1 dark:bg-slate-900 dark:border-slate-600"
            value={remarks}
            onChange={(e) => setRemarks(e.target.value)}
            title="Any notes about this login"
          />
        </label>
        {error && <p className="text-sm text-red-400">{error}</p>}
        <div className="flex justify-end space-x-2">
          <Tooltip text="Cancel">
            <button type="button" onClick={onClose} className="px-3 py-1">
              Cancel
            </button>
          </Tooltip>
          <Tooltip text="Save this credential">
            <button
              type="submit"
              disabled={submitting || secret.trim() === ""}
              className="rounded bg-[var(--accent)] px-3 py-1 disabled:opacity-50"
            >
              Save
            </button>
          </Tooltip>
        </div>
      </form>
    </div>
  );
}
