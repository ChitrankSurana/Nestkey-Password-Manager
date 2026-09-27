import { useState, type ComponentType } from "react";
import { api, isCommandError, type SiteDto, type FolderDto } from "../../lib/api";
import { confirmDialog } from "../../lib/confirm";
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
  IconPlus,
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

// One folder row: click navigates like any other nav item, hover reveals Rename/Delete (B3-9 gap
// surface - the prototype's own Folders list is fixed with no CRUD UI at all, so this has no
// direct Figma reference; kept in the same visual language as navItem). Rename edits in place via
// a plain input (Enter/blur commits, Escape cancels) rather than a modal - one folder, one small
// change, doesn't need drawer-weight chrome. Delete unfiles rather than deletes its sites
// (do_delete_folder's own contract), so both callbacks refresh sites too, not just folders.
function FolderRow({
  folder,
  active,
  onSelect,
  onChanged,
}: {
  folder: FolderDto;
  active: boolean;
  onSelect: () => void;
  onChanged: () => void;
}) {
  const [editing, setEditing] = useState(false);
  const [nameInput, setNameInput] = useState(folder.name);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // J5/J6: commit only on Enter; blur and Escape both cancel (discard the edit, revert the input,
  // clear any stale error) instead of committing. Previously onBlur was wired straight to
  // commitRename with no way to tell "Escape's own state update just unmounted this input" apart
  // from "the user clicked elsewhere to confirm" - so Escape's unmount-blur silently re-committed
  // the very edit it was meant to cancel, and Enter (which sets editing=false itself) triggered a
  // second commitRename via the blur that followed.
  function cancelEdit() {
    setEditing(false);
    setNameInput(folder.name);
    setError(null);
  }

  async function commitRename() {
    const trimmed = nameInput.trim();
    if (trimmed === "" || trimmed === folder.name) {
      cancelEdit();
      return;
    }
    setBusy(true);
    try {
      setError(null);
      await api.renameFolder(folder.id, trimmed);
      setEditing(false);
      onChanged();
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Couldn't rename folder.");
    } finally {
      setBusy(false);
    }
  }

  async function handleDelete() {
    if (!(await confirmDialog(`Delete folder "${folder.name}"? Its items will be unfiled, not deleted.`))) return;
    setBusy(true);
    try {
      setError(null);
      await api.deleteFolder(folder.id);
      onChanged();
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Couldn't delete folder.");
      setBusy(false);
    }
  }

  if (editing) {
    return (
      <div>
        <input
          autoFocus
          value={nameInput}
          disabled={busy}
          onChange={(e) => setNameInput(e.target.value)}
          onBlur={cancelEdit}
          onKeyDown={(e) => {
            if (e.key === "Enter") commitRename();
            if (e.key === "Escape") cancelEdit();
          }}
          className="w-full px-3 py-1.5 text-sm rounded-lg bg-mk-input border border-mk-accent text-mk-fg focus:outline-none"
        />
        {error && <p className="px-3 pt-1 text-[10px] text-mk-red">{error}</p>}
      </div>
    );
  }

  return (
    <div>
      <div
        className={`group w-full flex items-center gap-2.5 px-3 py-2 rounded-lg text-sm transition-colors duration-150 ${
          active ? "bg-mk-accent-muted text-mk-accent-light" : "text-mk-muted hover:bg-mk-hover hover:text-mk-fg2"
        }`}
      >
        <button onClick={onSelect} className="flex-1 flex items-center gap-2.5 min-w-0 text-left">
          <IconFolder size={15} />
          <span className="truncate">{folder.name}</span>
        </button>
        <div className="hidden group-hover:flex items-center gap-2 flex-shrink-0">
          <button onClick={() => setEditing(true)} title="Rename folder" className="text-[10px] hover:text-mk-fg2">
            Rename
          </button>
          <button onClick={handleDelete} title="Delete folder" className="hover:text-mk-red">
            <IconTrash size={11} />
          </button>
        </div>
      </div>
      {error && <p className="px-3 text-[10px] text-mk-red">{error}</p>}
    </div>
  );
}

// Ported from figma-make/src/App.tsx:438-560 (Sidebar) onto real data: counts come from the
// fetched site list (no VAULT_ITEMS mock), folders are the real per-vault list from useMakeVault
// (no hardcoded FOLDERS array), and theme/lock use the real api - src/newui/hooks/useVault.ts has
// no dark-mode toggle of its own, so this reads/writes theme directly via getConfig/updateTheme.
export function Sidebar({
  active,
  onNavigate,
  sites,
  folders,
  onFoldersChanged,
  dark,
  onToggleTheme,
  onLock,
}: {
  active: NavTarget;
  onNavigate: (target: NavTarget) => void;
  sites: SiteDto[];
  folders: FolderDto[];
  onFoldersChanged: () => void;
  dark: boolean;
  onToggleTheme: () => void;
  onLock: () => void;
}) {
  const [foldersOpen, setFoldersOpen] = useState(false);
  const [addingFolder, setAddingFolder] = useState(false);
  const [newFolderName, setNewFolderName] = useState("");
  const [addError, setAddError] = useState<string | null>(null);

  // J5/J6: same fix as FolderRow's cancelEdit/commitRename above - commit only on Enter, cancel
  // (and clear addError) on blur/Escape instead of committing on blur.
  function cancelAddFolder() {
    setAddingFolder(false);
    setNewFolderName("");
    setAddError(null);
  }

  async function commitAddFolder() {
    const trimmed = newFolderName.trim();
    if (trimmed === "") {
      cancelAddFolder();
      return;
    }
    try {
      setAddError(null);
      await api.addFolder(trimmed);
      setNewFolderName("");
      setAddingFolder(false);
      onFoldersChanged();
    } catch (err) {
      setAddError(isCommandError(err) ? err.message : "Couldn't add folder.");
    }
  }

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
          <div className="flex items-center justify-between px-2 py-1">
            <button
              onClick={() => setFoldersOpen(!foldersOpen)}
              className="flex items-center gap-2 rounded text-[10px] font-semibold uppercase tracking-widest text-mk-label transition-colors"
            >
              <IconChevron size={10} className={`transition-transform duration-200 ${foldersOpen ? "rotate-90" : ""}`} />
              Folders
            </button>
            <button onClick={() => { setFoldersOpen(true); setAddingFolder(true); }} title="Add folder" className="text-mk-faint hover:text-mk-fg2">
              <IconPlus size={11} />
            </button>
          </div>
          {foldersOpen && (
            <div className="mt-0.5 space-y-0.5">
              {addingFolder && (
                <input
                  autoFocus
                  value={newFolderName}
                  onChange={(e) => setNewFolderName(e.target.value)}
                  onBlur={cancelAddFolder}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") commitAddFolder();
                    if (e.key === "Escape") cancelAddFolder();
                  }}
                  placeholder="Folder name"
                  className="w-full px-3 py-1.5 text-sm rounded-lg bg-mk-input border border-mk-accent text-mk-fg focus:outline-none"
                />
              )}
              {addError && <p className="px-3 text-[10px] text-mk-red">{addError}</p>}
              {!addingFolder && folders.length === 0 && <p className="px-3 py-1 text-xs text-mk-faint">No folders yet</p>}
              {folders.map((f) => (
                <FolderRow
                  key={f.id}
                  folder={f}
                  active={active === `folder:${f.id}`}
                  onSelect={() => onNavigate(`folder:${f.id}`)}
                  onChanged={onFoldersChanged}
                />
              ))}
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
