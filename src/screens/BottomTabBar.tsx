import { NavDestination } from "./NavRail";
import { Tooltip } from "../components/Tooltip";
import { Icon } from "../components/Icon";

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
    <div className="sm:hidden fixed bottom-0 inset-x-0 z-40 flex border-t border-[var(--border)] bg-[var(--bg)]">
      {destinations.map((d) => (
        <Tooltip key={d.key} text={d.tooltip}>
          <button
            onClick={d.onClick}
            className={`flex-1 py-2 text-center text-lg ${
              d.key === activeKey
                ? "bg-[var(--accent-subtle-bg)] text-[var(--accent-subtle-fg)]"
                : "hover:bg-[var(--surface)]"
            }`}
          >
            <Icon name={d.icon} />
          </button>
        </Tooltip>
      ))}
      <Tooltip text="Lock the vault now">
        <button onClick={onLock} className="flex-1 py-2 text-center text-lg hover:bg-[var(--surface)]">
          <Icon name="lock-fill" />
        </button>
      </Tooltip>
    </div>
  );
}
