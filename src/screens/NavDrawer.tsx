import { NavDestination } from "./NavRail";
import { Tooltip } from "../components/Tooltip";
import { Icon } from "../components/Icon";

export function NavDrawer({
  destinations,
  activeKey,
  open,
  onClose,
  onLock,
}: {
  destinations: NavDestination[];
  activeKey: string | null;
  open: boolean;
  onClose: () => void;
  onLock: () => void;
}) {
  if (!open) {
    return null;
  }
  return (
    <div className="sm:hidden fixed inset-0 z-50 flex">
      <div className="absolute inset-0 bg-black/50" onClick={onClose} />
      <div className="relative flex flex-col w-40 shrink-0 gap-1 bg-[var(--bg)] border-r border-[var(--border)] p-2">
        {destinations.map((d) => (
          <Tooltip key={d.key} text={d.tooltip}>
            <button
              onClick={() => {
                d.onClick();
                onClose();
              }}
              className={`rounded px-2 py-1 text-left ${
                d.key === activeKey
                  ? "bg-[var(--accent-subtle-bg)] text-[var(--accent-subtle-fg)]"
                  : "hover:bg-[var(--surface)]"
              }`}
            >
              <Icon name={d.icon} /> {d.label}
            </button>
          </Tooltip>
        ))}
        <div className="flex-1" />
        <Tooltip text="Lock the vault now">
          <button
            onClick={() => {
              onLock();
              onClose();
            }}
            className="rounded px-2 py-1 text-left hover:bg-[var(--surface)]"
          >
            <Icon name="lock-fill" /> Lock
          </button>
        </Tooltip>
      </div>
    </div>
  );
}
