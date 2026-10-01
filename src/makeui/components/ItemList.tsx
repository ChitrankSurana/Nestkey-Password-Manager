import { IconPlus, IconSearch } from "../icons";
import { ItemRow } from "./ItemRow";
import { listEmptyState, sectionLabel, sitesForSection, type Section } from "../lib/vaultSelectors";
import type { useMakeVault } from "../hooks/useMakeVault";

// List pane, ported from figma-make/src/App.tsx:598-657 (ItemList) onto real vault data. Reads
// visibleSites/query/error/selection from useMakeVault() rather than owning its own fetch effects
// (D-09, same rule as src/newui/components/ItemList.tsx). Section filtering (type/folder/favorite)
// runs after search filtering, mirroring the prototype's own `items` prop being pre-filtered by
// the caller.
export function ItemList({
  vault,
  section,
  onAdd,
  onSelect,
  className = "flex w-[280px]",
}: {
  vault: ReturnType<typeof useMakeVault>;
  section: Section;
  onAdd: () => void;
  onSelect: (siteId: number) => void;
  /** Display + width classes only (B3-8) - MakeApp is the sole breakpoint owner, mirroring
   * src/newui/components/ItemList.tsx's own className prop, so this component never branches on
   * viewport width itself. Defaults to today's always-visible fixed width. */
  className?: string;
}) {
  const { visibleSites, folders, query, setQuery, results, error, selectedSiteId, sitesLoading } = vault;
  const filtered = sitesForSection(visibleSites, section);

  const showLoading = sitesLoading && filtered.length === 0;
  const empty = !showLoading && filtered.length === 0 ? listEmptyState(results === null ? "browse" : "search") : null;

  return (
    <div className={`${className} min-h-0 flex-shrink-0 flex-col h-full bg-mk-panel border-r border-mk-border`}>
      <div className="h-14 px-3 flex items-center justify-between border-b border-mk-border">
        <h2 className="text-sm font-semibold text-mk-fg">{sectionLabel(section, folders)}</h2>
        <div className="flex items-center gap-1">
          <span className="text-xs mr-1 text-mk-faint">{filtered.length}</span>
          <button
            onClick={onAdd}
            title="Add item"
            className="w-7 h-7 flex items-center justify-center rounded-lg bg-mk-accent hover:bg-mk-accent-hover transition-colors"
          >
            <IconPlus size={13} className="text-white" />
          </button>
        </div>
      </div>
      <div className="px-3 py-2.5 border-b border-mk-border">
        <div className="relative">
          <IconSearch size={13} className="absolute left-2.5 top-1/2 -translate-y-1/2 text-mk-faint" />
          <input
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search…"
            title="Search your vault in real time"
            className="w-full pl-8 pr-3 py-1.5 text-xs border rounded-lg bg-mk-input border-mk-border text-mk-fg2 focus:outline-none focus:ring-1 focus:ring-mk-accent-muted transition-all"
          />
        </div>
      </div>

      {error && <p className="px-3 pt-2 text-xs text-mk-red">{error}</p>}

      <div className="flex-1 overflow-y-auto">
        {showLoading && <p className="p-4 text-xs text-mk-faint">Loading your vault…</p>}
        {empty && (
          <div className="flex flex-col items-center justify-center h-full gap-2 p-6">
            <IconSearch size={24} className="text-mk-faint" />
            <p className="text-xs text-center text-mk-faint">{empty.heading}</p>
            <p className="text-xs text-center text-mk-faint opacity-70">{empty.body}</p>
          </div>
        )}
        {!showLoading &&
          !empty &&
          filtered.map((site) => (
            <ItemRow key={site.id} site={site} selected={selectedSiteId === site.id} onClick={() => onSelect(site.id)} />
          ))}
      </div>
    </div>
  );
}
