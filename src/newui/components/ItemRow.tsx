import { Icon } from "../../components/Icon";
import type { SiteDto } from "../../lib/api";

// One site row: neutral icon chip (globe2 - there is exactly one item type in this product,
// D-10, so never a per-type colored icon and never a favorite star), truncated name, truncated
// url subtitle. Selection uses the accent-subtle tint, never a solid accent fill.
export function ItemRow({
  site,
  selected,
  onSelect,
}: {
  site: SiteDto;
  selected: boolean;
  onSelect: () => void;
}) {
  return (
    <li
      onClick={onSelect}
      className={`flex cursor-pointer items-center gap-2.5 rounded px-2 py-1.5 ${
        selected
          ? "bg-[var(--accent-subtle-bg)] text-[var(--accent-subtle-fg)]"
          : "hover:bg-[var(--surface)]"
      }`}
    >
      <span className="flex h-8 w-8 shrink-0 items-center justify-center rounded border bg-[var(--surface)] border-[var(--border)]">
        <Icon name="globe2" />
      </span>
      <div className="min-w-0 flex-1">
        <p className="truncate text-sm">{site.name}</p>
        <p className="truncate text-xs opacity-70">
          {site.url ?? "(no URL)"}
        </p>
      </div>
    </li>
  );
}
