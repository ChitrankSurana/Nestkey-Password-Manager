import { useEffect, useState } from "react";
import { api, isCommandError, CredentialDto, SearchResultDto, SiteDto } from "../lib/api";
import { SecretField } from "../components/SecretField";
import { CopyButton } from "../makeui/components/CopyButton";
import { ItemRow } from "../makeui/components/ItemRow";
import { IconKey, IconLock, IconSearch, IconShield } from "../makeui/icons";

// N16: restyled onto the Make UI visual language (mk-* tokens, ItemRow/CopyButton reuse) instead
// of the plain unstyled list this screen shipped with. Still read-only and still its own component
// (not a passthrough of ItemList/DetailPanel, which are wired for add/edit/delete) - only the two
// pieces that are already read-agnostic (ItemRow, CopyButton) are reused directly.
export function ViewerVault({ username }: { username: string }) {
  const [sites, setSites] = useState<SiteDto[]>([]);
  const [query, setQuery] = useState("");
  const [searchResults, setSearchResults] = useState<SearchResultDto[] | null>(null);
  const [selectedSiteId, setSelectedSiteId] = useState<number | null>(null);
  const [credentials, setCredentials] = useState<CredentialDto[]>([]);
  const [credentialsLoading, setCredentialsLoading] = useState(false);
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
    // A stale selection must not resurface just because the search query changed (e.g. clearing
    // back to browse, or re-matching a previously-selected site).
    setSelectedSiteId(null);
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

  async function selectSite(siteId: number) {
    if (selectedSiteId === siteId) return;
    setCredentialsLoading(true);
    try {
      setError(null);
      // Decrypt only what's needed for the current screen: credentials for a site are fetched
      // only when that site is clicked, never prefetched for the whole list.
      setCredentials(await api.listCredentialsForSite(siteId));
      setSelectedSiteId(siteId);
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    } finally {
      setCredentialsLoading(false);
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
  // distinct sites, same as the previous version of this screen.
  const rows: SiteDto[] = searchResults
    ? Array.from(new Map(searchResults.map((r) => [r.siteId, r.siteName])).entries()).map(
        ([siteId, siteName]) =>
          sites.find((s) => s.id === siteId) ?? {
            id: siteId,
            name: siteName,
            itemType: "login",
            favorite: false,
            url: null,
            notes: null,
            folderId: null,
            cardBrand: null,
            cardLast4: null,
          },
      )
    : sites;
  const selectedSite = rows.find((s) => s.id === selectedSiteId) ?? null;

  return (
    <div className="h-screen overflow-hidden bg-mk-base text-mk-fg flex font-mk-sans">
      <div className="w-[280px] flex-shrink-0 flex flex-col h-full bg-mk-sidebar border-r border-mk-border">
        <div className="px-4 h-14 flex items-center gap-2.5 border-b border-mk-border">
          <div className="w-7 h-7 rounded-lg bg-mk-accent flex items-center justify-center flex-shrink-0">
            <IconShield size={15} className="text-white" />
          </div>
          <div className="flex-1 min-w-0">
            <div className="text-sm font-semibold leading-tight text-mk-fg truncate">{username}</div>
            <div className="text-[10px] leading-tight text-mk-faint">Read-only</div>
          </div>
          <button
            onClick={handleLock}
            title="Lock Vault"
            className="flex items-center justify-center w-8 h-8 rounded-lg text-mk-faint hover:opacity-80 transition-opacity flex-shrink-0"
          >
            <IconLock size={13} />
          </button>
        </div>

        <div className="px-3 py-2.5 border-b border-mk-border">
          <div className="relative">
            <IconSearch size={13} className="absolute left-2.5 top-1/2 -translate-y-1/2 text-mk-faint" />
            <input
              type="text"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="Search by site, username, or remarks…"
              title="Search your vault in real time"
              autoFocus
              className="w-full pl-8 pr-3 py-1.5 text-xs border rounded-lg bg-mk-input border-mk-border text-mk-fg2 focus:outline-none focus:ring-1 focus:ring-mk-accent-muted transition-all"
            />
          </div>
        </div>

        {error && <p className="px-3 pt-2 text-xs text-mk-red">{error}</p>}

        <div className="flex-1 overflow-y-auto">
          {rows.length === 0 && (
            <div className="flex flex-col items-center justify-center h-full gap-2 p-6">
              <IconKey size={24} className="text-mk-faint" />
              <p className="text-xs text-center text-mk-faint">
                {searchResults !== null ? "No matches." : "No sites yet."}
              </p>
            </div>
          )}
          {rows.map((site) => (
            <ItemRow key={site.id} site={site} selected={selectedSiteId === site.id} onClick={() => selectSite(site.id)} />
          ))}
        </div>
      </div>

      <div className="flex-1 min-h-0 flex flex-col overflow-hidden">
        {!selectedSite && (
          <div className="flex-1 flex flex-col items-center justify-center gap-2 text-mk-faint">
            <IconKey size={32} />
            <p className="text-sm">Select an item to view it</p>
          </div>
        )}
        {selectedSite && (
          <>
            <div className="h-14 px-5 flex items-center border-b border-mk-border flex-shrink-0">
              <h1 className="text-sm font-semibold text-mk-fg truncate">{selectedSite.name}</h1>
            </div>
            <div className="flex-1 overflow-y-auto p-5 space-y-3">
              {credentialsLoading && <p className="text-xs text-mk-faint">Loading…</p>}
              {!credentialsLoading && credentials.length === 0 && (
                <p className="text-xs text-mk-faint">No credentials for this item.</p>
              )}
              {!credentialsLoading &&
                credentials.map((c) => (
                  <div key={c.id} className="rounded-lg border border-mk-border bg-mk-card p-3 space-y-2">
                    <div className="flex items-center justify-between gap-2">
                      <span className="text-xs text-mk-muted">Username</span>
                      <div className="flex items-center gap-1.5">
                        <span className="text-sm text-mk-fg truncate">{c.username ?? "(no username)"}</span>
                        {c.username && <CopyButton text={c.username} title="Copy username" />}
                      </div>
                    </div>
                    <div className="flex items-center justify-between gap-2">
                      <span className="text-xs text-mk-muted">Password</span>
                      <SecretField secret={c.secret} />
                    </div>
                  </div>
                ))}
            </div>
          </>
        )}
      </div>
    </div>
  );
}
