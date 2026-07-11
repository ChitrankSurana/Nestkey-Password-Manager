import { useEffect, useState } from "react";
import { api, isCommandError, CredentialDto, SiteDto } from "../lib/api";
import { CredentialEditor } from "./CredentialEditor";

export function SiteDetail({ site, onBack }: { site: SiteDto; onBack: () => void }) {
  const [credentials, setCredentials] = useState<CredentialDto[]>([]);
  const [editing, setEditing] = useState<CredentialDto | null | "new">(null);
  const [error, setError] = useState<string | null>(null);

  async function refresh() {
    try {
      setError(null);
      setCredentials(await api.listCredentialsForSite(site.id));
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    }
  }

  useEffect(() => {
    refresh();
  }, [site.id]);

  return (
    <div className="min-h-screen dark:bg-slate-900 dark:text-slate-100 p-4">
      <button onClick={onBack} className="mb-4 underline text-sm" title="Back to vault list">
        ← Back
      </button>
      <div className="flex justify-between items-center mb-4">
        <h1 className="text-xl font-semibold">{site.name}</h1>
        <button
          onClick={() => setEditing("new")}
          className="rounded bg-blue-600 px-3 py-1"
          title="Add a credential to this site"
        >
          Add credential
        </button>
      </div>

      {error && <p className="text-sm text-red-400 mb-4">{error}</p>}

      <ul className="space-y-2">
        {credentials.map((c) => (
          <li key={c.id} className="rounded border px-2 py-1 dark:border-slate-700 flex justify-between">
            <span>{c.username ?? "(no username)"}</span>
            <div className="space-x-2">
              <button onClick={() => setEditing(c)} className="underline text-sm" title="Edit this credential">
                Edit
              </button>
              <button
                onClick={async () => {
                  try {
                    setError(null);
                    await api.softDeleteCredential(c.id);
                    refresh();
                  } catch (err) {
                    setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
                  }
                }}
                className="underline text-sm"
                title="Move this credential to Trash"
              >
                Delete
              </button>
            </div>
          </li>
        ))}
        {credentials.length === 0 && <li className="text-sm opacity-70">No credentials yet.</li>}
      </ul>

      {editing !== null && (
        <CredentialEditor
          siteId={site.id}
          existing={editing === "new" ? null : editing}
          onClose={() => setEditing(null)}
          onSaved={() => {
            setEditing(null);
            refresh();
          }}
        />
      )}
    </div>
  );
}
