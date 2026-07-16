// src/screens/NavRail.tsx
import { Tooltip } from "../components/Tooltip";

export interface NavDestination {
  key: string;
  label: string;
  icon: string; // a single emoji/character, kept simple - no icon library dependency
  tooltip: string;
  onClick: () => void;
}

export function NavRail({
  destinations,
  activeKey,
  collapsed,
  onToggleCollapsed,
  onLock,
}: {
  destinations: NavDestination[];
  activeKey: string | null;
  collapsed: boolean;
  onToggleCollapsed: () => void;
  onLock: () => void;
}) {
  const widthClass = collapsed ? "w-12" : "w-40";

  return (
    <div className={`flex flex-col ${widthClass} shrink-0 gap-1 border-r dark:border-slate-700 p-2`}>
      <Tooltip text={collapsed ? "Expand" : "Collapse"}>
        <button
          onClick={onToggleCollapsed}
          className="rounded px-2 py-1 text-left hover:bg-slate-800"
        >
          {collapsed ? "»" : "« Collapse"}
        </button>
      </Tooltip>
      <div className="h-2" />
      {destinations.map((d) => (
        <Tooltip key={d.key} text={d.tooltip}>
          <button
            onClick={d.onClick}
            className={`rounded px-2 py-1 text-left ${
              d.key === activeKey ? "bg-[var(--accent)]" : "hover:bg-slate-800"
            }`}
          >
            {collapsed ? d.icon : `${d.icon} ${d.label}`}
          </button>
        </Tooltip>
      ))}
      <div className="flex-1" />
      <Tooltip text="Lock the vault now">
        <button onClick={onLock} className="rounded px-2 py-1 text-left hover:bg-slate-800">
          {collapsed ? "🔒" : "🔒 Lock"}
        </button>
      </Tooltip>
    </div>
  );
}
