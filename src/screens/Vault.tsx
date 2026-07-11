import { useEffect, useState } from "react";
import { api, isCommandError, SiteDto, SearchResultDto } from "../lib/api";
import { SiteEditor } from "./SiteEditor";
import { SiteDetail } from "./SiteDetail";
import { Trash } from "./Trash";
import { Settings } from "./Settings";
import { Tooltip } from "../components/Tooltip";

export function Vault({ username }: { username: string }) {
  const [sites, setSites] = useState<SiteDto[]>([]);
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchResultDto[] | null>(null);
  const [selectedSiteId, setSelectedSiteId] = useState<number | null>(null);
  const [addingSite, setAddingSite] = useState(false);
  const [showTrash, setShowTrash] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function refreshSites() {
    try {
      setError(null);
      setSites(await api.listSites());
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    }
  }

  async function runSearch(q: string) {
    if (q.trim() === "") {
      setResults(null);
      return;
    }
    try {
      setError(null);
      // Real-time search (TRD NFR-3: results update within one UI frame of keystroke) - no debounce,
      // this is a local IPC call against a SQLite index, not a network request.
      setResults(await api.search(q));
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    }
  }

  useEffect(() => {
    refreshSites();
  }, []);

  useEffect(() => {
    runSearch(query);
  }, [query]);

  if (selectedSiteId !== null) {
    return (
      <SiteDetail
        site={sites.find((s) => s.id === selectedSiteId)!}
        onBack={() => {
          setSelectedSiteId(null);
          refreshSites();
          runSearch(query);
        }}
      />
    );
  }

  if (showTrash) {
    return (
      <Trash
        onBack={() => {
          setShowTrash(false);
          runSearch(query);
        }}
      />
    );
  }

  if (showSettings) {
    return <Settings onBack={() => setShowSettings(false)} />;
  }

  return (
    <div className="min-h-screen dark:bg-slate-900 dark:text-slate-100 p-4">
      <div className="flex justify-between items-center mb-4">
        <h1 className="text-xl font-semibold">Vault — {username}</h1>
        <div className="space-x-2">
          <Tooltip text="Add a new site">
            <button
              onClick={() => setAddingSite(true)}
              className="rounded bg-blue-600 px-3 py-1"
            >
              Add site
            </button>
          </Tooltip>
          <Tooltip text="View trash">
            <button onClick={() => setShowTrash(true)} className="rounded bg-slate-700 px-3 py-1">
              Trash
            </button>
          </Tooltip>
          <Tooltip text="Open settings">
            <button
              onClick={() => setShowSettings(true)}
              className="rounded bg-slate-700 px-3 py-1"
            >
              Settings
            </button>
          </Tooltip>
          <Tooltip text="Lock the vault now">
            <button
              onClick={async () => {
                try {
                  setError(null);
                  await api.lock();
                } catch (err) {
                  setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
                }
              }}
              className="rounded bg-slate-700 px-3 py-1"
            >
              Lock
            </button>
          </Tooltip>
        </div>
      </div>

      {error && <p className="text-sm text-red-400 mb-4">{error}</p>}

      <input
        className="w-full rounded border px-2 py-1 mb-4 dark:bg-slate-800 dark:border-slate-600"
        placeholder="Search by site, username, or remarks..."
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        title="Search your vault in real time"
      />

      {results !== null ? (
        <ul className="space-y-1">
          {results.map((r) => (
            <li
              key={r.credentialId}
              className="cursor-pointer rounded border px-2 py-1 dark:border-slate-700"
              onClick={() => setSelectedSiteId(r.siteId)}
            >
              {r.siteName} — {r.username ?? "(no username)"}
            </li>
          ))}
          {results.length === 0 && <li className="text-sm opacity-70">No matches.</li>}
        </ul>
      ) : (
        <ul className="space-y-1">
          {sites.map((s) => (
            <li
              key={s.id}
              className="cursor-pointer rounded border px-2 py-1 dark:border-slate-700"
              onClick={() => setSelectedSiteId(s.id)}
            >
              {s.name}
            </li>
          ))}
          {sites.length === 0 && <li className="text-sm opacity-70">No sites yet — add one.</li>}
        </ul>
      )}

      {addingSite && (
        <SiteEditor
          onClose={() => setAddingSite(false)}
          onSaved={() => {
            setAddingSite(false);
            refreshSites();
          }}
        />
      )}
    </div>
  );
}
