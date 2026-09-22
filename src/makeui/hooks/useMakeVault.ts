import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, isCommandError, type CredentialDto, type FolderDto, type SearchResultDto, type SiteDto } from "../../lib/api";
import { sitesForResults } from "../lib/vaultSelectors";

const FALLBACK_ERROR = "Something went wrong. Please try again.";

// Single owner of vault fetch state for the Make UI (mirrors src/newui/hooks/useVault.ts's D-09
// lazy-decryption invariant: sites/folders carry no secrets, credentials for the selected site
// load only on selection, and refreshCredentials below is the one call site). Adds folders, which
// newui has no analog for.
export function useMakeVault() {
  const [sites, setSites] = useState<SiteDto[]>([]);
  const [folders, setFolders] = useState<FolderDto[]>([]);
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

  const refreshFolders = useCallback(async () => {
    try {
      setFolders(await api.listFolders());
    } catch (err) {
      setError(isCommandError(err) ? err.message : FALLBACK_ERROR);
    }
  }, []);

  const latestQuery = useRef("");

  const runSearch = useCallback(async (q: string) => {
    latestQuery.current = q;
    if (q.trim() === "") {
      setResults(null);
      return;
    }
    try {
      setError(null);
      const searchResults = await api.search(q);
      if (latestQuery.current === q) setResults(searchResults);
    } catch (err) {
      if (latestQuery.current === q) setError(isCommandError(err) ? err.message : FALLBACK_ERROR);
    }
  }, []);

  const clearCredentials = useCallback(() => {
    setCredentials(null);
  }, []);

  const selectSite = useCallback((siteId: number | null) => {
    setSelectedSiteId(siteId);
  }, []);

  const latestRequestedSiteId = useRef<number | null>(null);

  const refreshCredentials = useCallback(async (siteId: number) => {
    latestRequestedSiteId.current = siteId;
    try {
      setError(null);
      setCredentialsLoading(true);
      const creds = await api.listCredentialsForSite(siteId);
      if (latestRequestedSiteId.current === siteId) setCredentials(creds);
    } catch (err) {
      if (latestRequestedSiteId.current === siteId) {
        setError(isCommandError(err) ? err.message : FALLBACK_ERROR);
      }
    } finally {
      if (latestRequestedSiteId.current === siteId) setCredentialsLoading(false);
    }
  }, []);

  useEffect(() => {
    refreshSites();
    refreshFolders();
  }, [refreshSites, refreshFolders]);

  useEffect(() => {
    runSearch(query);
  }, [query, runSearch]);

  useEffect(() => {
    if (selectedSiteId === null) {
      clearCredentials();
      return;
    }
    clearCredentials();
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
    folders,
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
    refreshFolders,
    refreshCredentials,
    clearCredentials,
  };
}
