import type { ComponentType } from "react";
import type { CredentialDto, SearchResultDto, SiteDto } from "../../lib/api";
import { IconCard, IconKey, IconNote, IconPerson, type IconProps } from "../icons";

// Pure list-pane logic for the Make variant. Mirrors src/newui/lib/vaultSelectors.ts (D-09-style
// separation: no React, no `api` import) but adds section/folder/type filtering, which newui has
// no analog for since it has no favorites/folders/item types.

/** Same projection as newui's sitesForResults - one SiteDto per distinct siteId, search-result order. */
export function sitesForResults(sites: SiteDto[], results: SearchResultDto[]): SiteDto[] {
  const byId = new Map<number, SiteDto>();
  for (const s of sites) byId.set(s.id, s);
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

/** Sidebar sections. "folder:<id>" is a dynamic section per real folder (no hardcoded FOLDERS list, D-14 gone). */
export type Section = "all" | "favorites" | "logins" | "notes" | "cards" | "identities" | `folder:${number}`;

const SECTION_LABEL: Record<string, string> = {
  all: "All Items",
  favorites: "Favorites",
  logins: "Logins",
  notes: "Secure Notes",
  cards: "Payment Cards",
  identities: "Identities",
};

export function sectionLabel(section: Section, folders: { id: number; name: string }[]): string {
  if (section.startsWith("folder:")) {
    const id = Number(section.slice("folder:".length));
    return folders.find((f) => f.id === id)?.name ?? "Folder";
  }
  return SECTION_LABEL[section] ?? section;
}

/** Filters a site list by sidebar section. Search filtering (sitesForResults) is applied separately, before this. */
export function sitesForSection(sites: SiteDto[], section: Section): SiteDto[] {
  if (section === "all") return sites;
  if (section === "favorites") return sites.filter((s) => s.favorite);
  if (section === "logins") return sites.filter((s) => s.itemType === "login");
  if (section === "notes") return sites.filter((s) => s.itemType === "note");
  if (section === "cards") return sites.filter((s) => s.itemType === "card");
  if (section === "identities") return sites.filter((s) => s.itemType === "identity");
  const id = Number(section.slice("folder:".length));
  return sites.filter((s) => s.folderId === id);
}

/** "0 items" / "1 item" / "7 items" - same rule as newui's itemCountLabel. */
export function itemCountLabel(n: number): string {
  return `${n} item${n === 1 ? "" : "s"}`;
}

export function listEmptyState(mode: "browse" | "search"): { heading: string; body: string } {
  if (mode === "search") return { heading: "No matches", body: "Try a different search term." };
  return { heading: "No items here yet", body: "Add one to start building your vault." };
}

export function itemTypeConfig(itemType: string): {
  Icon: ComponentType<IconProps>;
  bg: string;
  fg: string;
  label: string;
} {
  switch (itemType) {
    case "note":
      return { Icon: IconNote, bg: "bg-mk-note-bg", fg: "text-mk-note-fg", label: "Secure Note" };
    case "card":
      return { Icon: IconCard, bg: "bg-mk-card-bg", fg: "text-mk-card-fg", label: "Payment Card" };
    case "identity":
      return { Icon: IconPerson, bg: "bg-mk-identity-bg", fg: "text-mk-identity-fg", label: "Identity" };
    default:
      return { Icon: IconKey, bg: "bg-mk-login-bg", fg: "text-mk-login-fg", label: "Login" };
  }
}

/** Reserved CustomField labels sealed on a non-login item's sole credential row (D-H). */
export const RESERVED_LABEL = {
  cardNumber: "Card Number",
  cardExpiry: "Expiration Date",
  cardCvv: "Security Code (CVV)",
  note: "Note",
  fullName: "Full Name",
} as const;

/** Looks up one reserved-label field's value from a credential's sealed custom fields. */
export function reservedField(credentials: CredentialDto[] | null, label: string): string | null {
  const value = credentials?.[0]?.customFields.find((f) => f.label === label)?.value;
  return value ?? null;
}
