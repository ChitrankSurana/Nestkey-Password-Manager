import { useEffect, useState } from "react";
import { api, isCommandError, SiteDto, SearchResultDto } from "../lib/api";
import { SiteEditor } from "./SiteEditor";
import { Tooltip } from "../components/Tooltip";
import { Icon } from "../components/Icon";

export function Vault({
  username,
  onSelectSite,
  refreshSignal,
}: {
  username: string;
  onSelectSite: (siteId: number) => void;
  // Bumped by AppShell whenever SiteDetail closes, so a site rename/delete made while the
  // overlay was open is reflected here without re-running the search on every keystroke.
  refreshSignal: number;
}) {
  const [sites, setSites] = useState<SiteDto[]>([]);
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchResultDto[] | null>(null);
  const [addingSite, setAddingSite] = useState(false);
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

  useEffect(() => {
    if (refreshSignal === 0) return; // 0 is the initial value; the mount effect above covers it
    refreshSites();
    runSearch(query);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- only refreshSignal should retrigger this
  }, [refreshSignal]);

  return (
    <div className="flex-1 p-4">
      {/* Sticky: title, add button, error and search stay put while the site list scrolls. */}
      <div className="sticky top-0 z-10 -mx-4 -mt-4 mb-2 bg-[var(--bg)] px-4 pb-2 pt-4">
      <div className="flex justify-between items-center mb-4">
        <h1 className="text-xl font-semibold">Vault — {username}</h1>
        <Tooltip text="Add a new site">
          <button onClick={() => setAddingSite(true)} className="rounded bg-[var(--accent)] px-3 py-1">
            <Icon name="plus-lg" /> Add site
          </button>
        </Tooltip>
      </div>

      {error && <p className="text-sm text-red-400 mb-4">{error}</p>}

      <input
        className="w-full rounded border px-2 py-1 bg-[var(--surface)] border-[var(--border)]"
        placeholder="Search by site, username, or remarks..."
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        title="Search your vault in real time"
      />
      </div>

      {results !== null ? (
        <ul className="space-y-1">
          {results.map((r) => (
            <li
              key={r.credentialId}
              className="cursor-pointer rounded border px-2 py-1 border-[var(--border)]"
              onClick={() => onSelectSite(r.siteId)}
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
              className="cursor-pointer rounded border px-2 py-1 border-[var(--border)]"
              onClick={() => onSelectSite(s.id)}
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
