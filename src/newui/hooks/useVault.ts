import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, isCommandError, type CredentialDto, type SearchResultDto, type SiteDto } from "../../lib/api";
import { sitesForResults } from "../lib/vaultSelectors";

const FALLBACK_ERROR = "Something went wrong. Please try again.";

// Single owner of vault fetch state for the new UI (D-09). Sites load eagerly with no secrets
// (api.listSites - SiteDto carries no secret field); credentials for a site load ONLY on
// selection, via the one credential-fetch command, and ONLY from this hook - `refreshCredentials`
// below is the one and only call site anywhere under src/newui/. This is the lazy-decryption
// invariant: no credential is ever prefetched, batched across sites, or fetched to render the
// list, because there is no cross-site credential listing command and therefore no
// credential-derived badge is possible under this rule.
export function useVault() {
  const [sites, setSites] = useState<SiteDto[]>([]);
  const [results, setResults] = useState<SearchResultDto[] | null>(null);
  const [query, setQuery] = useState("");
  const [selectedSiteId, setSelectedSiteId] = useState<number | null>(null);
  const [credentials, setCredentials] = useState<CredentialDto[] | null>(null);
  const [sitesLoading, setSitesLoading] = useState(true);
  const [credentialsLoading, setCredentialsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refreshSites = useCallback(async () => {
    try {
      setError(null);
      setSites(await api.listSites());
    } catch (err) {
      setError(isCommandError(err) ? err.message : FALLBACK_ERROR);
    } finally {
      setSitesLoading(false);
    }
  }, []);

  // Guards against out-of-order responses (WR-01): if a later keystroke's search resolves
  // before an earlier one, only the response matching the currently-typed query is committed.
  const latestQuery = useRef("");

  const runSearch = useCallback(async (q: string) => {
    latestQuery.current = q;
    if (q.trim() === "") {
      setResults(null);
      return;
    }
    try {
      setError(null);
      // Real-time search (TRD NFR-3: results update within one UI frame of keystroke) - no debounce,
      // this is a local IPC call against a SQLite index, not a network request.
      const searchResults = await api.search(q);
      if (latestQuery.current === q) {
        setResults(searchResults);
      }
    } catch (err) {
      if (latestQuery.current === q) {
        setError(isCommandError(err) ? err.message : FALLBACK_ERROR);
      }
    }
  }, []);

  const clearCredentials = useCallback(() => {
    setCredentials(null);
  }, []);

  const selectSite = useCallback((siteId: number | null) => {
    setSelectedSiteId(siteId);
  }, []);

  // Guards against out-of-order responses (CR-01): if the user switches sites while a
  // credential fetch for a previous site is still in flight, only the response matching the
  // currently selected site is committed to state.
  const latestRequestedSiteId = useRef<number | null>(null);

  // The lazy-decryption invariant's one call site (D-09).
  const refreshCredentials = useCallback(async (siteId: number) => {
    latestRequestedSiteId.current = siteId;
    try {
      setError(null);
      setCredentialsLoading(true);
      const creds = await api.listCredentialsForSite(siteId);
      if (latestRequestedSiteId.current === siteId) {
        setCredentials(creds);
      }
    } catch (err) {
      if (latestRequestedSiteId.current === siteId) {
        setError(isCommandError(err) ? err.message : FALLBACK_ERROR);
      }
    } finally {
      if (latestRequestedSiteId.current === siteId) {
        setCredentialsLoading(false);
      }
    }
  }, []);

  useEffect(() => {
    refreshSites();
  }, [refreshSites]);

  useEffect(() => {
    runSearch(query);
  }, [query, runSearch]);

  // Credentials must be null whenever there is no selection - including the very first render
  // (selectedSiteId starts null), a deselect via selectSite(null), and the vault-locked listener
  // below. DetailPanel (plan 08-04) stays mounted inside the three-pane shell, unlike the classic
  // SiteDetail overlay which unmounted on close, so this hook must clear explicitly rather than
  // relying on unmount.
  useEffect(() => {
    if (selectedSiteId === null) {
      clearCredentials();
      return;
    }
    clearCredentials(); // never show a stale site's credentials under the new site's header
    refreshCredentials(selectedSiteId);
  }, [selectedSiteId, refreshCredentials, clearCredentials]);

  useEffect(() => {
    const unlistenPromise = listen("vault-locked", () => {
      clearCredentials();
      selectSite(null);
    });
    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, [clearCredentials, selectSite]);

  const visibleSites = results === null ? sites : sitesForResults(sites, results);
  const selectedSite = sites.find((s) => s.id === selectedSiteId) ?? null;

  return {
    sites,
    results,
    visibleSites,
    query,
    setQuery,
    selectedSiteId,
    selectSite,
    selectedSite,
    credentials,
    sitesLoading,
    credentialsLoading,
    error,
    refreshSites,
    refreshCredentials,
    clearCredentials,
  };
}
