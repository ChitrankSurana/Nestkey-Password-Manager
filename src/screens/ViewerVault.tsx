import { useEffect, useState } from "react";
import { api, isCommandError, CredentialDto, SearchResultDto, SiteDto } from "../lib/api";
import { SecretField } from "../components/SecretField";
import { Icon } from "../components/Icon";

export function ViewerVault({ username }: { username: string }) {
  const [sites, setSites] = useState<SiteDto[]>([]);
  const [query, setQuery] = useState("");
  const [searchResults, setSearchResults] = useState<SearchResultDto[] | null>(null);
  const [expandedSiteId, setExpandedSiteId] = useState<number | null>(null);
  const [credentials, setCredentials] = useState<CredentialDto[]>([]);
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
    // A stale expanded site must not resurface just because the search query changed (e.g.
    // clearing back to browse, or re-matching a previously-expanded site).
    setExpandedSiteId(null);
    setCredentials([]);
    if (q.trim() === "") {
      setSearchResults(null);
      return;
    }
    try {
      setError(null);
      // Real-time search (same backend call Vault.tsx uses) - no debounce, this is a local IPC
      // call against a SQLite index, not a network request.
      setSearchResults(await api.search(q));
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

  async function toggleExpand(siteId: number) {
    if (expandedSiteId === siteId) {
      setExpandedSiteId(null);
      setCredentials([]);
      return;
    }
    try {
      setError(null);
      // Decrypt only what's needed for the current screen: credentials for a site are fetched
      // only when that site is clicked, never prefetched for the whole list.
      setCredentials(await api.listCredentialsForSite(siteId));
      setExpandedSiteId(siteId);
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    }
  }

  async function handleLock() {
    try {
      setError(null);
      await api.lock();
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    }
  }

  // Browse mode shows every site; search mode collapses the matching credentials down to their
  // distinct sites. Either way, clicking a row expands the same way via listCredentialsForSite -
  // no credential is decrypted until its site is actually clicked.
  const rows: { siteId: number; siteName: string }[] = searchResults
    ? Array.from(new Map(searchResults.map((r) => [r.siteId, r.siteName])).entries()).map(
        ([siteId, siteName]) => ({ siteId, siteName }),
      )
    : sites.map((s) => ({ siteId: s.id, siteName: s.name }));

  return (
    <div className="min-h-screen bg-[var(--bg)] text-[var(--fg)] p-6">
      <div className="flex items-center justify-between mb-4">
        <h1 className="text-xl font-semibold">Vault — {username} (read-only)</h1>
        <button onClick={handleLock} className="rounded bg-[var(--accent)] px-3 py-1.5 text-sm">
          <Icon name="lock-fill" /> Lock
        </button>
      </div>

      {error && <p className="text-sm text-red-400 mb-4">{error}</p>}

      <input
        className="w-full rounded border px-2 py-1 mb-4 bg-[var(--surface)] border-[var(--border)]"
        placeholder="Search by site, username, or remarks..."
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        title="Search your vault in real time"
        autoFocus
      />

      <ul className="space-y-1">
        {rows.map((row) => (
          <li key={row.siteId} className="rounded border border-[var(--border)]">
            <button
              type="button"
              className="w-full text-left px-2 py-1 hover:bg-[var(--surface)]"
              onClick={() => toggleExpand(row.siteId)}
            >
              {row.siteName}
            </button>
            {expandedSiteId === row.siteId && (
              <ul className="px-3 pb-2 space-y-2">
                {credentials.map((c) => (
                  <li key={c.id} className="flex items-center gap-2 text-sm">
                    <span className="w-32 truncate">{c.username ?? "(no username)"}</span>
                    <SecretField secret={c.secret} />
                  </li>
                ))}
              </ul>
            )}
          </li>
        ))}
        {rows.length === 0 && (
          <li className="text-sm opacity-70">
            {searchResults !== null ? "No matches." : "No sites yet."}
          </li>
        )}
      </ul>
    </div>
  );
}
