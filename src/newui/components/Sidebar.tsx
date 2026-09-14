import { Tooltip } from "../../components/Tooltip";
import { Icon } from "../../components/Icon";
import { SidebarNavItem } from "./SidebarNavItem";
import type { NavDestination } from "../../screens/NavRail";

// Vault-branded sidebar: header chip + Vault/{username} block (new chrome, no analog in
// NavRail.tsx - build fresh per UI-SPEC), the six real nav destinations (owned by NewShell,
// D-10: never extend beyond what's passed in), a collapse toggle, and a bottom-pinned lock
// button (both copied from NavRail.tsx's shape).
export function Sidebar({
  destinations,
  activeKey,
  collapsed,
  onToggleCollapsed,
  onLock,
  username,
}: {
  destinations: NavDestination[];
  activeKey: string | null;
  collapsed: boolean;
  onToggleCollapsed: () => void;
  onLock: () => void;
  username: string;
}) {
  const widthClass = collapsed ? "w-12" : "w-[220px]";

  return (
    <div
      className={`flex flex-col ${widthClass} shrink-0 gap-1 border-r border-[var(--border)] p-2`}
    >
      <div className="flex items-center gap-2 px-1 py-2">
        <span className="flex h-8 w-8 shrink-0 items-center justify-center rounded bg-[var(--accent-600)] text-white">
          <Icon name="shield-lock-fill" />
        </span>
        {!collapsed && (
          <div className="min-w-0">
            <p className="truncate text-base font-semibold leading-tight">Vault</p>
            <p className="truncate text-sm leading-tight opacity-70">{username}</p>
          </div>
        )}
      </div>

      <div className="h-1" />

      {destinations.map((d) => (
        <SidebarNavItem key={d.key} destination={d} active={d.key === activeKey} collapsed={collapsed} />
      ))}

      <div className="flex-1" />

      <Tooltip text={collapsed ? "Expand" : "Collapse"}>
        <button
          onClick={onToggleCollapsed}
          className="rounded px-2 py-1 text-left hover:bg-[var(--surface)]"
        >
          <Icon name={collapsed ? "chevron-double-right" : "chevron-double-left"} />
          {!collapsed && " Collapse"}
        </button>
      </Tooltip>
      <Tooltip text="Lock the vault now">
        <button onClick={onLock} className="rounded px-2 py-1 text-left hover:bg-[var(--surface)]">
          <Icon name="lock-fill" />
          {!collapsed && " Lock"}
        </button>
      </Tooltip>
    </div>
  );
}
