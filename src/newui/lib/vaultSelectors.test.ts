import { describe, it, expect } from "vitest";
import { sitesForResults, itemCountLabel, listEmptyState } from "./vaultSelectors";
import type { SiteDto, SearchResultDto } from "../../lib/api";

function site(id: number, name: string): SiteDto {
  return { id, name, url: null, notes: null };
}

function result(siteId: number, siteName: string, credentialId: number): SearchResultDto {
  return { siteId, siteName, credentialId, username: null };
}

describe("sitesForResults", () => {
  it("returns [] when sites is empty, regardless of results", () => {
    expect(sitesForResults([], [result(1, "a", 1)])).toEqual([]);
    expect(sitesForResults([], [])).toEqual([]);
  });

  it("returns one SiteDto per distinct siteId in results, in first-appearance order", () => {
    const sites = [site(1, "Alpha"), site(2, "Beta"), site(3, "Gamma")];
    // Two credentials on site 2, one on site 1 - site 2 appears first in results.
    const results = [result(2, "Beta", 10), result(1, "Alpha", 11), result(2, "Beta", 12)];
    expect(sitesForResults(sites, results)).toEqual([site(2, "Beta"), site(1, "Alpha")]);
  });

  it("drops any result whose siteId has no matching entry in sites", () => {
    const sites = [site(1, "Alpha")];
    const results = [result(1, "Alpha", 10), result(99, "Ghost", 11)];
    expect(sitesForResults(sites, results)).toEqual([site(1, "Alpha")]);
  });
});

describe("itemCountLabel", () => {
  it("pluralizes correctly for 0, 1, and n items", () => {
    expect(itemCountLabel(0)).toBe("0 items");
    expect(itemCountLabel(1)).toBe("1 item");
    expect(itemCountLabel(7)).toBe("7 items");
  });
});

describe("listEmptyState", () => {
  it("returns the exact browse-mode copy", () => {
    expect(listEmptyState("browse")).toEqual({
      heading: "No sites yet",
      body: "Add one to start building your vault.",
    });
  });

  it("returns the exact search-mode copy", () => {
    expect(listEmptyState("search")).toEqual({
      heading: "No matches",
      body: "Try a different search term.",
    });
  });
});
