import { useEffect, useState } from "react";
import { api, isCommandError, type TrashedCredentialDto } from "../../lib/api";
import { confirmDialog } from "../../lib/confirm";
import { itemTypeConfig } from "../lib/vaultSelectors";
import { IconShield, IconTrash } from "../icons";

// Trash as a section (matches figma-make/src/App.tsx:705-715,2203 - the prototype has no separate
// Trash page, it's a filter on the normal list/detail chrome with Restore / Delete Forever in the
// detail header). Can't literally reuse ItemList/DetailPanel here: those read `vault.sites`
// (`listSites` excludes every trashed row, cascade-deleted sites included), while trash operates
// on `list_trash`'s own TrashedCredentialDto shape (credential-level, not site-level) - so this is
// a second, smaller list+detail pair sharing the same visual language, not the same data source.
// Per-item permanent delete isn't offered: the backend has only bulk purge_trash (no
// purge_one_credential command), matching what Classic's Trash.tsx has always offered - adding a
// new backend command is out of scope for a restyle.
export function TrashView({ onRestored }: { onRestored?: () => void }) {
  const [items, setItems] = useState<TrashedCredentialDto[] | null>(null);
  const [selected, setSelected] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function refresh() {
    try {
      setError(null);
      const list = await api.listTrash();
      setItems(list);
      if (selected !== null && !list.some((t) => t.credentialId === selected)) {
        setSelected(null);
      }
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    }
  }

  useEffect(() => {
    refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const selectedItem = items?.find((t) => t.credentialId === selected) ?? null;

  async function handleRestore(credentialId: number) {
    setBusy(true);
    try {
      setError(null);
      await api.restoreCredential(credentialId);
      await refresh();
      onRestored?.();
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    } finally {
      setBusy(false);
    }
  }

  async function handleEmptyTrash() {
    if (!items || items.length === 0) return;
    if (!(await confirmDialog(`Permanently delete ${items.length} item(s)? This cannot be undone.`))) return;
    setBusy(true);
    try {
      setError(null);
      await api.purgeTrash();
      await refresh();
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <>
      <div className="flex w-full md:w-[280px] flex-shrink-0 flex-col h-full bg-mk-panel border-r border-mk-border">
        <div className="h-14 px-3 flex items-center justify-between border-b border-mk-border">
          <h2 className="text-sm font-semibold text-mk-fg">Trash</h2>
          <div className="flex items-center gap-1">
            <span className="text-xs mr-1 text-mk-faint">{items?.length ?? 0}</span>
            <button
              onClick={handleEmptyTrash}
              disabled={busy || !items || items.length === 0}
              title="Permanently delete everything in Trash"
              className="w-7 h-7 flex items-center justify-center rounded-lg bg-mk-red hover:bg-mk-red-hover transition-colors disabled:opacity-50"
            >
              <IconTrash size={13} className="text-white" />
            </button>
          </div>
        </div>

        {error && <p className="px-3 pt-2 text-xs text-mk-red">{error}</p>}

        <div className="flex-1 overflow-y-auto">
          {items === null && <p className="p-4 text-xs text-mk-faint">Loading…</p>}
          {items !== null && items.length === 0 && (
            <div className="flex flex-col items-center justify-center h-full gap-2 p-6">
              <IconTrash size={24} className="text-mk-faint" />
              <p className="text-xs text-center text-mk-faint">Trash is empty</p>
            </div>
          )}
          {items?.map((t) => {
            const cfg = itemTypeConfig(t.itemType);
            const { Icon } = cfg;
            return (
              <button
                key={t.credentialId}
                onClick={() => setSelected(t.credentialId)}
                className={`w-full flex items-center gap-3 px-3 py-2.5 border-b border-mk-border transition-colors duration-100 ${
                  selected === t.credentialId ? "bg-mk-accent-muted border-l-2 border-l-mk-accent" : "border-l-2 border-l-transparent hover:bg-mk-hover"
                }`}
              >
                <div className={`w-8 h-8 rounded-lg flex items-center justify-center flex-shrink-0 ${cfg.bg}`}>
                  <Icon size={14} className={cfg.fg} />
                </div>
                <div className="flex-1 min-w-0 text-left">
                  <div className="text-sm font-medium truncate text-mk-fg2">{t.siteName}</div>
                  <div className="text-xs truncate text-mk-faint">{t.username ?? "Deleted " + t.deletedAt}</div>
                </div>
              </button>
            );
          })}
        </div>
      </div>

      <div className={`${selected !== null ? "flex" : "hidden"} md:flex min-h-0 flex-1 flex-col bg-mk-base`}>
        {selectedItem === null ? (
          <div className="flex-1 flex flex-col items-center justify-center gap-3">
            <div className="w-16 h-16 rounded-2xl flex items-center justify-center bg-mk-accent-muted">
              <IconShield size={28} className="text-mk-accent opacity-60" />
            </div>
            <p className="text-sm font-medium text-mk-faint">Select an item to restore it</p>
          </div>
        ) : (
          <div className="flex-1 overflow-y-auto px-5 py-4">
            <div className="h-14 -mx-5 -mt-4 px-5 flex items-center justify-between flex-shrink-0 border-b border-mk-border mb-4">
              <div>
                <h1 className="text-sm font-semibold text-mk-fg">{selectedItem.siteName}</h1>
                <p className="text-xs text-mk-faint">Deleted {selectedItem.deletedAt}</p>
              </div>
              <button
                onClick={() => handleRestore(selectedItem.credentialId)}
                disabled={busy}
                className="px-3 py-1.5 rounded-lg text-xs font-medium bg-mk-accent text-white disabled:opacity-50"
              >
                Restore
              </button>
            </div>
            {selectedItem.username && (
              <div className="p-3 rounded-xl border border-mk-border bg-mk-card">
                <p className="text-[10px] font-medium uppercase tracking-widest mb-1 text-mk-faint">Username / Email</p>
                <p className="text-sm text-mk-fg2">{selectedItem.username}</p>
              </div>
            )}
            <p className="text-xs text-mk-faint mt-4">
              Deleting permanently is only available in bulk via "Empty Trash" - restore this item first if you want to keep it.
            </p>
          </div>
        )}
      </div>
    </>
  );
}
