// src/screens/NavRail.tsx
import { Tooltip } from "../components/Tooltip";
import { Icon } from "../components/Icon";

export interface NavDestination {
  key: string;
  label: string;
  icon: string; // a bootstrap-icons glyph name (no "bi-" prefix), rendered via <Icon/>
  tooltip: string;
  onClick: () => void;
}

export function NavRail({
  destinations,
  activeKey,
  collapsed,
  onToggleCollapsed,
  onLock,
  showToggle = true,
}: {
  destinations: NavDestination[];
  activeKey: string | null;
  collapsed: boolean;
  onToggleCollapsed: () => void;
  onLock: () => void;
  showToggle?: boolean;
}) {
  const widthClass = collapsed ? "w-12" : "w-40";

  return (
    <div className={`flex flex-col ${widthClass} shrink-0 gap-1 border-r border-[var(--border)] p-2`}>
      {showToggle && (
        <>
          <Tooltip text={collapsed ? "Expand" : "Collapse"}>
            <button
              onClick={onToggleCollapsed}
              className="rounded px-2 py-1 text-left hover:bg-[var(--surface)]"
            >
              <Icon name={collapsed ? "chevron-double-right" : "chevron-double-left"} />
              {!collapsed && " Collapse"}
            </button>
          </Tooltip>
          <div className="h-2" />
        </>
      )}
      {destinations.map((d) => {
        const active = d.key === activeKey;
        return (
          <Tooltip key={d.key} text={d.tooltip}>
            <button
              onClick={d.onClick}
              className={`rounded px-2 py-1 text-left border-l-2 ${
                active
                  ? "bg-[var(--accent-subtle-bg)] text-[var(--accent-subtle-fg)] border-[var(--accent)]"
                  : "border-transparent hover:bg-[var(--surface)]"
              }`}
            >
              <Icon name={d.icon} />
              {!collapsed && ` ${d.label}`}
            </button>
          </Tooltip>
        );
      })}
      <div className="flex-1" />
      <Tooltip text="Lock the vault now">
        <button onClick={onLock} className="rounded px-2 py-1 text-left hover:bg-[var(--surface)]">
          <Icon name="lock-fill" />
          {!collapsed && " Lock"}
        </button>
      </Tooltip>
    </div>
  );
}
