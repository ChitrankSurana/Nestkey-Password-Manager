import { useEffect, useState } from "react";
import { api, isCommandError, CredentialDto, SiteDto } from "../lib/api";
import { CredentialEditor } from "./CredentialEditor";
import { Tooltip } from "../components/Tooltip";

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
      <Tooltip text="Back to vault list">
        <button onClick={onBack} className="mb-4 underline text-sm">
          ← Back
        </button>
      </Tooltip>
      <div className="flex justify-between items-center mb-4">
        <h1 className="text-xl font-semibold">{site.name}</h1>
        <Tooltip text="Add a credential to this site">
          <button
            onClick={() => setEditing("new")}
            className="rounded bg-blue-600 px-3 py-1"
          >
            Add credential
          </button>
        </Tooltip>
      </div>

      {error && <p className="text-sm text-red-400 mb-4">{error}</p>}

      <ul className="space-y-2">
        {credentials.map((c) => (
          <li key={c.id} className="rounded border px-2 py-1 dark:border-slate-700 flex justify-between">
            <span>{c.username ?? "(no username)"}</span>
            <div className="space-x-2">
              <Tooltip text="Edit this credential">
                <button onClick={() => setEditing(c)} className="underline text-sm">
                  Edit
                </button>
              </Tooltip>
              <Tooltip text="Move this credential to Trash">
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
                >
                  Delete
                </button>
              </Tooltip>
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
