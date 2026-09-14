import { Tooltip } from "../../components/Tooltip";
import { Icon } from "../../components/Icon";
import { ItemRow } from "./ItemRow";
import { itemCountLabel, listEmptyState } from "../lib/vaultSelectors";
import type { useVault } from "../hooks/useVault";

// List pane: header, count, add-site button, search box, rows and empty states. Reads
// visibleSites/query/error/selection from useVault() instead of owning its own fetch effects
// (D-09) - the hook is the single owner of vault fetch state.
export function ItemList({
  vault,
  onAddSite,
}: {
  vault: ReturnType<typeof useVault>;
  onAddSite: () => void;
}) {
  const { visibleSites, query, setQuery, results, error, selectedSiteId, selectSite, sitesLoading } = vault;

  const showLoading = sitesLoading && visibleSites.length === 0;
  const empty =
    !showLoading && visibleSites.length === 0
      ? listEmptyState(results === null ? "browse" : "search")
      : null;

  return (
    <div className="flex w-[280px] shrink-0 flex-col border-r border-[var(--border)] p-2">
      <div className="mb-2 flex items-center justify-between">
        <div className="flex items-baseline gap-2">
          <h2 className="text-base font-semibold">Vault</h2>
          <span className="text-xs opacity-70">{itemCountLabel(visibleSites.length)}</span>
        </div>
        <Tooltip text="Add a new site">
          <button onClick={onAddSite} className="rounded bg-[var(--accent)] px-2 py-1 text-sm">
            <Icon name="plus-lg" /> Add site
          </button>
        </Tooltip>
      </div>

      {error && <p className="mb-2 text-sm text-red-400">{error}</p>}

      <div className="relative mb-2">
        <Icon
          name="search"
          className="pointer-events-none absolute left-2 top-1/2 -translate-y-1/2 opacity-50"
        />
        <input
          className="w-full rounded border bg-[var(--surface)] border-[var(--border)] py-1 pl-7 pr-2"
          placeholder="Search by site, username, or remarks..."
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          title="Search your vault in real time"
        />
      </div>

      <ul className="flex-1 space-y-1 overflow-y-auto">
        {showLoading && <li className="px-1 py-2 text-sm opacity-70">Loading your sites…</li>}
        {empty && (
          <li className="px-1 py-2">
            <p className="text-sm">{empty.heading}</p>
            <p className="text-sm opacity-70">{empty.body}</p>
          </li>
        )}
        {!showLoading &&
          !empty &&
          visibleSites.map((s) => (
            <ItemRow
              key={s.id}
              site={s}
              selected={s.id === selectedSiteId}
              onSelect={() => selectSite(s.id)}
            />
          ))}
      </ul>
    </div>
  );
}
