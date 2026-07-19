import { NavDestination } from "./NavRail";
import { Tooltip } from "../components/Tooltip";

export function BottomTabBar({
  destinations,
  activeKey,
  onLock,
}: {
  destinations: NavDestination[];
  activeKey: string | null;
  onLock: () => void;
}) {
  return (
    <div className="sm:hidden fixed bottom-0 inset-x-0 z-40 flex border-t dark:border-slate-700 dark:bg-slate-900">
      {destinations.map((d) => (
        <Tooltip key={d.key} text={d.tooltip}>
          <button
            onClick={d.onClick}
            className={`flex-1 py-2 text-center text-lg ${
              d.key === activeKey ? "bg-[var(--accent)]" : "dark:hover:bg-slate-800"
            }`}
          >
            {d.icon}
          </button>
        </Tooltip>
      ))}
      <Tooltip text="Lock the vault now">
        <button onClick={onLock} className="flex-1 py-2 text-center text-lg dark:hover:bg-slate-800">
          🔒
        </button>
      </Tooltip>
    </div>
  );
}
