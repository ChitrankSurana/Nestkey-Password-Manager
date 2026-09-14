import { useEffect, useState } from "react";
import { api, isCommandError } from "../lib/api";
import { useVault } from "./hooks/useVault";
import { Sidebar } from "./components/Sidebar";
import { ItemList } from "./components/ItemList";
import { DetailPanel } from "./components/DetailPanel";
import { SiteEditor } from "../screens/SiteEditor";
import type { NavDestination } from "../screens/NavRail";

type Screen = "vault" | "health" | "generator" | "trash" | "data" | "settings";

// Sole breakpoint owner (D-18). This task delivers the >=1024px tier: full Sidebar + 280px
// ItemList + flex-1 detail column, now rendering DetailPanel for the current selection. Plan
// 08-06 adds the two narrower responsive tiers.
export function NewShell({ username }: { username: string }) {
  const [collapsed, setCollapsed] = useState(false);
  const [addingSite, setAddingSite] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // Owns which pane occupies the detail column. Only "vault" renders content this plan - the
  // other five destinations dispatch into this same slice once plan 08-05 builds their
  // *Pane.tsx wrappers (GeneratorPane etc.), so this state exists now for them to switch on.
  const [screen, setScreen] = useState<Screen>("vault");
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
  // stub). Each destination now dispatches into the `screen` slice, matching AppShell.tsx's own
  // setScreen pattern - only "vault" has a rendered pane this plan, but the buttons are already
  // tooltip-reachable (UX-01) and wired for plan 08-05 to give the other five real content.
  const destinations: NavDestination[] = [
    { key: "vault", label: "Vault", icon: "house", tooltip: "Your sites and credentials", onClick: () => setScreen("vault") },
    { key: "health", label: "Health", icon: "heart-pulse", tooltip: "Password health dashboard", onClick: () => setScreen("health") },
    { key: "generator", label: "Generator", icon: "dice-5", tooltip: "Generate a password", onClick: () => setScreen("generator") },
    { key: "trash", label: "Trash", icon: "trash", tooltip: "View trash", onClick: () => setScreen("trash") },
    { key: "data", label: "Data", icon: "box-seam", tooltip: "Backup, export, and import", onClick: () => setScreen("data") },
    { key: "settings", label: "Settings", icon: "gear", tooltip: "Open settings", onClick: () => setScreen("settings") },
  ];

  return (
    <div className="min-h-screen bg-[var(--bg)] text-[var(--fg)] flex">
      <Sidebar
        destinations={destinations}
        activeKey={screen}
        collapsed={collapsed}
        onToggleCollapsed={toggleCollapsed}
        onLock={handleLock}
        username={username}
      />

      <ItemList vault={vault} onAddSite={() => setAddingSite(true)} />

      <div className="flex flex-1 flex-col">
        {error && <p className="p-4 pb-0 text-sm text-red-400">{error}</p>}
        {screen === "vault" &&
          (vault.selectedSiteId !== null ? (
            <DetailPanel
              site={vault.selectedSite}
              credentials={vault.credentials}
              credentialsLoading={vault.credentialsLoading}
              error={null}
              // CredentialEditor wiring (add/edit credential) is CRUD-wiring scope, a later plan
              // per CONTEXT.md's phasing - this plan only builds the read/reveal/copy column.
              onAddCredential={() => {}}
            />
          ) : (
            <div className="flex flex-1 items-center justify-center bg-[var(--bg)] p-4 text-center">
              <div>
                <p className="text-base font-semibold">Select a site to view its details</p>
                <p className="text-sm opacity-70">Your vault is encrypted locally</p>
              </div>
            </div>
          ))}
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
