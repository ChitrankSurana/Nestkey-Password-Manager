import { useEffect, useState } from "react";
import { api, isCommandError, type CredentialDto } from "../lib/api";
import { useVault } from "./hooks/useVault";
import { Sidebar } from "./components/Sidebar";
import { ItemList } from "./components/ItemList";
import { DetailPanel } from "./components/DetailPanel";
import { SiteEditor } from "../screens/SiteEditor";
import { CredentialEditor } from "../screens/CredentialEditor";
import { GeneratorPane } from "./screens/GeneratorPane";
import { HealthPane } from "./screens/HealthPane";
import { DataPane } from "./screens/DataPane";
import { TrashPane } from "./screens/TrashPane";
import { SettingsPane } from "./screens/SettingsPane";
import { BottomTabBar } from "../screens/BottomTabBar";
import { NavDrawer } from "../screens/NavDrawer";
import { Icon } from "../components/Icon";
import type { NavDestination } from "../screens/NavRail";

type Screen = "vault" | "health" | "generator" | "trash" | "data" | "settings";

// Sole breakpoint owner (D-18). No other file under src/newui/ branches on viewport width -
// every sm:/lg: prefix that controls layout lives here. >=1024px (lg:): full Sidebar, honouring
// the saved navCollapsed preference. 640-1023px (sm: to lg:): the same Sidebar forced into its
// icon-rail form (collapsed=true, showToggle=false) - a layout consequence of width, not a user
// choice, so it never calls api.updateNavCollapsed and never overwrites what the user actually
// chose. Below 640px the sidebar is replaced entirely (see the mobileNavStyle branch below).
export function NewShell({ username }: { username: string }) {
  const [collapsed, setCollapsed] = useState(false);
  const [mobileNavStyle, setMobileNavStyle] = useState<string>("icon-rail");
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [addingSite, setAddingSite] = useState(false);
  const [editingSite, setEditingSite] = useState(false);
  const [editingCredential, setEditingCredential] = useState<CredentialDto | "new" | null>(null);
  const [error, setError] = useState<string | null>(null);
  // Owns which pane occupies the detail column. Only "vault" renders content this plan - the
  // other five destinations dispatch into this same slice once plan 08-05 builds their
  // *Pane.tsx wrappers (GeneratorPane etc.), so this state exists now for them to switch on.
  const [screen, setScreen] = useState<Screen>("vault");
  const vault = useVault();

  // A rename/new-credential must show up without the user reselecting - refreshes the list and,
  // when a site is selected, its credentials too. Both editors' onSaved route here.
  function refreshAfterSave() {
    vault.refreshSites();
    if (vault.selectedSiteId !== null) {
      vault.refreshCredentials(vault.selectedSiteId);
    }
  }

  useEffect(() => {
    api
      .getConfig()
      .then((c) => {
        setCollapsed(c.navCollapsed);
        setMobileNavStyle(c.mobileNavStyle);
      })
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

  function goVault() {
    setScreen("vault");
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
      <div className="hidden lg:flex">
        <Sidebar
          destinations={destinations}
          activeKey={screen}
          collapsed={collapsed}
          onToggleCollapsed={toggleCollapsed}
          onLock={handleLock}
          username={username}
        />
      </div>

      <div className="hidden sm:flex lg:hidden">
        <Sidebar
          destinations={destinations}
          activeKey={screen}
          collapsed={true}
          onToggleCollapsed={() => {}}
          onLock={handleLock}
          username={username}
          showToggle={false}
        />
      </div>

      {/* Below 640px the sidebar is replaced entirely (D-18/D-11), copying AppShell's own
          three-way mobileNavStyle branch verbatim, default case included: neither bottom-tabs
          nor drawer falls back to the same icon-rail Sidebar shape the mid tier already uses. */}
      {mobileNavStyle !== "bottom-tabs" && mobileNavStyle !== "drawer" && (
        <div className="sm:hidden">
          <Sidebar
            destinations={destinations}
            activeKey={screen}
            collapsed={true}
            onToggleCollapsed={() => {}}
            onLock={handleLock}
            username={username}
            showToggle={false}
          />
        </div>
      )}

      {mobileNavStyle === "bottom-tabs" && (
        <BottomTabBar destinations={destinations} activeKey={screen} onLock={handleLock} />
      )}

      {mobileNavStyle === "drawer" && (
        <NavDrawer
          destinations={destinations}
          activeKey={screen}
          open={drawerOpen}
          onClose={() => setDrawerOpen(false)}
          onLock={handleLock}
        />
      )}

      <ItemList
        vault={vault}
        onAddSite={() => setAddingSite(true)}
        className={`${screen === "vault" && vault.selectedSiteId === null ? "flex" : "hidden"} sm:flex w-full sm:w-[280px]`}
      />

      <div
        className={`flex flex-1 flex-col ${mobileNavStyle === "bottom-tabs" ? "pb-16 sm:pb-0" : ""}`}
      >
        {mobileNavStyle === "drawer" && (
          <button
            onClick={() => setDrawerOpen(true)}
            className="sm:hidden self-start m-2 rounded px-2 py-1 hover:bg-[var(--surface)]"
            aria-label="Open menu"
          >
            <Icon name="list" />
          </button>
        )}

        {error && <p className="p-4 pb-0 text-sm text-red-400">{error}</p>}
        {screen === "vault" && (
          // Below 640px exactly one of ItemList/this pane is visible, switched purely by
          // selectedSiteId !== null (D-18) - hidden here when nothing is selected so the list
          // above can occupy the full width instead; always visible from sm: up, where both
          // panes show side by side (unaffected by this tier).
          <div
            className={`${vault.selectedSiteId !== null ? "flex" : "hidden"} sm:flex flex-1 flex-col`}
          >
            {vault.selectedSiteId !== null ? (
              <DetailPanel
                site={vault.selectedSite}
                credentials={vault.credentials}
                credentialsLoading={vault.credentialsLoading}
                error={null}
                onEditSite={() => setEditingSite(true)}
                onAddCredential={() => setEditingCredential("new")}
                onEditCredential={(credential) => setEditingCredential(credential)}
                onCredentialDeleted={refreshAfterSave}
                onOpenHealth={() => setScreen("health")}
                onBack={() => vault.selectSite(null)}
                onSiteDeleted={() => {
                  vault.selectSite(null);
                  vault.refreshSites();
                }}
              />
            ) : (
              <div className="flex flex-1 items-center justify-center bg-[var(--bg)] p-4 text-center">
                <div>
                  <p className="text-base font-semibold">Select a site to view its details</p>
                  <p className="text-sm opacity-70">Your vault is encrypted locally</p>
                </div>
              </div>
            )}
          </div>
        )}
        {screen === "health" && (
          <HealthPane
            onBack={goVault}
            onNavigateToSite={(siteId) => {
              setScreen("vault");
              vault.selectSite(siteId);
            }}
          />
        )}
        {screen === "generator" && <GeneratorPane onBack={goVault} />}
        {screen === "trash" && <TrashPane onBack={goVault} onRestored={refreshAfterSave} />}
        {screen === "data" && <DataPane onBack={goVault} />}
        {screen === "settings" && <SettingsPane onBack={goVault} username={username} />}
      </div>

      {addingSite && (
        <SiteEditor
          onClose={() => setAddingSite(false)}
          onSaved={() => {
            setAddingSite(false);
            refreshAfterSave();
          }}
        />
      )}

      {editingSite && vault.selectedSite && (
        <SiteEditor
          existing={vault.selectedSite}
          credentialCount={vault.credentials?.length ?? null}
          onClose={() => setEditingSite(false)}
          onSaved={() => {
            setEditingSite(false);
            refreshAfterSave();
          }}
          onDeleted={() => {
            setEditingSite(false);
            vault.selectSite(null);
            vault.refreshSites();
          }}
        />
      )}

      {editingCredential !== null && vault.selectedSiteId !== null && (
        <CredentialEditor
          siteId={vault.selectedSiteId}
          existing={editingCredential === "new" ? null : editingCredential}
          onClose={() => setEditingCredential(null)}
          onSaved={() => {
            setEditingCredential(null);
            refreshAfterSave();
          }}
        />
      )}
    </div>
  );
}
