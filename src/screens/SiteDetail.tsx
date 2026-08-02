import { useEffect, useState } from "react";
import { api, isCommandError, CredentialDto, SiteDto } from "../lib/api";
import { CredentialEditor } from "./CredentialEditor";
import { SiteEditor } from "./SiteEditor";
import { SecretField } from "../components/SecretField";
import { Tooltip } from "../components/Tooltip";
import { Icon } from "../components/Icon";

// Renders as an overlay over the vault list (Phase 2 of the UI plan) rather than replacing the
// whole screen, so the list's search/scroll state survives opening/closing a site. Takes siteId
// rather than a SiteDto - AppShell owns selection but not the sites list, and there's no
// get_site command, so this fetches its own site record the same way it already fetches
// credentials (no new backend command for a personal-vault-scale list).
export function SiteDetail({ siteId, onBack }: { siteId: number; onBack: () => void }) {
  const [site, setSite] = useState<SiteDto | null>(null);
  const [credentials, setCredentials] = useState<CredentialDto[]>([]);
  const [editing, setEditing] = useState<CredentialDto | null | "new">(null);
  const [editingSite, setEditingSite] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function refresh() {
    try {
      setError(null);
      const [sites, creds] = await Promise.all([api.listSites(), api.listCredentialsForSite(siteId)]);
      setSite(sites.find((s) => s.id === siteId) ?? null);
      setCredentials(creds);
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    }
  }

  useEffect(() => {
    refresh();
  }, [siteId]);

  return (
    <div className="fixed inset-0 z-50 bg-black/50 flex items-center justify-center p-4">
      <div className="w-full max-w-lg max-h-[85vh] overflow-y-auto rounded bg-[var(--surface)] p-4 text-[var(--fg)]">
        <Tooltip text="Close">
          <button onClick={onBack} className="mb-4 underline text-sm">
            <Icon name="x-lg" /> Close
          </button>
        </Tooltip>
        <div className="flex justify-between items-center mb-4">
          <h1 className="text-xl font-semibold">{site?.name ?? "…"}</h1>
          <div className="space-x-2">
            <Tooltip text="Edit this site">
              <button
                onClick={() => setEditingSite(true)}
                disabled={!site}
                className="rounded border px-3 py-1 border-[var(--border)] disabled:opacity-50"
              >
                <Icon name="pencil-fill" /> Edit site
              </button>
            </Tooltip>
            <Tooltip text="Add a credential to this site">
              <button
                onClick={() => setEditing("new")}
                className="rounded bg-[var(--accent)] px-3 py-1"
              >
                <Icon name="plus-lg" /> Add credential
              </button>
            </Tooltip>
          </div>
        </div>

        {error && <p className="text-sm text-red-400 mb-4">{error}</p>}

        <ul className="space-y-2">
          {credentials.map((c) => (
            <li key={c.id} className="rounded border px-2 py-1 border-[var(--border)] flex items-center gap-2">
              <span className="w-32 truncate">{c.username ?? "(no username)"}</span>
              <SecretField secret={c.secret} />
              <Tooltip text="Edit this credential">
                <button onClick={() => setEditing(c)} className="underline text-sm">
                  <Icon name="pencil-fill" /> Edit
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
                  className="underline text-sm text-[var(--danger)]"
                >
                  <Icon name="trash" /> Delete
                </button>
              </Tooltip>
            </li>
          ))}
          {credentials.length === 0 && <li className="text-sm opacity-70">No credentials yet.</li>}
        </ul>

        {editing !== null && (
          <CredentialEditor
            siteId={siteId}
            existing={editing === "new" ? null : editing}
            onClose={() => setEditing(null)}
            onSaved={() => {
              setEditing(null);
              refresh();
            }}
          />
        )}

        {editingSite && site && (
          <SiteEditor
            existing={site}
            onClose={() => setEditingSite(false)}
            onSaved={() => {
              setEditingSite(false);
              refresh();
            }}
            onDeleted={() => {
              setEditingSite(false);
              onBack();
            }}
          />
        )}
      </div>
    </div>
  );
}
