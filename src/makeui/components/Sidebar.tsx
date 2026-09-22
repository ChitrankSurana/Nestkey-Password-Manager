import { useState, type ComponentType } from "react";
import type { SiteDto, FolderDto } from "../../lib/api";
import {
  IconActivity,
  IconCard,
  IconChevron,
  IconDatabase,
  IconFolder,
  IconKey,
  IconLock,
  IconMoon,
  IconNote,
  IconPerson,
  IconSettings,
  IconShield,
  IconStar,
  IconSun,
  IconTrash,
  IconZap,
  type IconProps,
} from "../icons";
import type { Section } from "../lib/vaultSelectors";

export type NavTarget = Section | "health" | "generator" | "data" | "trash" | "settings";

// Ported from figma-make/src/App.tsx:438-560 (Sidebar) onto real data: counts come from the
// fetched site list (no VAULT_ITEMS mock), folders are the real per-vault list from useMakeVault
// (no hardcoded FOLDERS array), and theme/lock use the real api - src/newui/hooks/useVault.ts has
// no dark-mode toggle of its own, so this reads/writes theme directly via getConfig/updateTheme.
export function Sidebar({
  active,
  onNavigate,
  sites,
  folders,
  dark,
  onToggleTheme,
  onLock,
}: {
  active: NavTarget;
  onNavigate: (target: NavTarget) => void;
  sites: SiteDto[];
  folders: FolderDto[];
  dark: boolean;
  onToggleTheme: () => void;
  onLock: () => void;
}) {
  const [foldersOpen, setFoldersOpen] = useState(false);

  const live = sites; // trash is not modeled as a site state here - soft-deleted sites drop out of listSites
  const counts = {
    login: live.filter((s) => s.itemType === "login").length,
    note: live.filter((s) => s.itemType === "note").length,
    card: live.filter((s) => s.itemType === "card").length,
    identity: live.filter((s) => s.itemType === "identity").length,
    fav: live.filter((s) => s.favorite).length,
    all: live.length,
  };

  function navItem(id: NavTarget, label: string, Icon: ComponentType<IconProps>, count?: number) {
    const isActive = active === id;
    return (
      <button
        key={id}
        onClick={() => onNavigate(id)}
        className={`w-full flex items-center gap-2.5 px-3 py-2 rounded-lg text-sm transition-colors duration-150 ${
          isActive ? "bg-mk-accent-muted text-mk-accent-light" : "text-mk-muted hover:bg-mk-hover hover:text-mk-fg2"
        }`}
      >
        <Icon size={15} />
        <span className="flex-1 text-left">{label}</span>
        {count !== undefined && (
          <span
            className={`text-xs px-1.5 py-0.5 rounded-full font-medium ${
              isActive ? "bg-mk-badge-active text-mk-accent-light" : "bg-mk-badge text-mk-faint"
            }`}
          >
            {count}
          </span>
        )}
      </button>
    );
  }

  return (
    <aside className="w-[220px] flex-shrink-0 flex flex-col h-full bg-mk-sidebar border-r border-mk-border">
      <div className="px-4 h-14 flex items-center gap-2.5 border-b border-mk-border">
        <div className="w-7 h-7 rounded-lg bg-mk-accent flex items-center justify-center flex-shrink-0">
          <IconShield size={15} className="text-white" />
        </div>
        <div>
          <div className="text-sm font-semibold leading-tight text-mk-fg">LocalVault</div>
          <div className="text-[10px] leading-tight text-mk-faint">Personal</div>
        </div>
      </div>

      <nav className="flex-1 overflow-y-auto p-2 space-y-0.5">
        <div className="px-2 pt-1 pb-1">
          <p className="text-[10px] font-semibold uppercase tracking-widest text-mk-label">Vault</p>
        </div>
        {navItem("all", "All Items", IconKey, counts.all)}
        {navItem("favorites", "Favorites", IconStar, counts.fav)}

        <div className="px-2 pt-3 pb-1">
          <p className="text-[10px] font-semibold uppercase tracking-widest text-mk-label">Types</p>
        </div>
        {navItem("logins", "Logins", IconKey, counts.login)}
        {navItem("notes", "Secure Notes", IconNote, counts.note)}
        {navItem("cards", "Payment Cards", IconCard, counts.card)}
        {navItem("identities", "Identities", IconPerson, counts.identity)}

        <div className="pt-2">
          <button
            onClick={() => setFoldersOpen(!foldersOpen)}
            className="w-full flex items-center gap-2 px-2 py-1 rounded text-[10px] font-semibold uppercase tracking-widest text-mk-label transition-colors"
          >
            <IconChevron size={10} className={`transition-transform duration-200 ${foldersOpen ? "rotate-90" : ""}`} />
            Folders
          </button>
          {foldersOpen && (
            <div className="mt-0.5 space-y-0.5">
              {folders.length === 0 && <p className="px-3 py-1 text-xs text-mk-faint">No folders yet</p>}
              {folders.map((f) => navItem(`folder:${f.id}`, f.name, IconFolder))}
            </div>
          )}
        </div>

        <div className="pt-2">
          <div className="px-2 pb-1">
            <p className="text-[10px] font-semibold uppercase tracking-widest text-mk-label">Other</p>
          </div>
          {navItem("health", "Health Dashboard", IconActivity)}
          {navItem("generator", "Password Generator", IconZap)}
          {navItem("data", "Data Hub", IconDatabase)}
          {navItem("trash", "Trash", IconTrash)}
        </div>
      </nav>

      <div className="p-2 flex items-center gap-1 border-t border-mk-border">
        <button onClick={onLock} title="Lock Vault" className="flex items-center justify-center w-8 h-8 rounded-lg text-mk-faint hover:opacity-80 transition-opacity">
          <IconLock size={13} />
        </button>
        <button
          onClick={onToggleTheme}
          title={dark ? "Switch to light mode" : "Switch to dark mode"}
          className="flex items-center justify-center w-8 h-8 rounded-lg text-mk-faint hover:opacity-80 transition-opacity"
        >
          {dark ? <IconSun size={13} /> : <IconMoon size={13} />}
        </button>
        <button
          onClick={() => onNavigate("settings")}
          title="Settings"
          className={`flex items-center justify-center w-8 h-8 rounded-lg hover:opacity-80 transition-opacity ${active === "settings" ? "text-mk-accent-light" : "text-mk-faint"}`}
        >
          <IconSettings size={13} />
        </button>
      </div>
    </aside>
  );
}
