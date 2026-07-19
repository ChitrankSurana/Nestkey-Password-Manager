import { NavDestination } from "./NavRail";
import { Tooltip } from "../components/Tooltip";

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
      <div className="relative flex flex-col w-40 shrink-0 gap-1 dark:bg-slate-900 border-r dark:border-slate-700 p-2">
        {destinations.map((d) => (
          <Tooltip key={d.key} text={d.tooltip}>
            <button
              onClick={() => {
                d.onClick();
                onClose();
              }}
              className={`rounded px-2 py-1 text-left ${
                d.key === activeKey ? "bg-[var(--accent)]" : "dark:hover:bg-slate-800"
              }`}
            >
              {d.icon} {d.label}
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
            className="rounded px-2 py-1 text-left dark:hover:bg-slate-800"
          >
            🔒 Lock
          </button>
        </Tooltip>
      </div>
    </div>
  );
}
