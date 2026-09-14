import { Tooltip } from "../../components/Tooltip";
import { Icon } from "../../components/Icon";
import type { NavDestination } from "../../screens/NavRail";

// One tooltip-wrapped, accent-subtle-tinted nav button - mirrors NavRail's per-destination
// button shape (src/screens/NavRail.tsx lines 49-61), but takes `active` as a prop instead of
// deriving it from an `activeKey` comparison, since Sidebar owns that comparison per destination.
export function SidebarNavItem({
  destination,
  active,
  collapsed,
}: {
  destination: NavDestination;
  active: boolean;
  collapsed: boolean;
}) {
  return (
    <Tooltip text={destination.tooltip}>
      <button
        onClick={destination.onClick}
        className={`rounded px-2 py-1 text-left border-l-2 ${
          active
            ? "bg-[var(--accent-subtle-bg)] text-[var(--accent-subtle-fg)] border-[var(--accent)]"
            : "border-transparent hover:bg-[var(--surface)]"
        }`}
      >
        <Icon name={destination.icon} />
        {!collapsed && ` ${destination.label}`}
      </button>
    </Tooltip>
  );
}
