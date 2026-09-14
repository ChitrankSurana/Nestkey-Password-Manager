import { useEffect, useState } from "react";
import { api, isCommandError } from "../lib/api";
import { useVault } from "./hooks/useVault";
import { Sidebar } from "./components/Sidebar";
import { ItemList } from "./components/ItemList";
import { SiteEditor } from "../screens/SiteEditor";
import type { NavDestination } from "../screens/NavRail";

// Sole breakpoint owner (D-18). This task delivers the >=1024px tier: full Sidebar + 280px
// ItemList + flex-1 detail column. Plan 08-04 replaces the detail column's contents with
// DetailPanel; plan 08-06 adds the two narrower responsive tiers.
export function NewShell({ username }: { username: string }) {
  const [collapsed, setCollapsed] = useState(false);
  const [addingSite, setAddingSite] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const vault = useVault();

  useEffect(() => {
    api
      .getConfig()
      .then((c) => setCollapsed(c.navCollapsed))
      .catch(() => {});
  }, []);

  async function toggleCollapsed() {
    const next = !collapsed;
    setCollapsed(next);
    try {
      await api.updateNavCollapsed(next);
    } catch {
      // best-effort persistence - a failed save just means it resets to expanded next launch,
      // not worth surfacing an error banner for a UI-preference toggle (mirrors AppShell.tsx).
    }
  }

  async function handleLock() {
    try {
      setError(null);
      await api.lock();
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    }
  }

  // Copied verbatim from AppShell.tsx's destinations array (D-10 - exactly these six keys,
  // never extended - no favourites/folders/item-type/sync/history entry, not even as a disabled
  // stub). Secondary-screen navigation (health/generator/trash/data/settings panes) is out of
  // this plan's scope - a later plan wires real content behind these keys; for now the buttons
  // render and are tooltip-reachable (UX-01) alongside the vault view this plan builds.
  const destinations: NavDestination[] = [
    { key: "vault", label: "Vault", icon: "house", tooltip: "Your sites and credentials", onClick: () => {} },
    { key: "health", label: "Health", icon: "heart-pulse", tooltip: "Password health dashboard", onClick: () => {} },
    { key: "generator", label: "Generator", icon: "dice-5", tooltip: "Generate a password", onClick: () => {} },
    { key: "trash", label: "Trash", icon: "trash", tooltip: "View trash", onClick: () => {} },
    { key: "data", label: "Data", icon: "box-seam", tooltip: "Backup, export, and import", onClick: () => {} },
    { key: "settings", label: "Settings", icon: "gear", tooltip: "Open settings", onClick: () => {} },
  ];

  return (
    <div className="min-h-screen bg-[var(--bg)] text-[var(--fg)] flex">
      <Sidebar
        destinations={destinations}
        activeKey="vault"
        collapsed={collapsed}
        onToggleCollapsed={toggleCollapsed}
        onLock={handleLock}
        username={username}
      />

      <ItemList vault={vault} onAddSite={() => setAddingSite(true)} />

      <div className="flex-1 p-4">
        {error && <p className="mb-4 text-sm text-red-400">{error}</p>}
        {vault.selectedSite ? (
          <h2 className="text-base font-semibold">{vault.selectedSite.name}</h2>
        ) : (
          <div>
            <p className="text-base font-semibold">Select a site to view its details</p>
            <p className="text-sm opacity-70">Your vault is encrypted locally</p>
          </div>
        )}
      </div>

      {addingSite && (
        <SiteEditor
          onClose={() => setAddingSite(false)}
          onSaved={() => {
            setAddingSite(false);
            vault.refreshSites();
          }}
        />
      )}
    </div>
  );
}
