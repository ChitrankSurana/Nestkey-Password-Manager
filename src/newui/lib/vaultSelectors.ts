import type { SiteDto, SearchResultDto } from "../../lib/api";

// Pure list-pane logic for the new UI's ItemList: no React, no `api` import (type-only imports
// of SiteDto/SearchResultDto are fine and keep this file loadable under vitest's node environment).

/**
 * Projects search results onto the sites they belong to. Today's `search` command returns one
 * row per matching credential (SearchResultDto), while the new list pane is a list of sites - so
 * two credentials on one site must collapse to a single list row (D-08). Emits one SiteDto per
 * distinct siteId, in first-appearance order of `results`, dropping any siteId absent from
 * `sites`.
 */
export function sitesForResults(sites: SiteDto[], results: SearchResultDto[]): SiteDto[] {
  const byId = new Map<number, SiteDto>();
  for (const s of sites) {
    byId.set(s.id, s);
  }

  const seen = new Set<number>();
  const out: SiteDto[] = [];
  for (const r of results) {
    if (seen.has(r.siteId)) continue;
    const site = byId.get(r.siteId);
    if (!site) continue;
    seen.add(r.siteId);
    out.push(site);
  }
  return out;
}

/** "0 items" / "1 item" / "7 items" - singular exactly at n === 1. */
export function itemCountLabel(n: number): string {
  return `${n} item${n === 1 ? "" : "s"}`;
}

/**
 * The two empty-state copy pairs, verbatim from the UI-SPEC Copywriting Contract. These strings
 * are the contract itself, not a shape to derive - callers and tests must compare them literally.
 */
export function listEmptyState(mode: "browse" | "search"): { heading: string; body: string } {
  if (mode === "search") {
    return { heading: "No matches", body: "Try a different search term." };
  }
  return { heading: "No sites yet", body: "Add one to start building your vault." };
}
