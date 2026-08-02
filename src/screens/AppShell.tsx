import { useEffect, useState } from "react";
import { api, isCommandError } from "../lib/api";
import { Vault } from "./Vault";
import { SiteDetail } from "./SiteDetail";
import { Trash } from "./Trash";
import { Settings } from "./Settings";
import { NavRail, NavDestination } from "./NavRail";
import { BottomTabBar } from "./BottomTabBar";
import { NavDrawer } from "./NavDrawer";
import { Data } from "./Data";
import { HealthDashboard } from "./HealthDashboard";
import { Generator } from "./Generator";
import { Icon } from "../components/Icon";

type ShellScreen = "vault" | "health" | "generator" | "trash" | "data" | "settings";

// Persistent app shell (Phase 1 of the UI plan): nav rail lives here, above every screen, instead
// of only inside Vault.tsx - so it survives navigating into Settings/Health/Generator/Data/Trash.
export function AppShell({ username }: { username: string }) {
  const [screen, setScreen] = useState<ShellScreen>("vault");
  const [selectedSiteId, setSelectedSiteId] = useState<number | null>(null);
  const [vaultRefreshSignal, setVaultRefreshSignal] = useState(0);
  const [collapsed, setCollapsed] = useState(false);
  const [mobileNavStyle, setMobileNavStyle] = useState<string>("icon-rail");
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [error, setError] = useState<string | null>(null);

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
      // not worth surfacing an error banner for a UI-preference toggle.
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

  const destinations: NavDestination[] = [
    { key: "vault", label: "Vault", icon: "house", tooltip: "Your sites and credentials", onClick: () => setScreen("vault") },
    { key: "health", label: "Health", icon: "heart-pulse", tooltip: "Password health dashboard", onClick: () => setScreen("health") },
    { key: "generator", label: "Generator", icon: "dice-5", tooltip: "Generate a password", onClick: () => setScreen("generator") },
    { key: "trash", label: "Trash", icon: "trash", tooltip: "View trash", onClick: () => setScreen("trash") },
    { key: "data", label: "Data", icon: "box-seam", tooltip: "Backup, export, and import", onClick: () => setScreen("data") },
    { key: "settings", label: "Settings", icon: "gear", tooltip: "Open settings", onClick: () => setScreen("settings") },
  ];

  function goVault() {
    setScreen("vault");
  }

  return (
    <div className="min-h-screen bg-[var(--bg)] text-[var(--fg)] flex">
      <div className="hidden sm:flex">
        <NavRail
          collapsed={collapsed}
          onToggleCollapsed={toggleCollapsed}
          activeKey={screen}
          onLock={handleLock}
          destinations={destinations}
        />
      </div>

      {mobileNavStyle !== "bottom-tabs" && mobileNavStyle !== "drawer" && (
        <div className="sm:hidden">
          <NavRail
            collapsed={true}
            onToggleCollapsed={() => {}}
            activeKey={screen}
            onLock={handleLock}
            destinations={destinations}
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

      <div className={`flex-1 flex flex-col ${mobileNavStyle === "bottom-tabs" ? "pb-16 sm:pb-0" : ""}`}>
        {mobileNavStyle === "drawer" && (
          <button
            onClick={() => setDrawerOpen(true)}
            className="sm:hidden self-start m-2 rounded px-2 py-1 hover:bg-[var(--surface)]"
            aria-label="Open menu"
          >
            <Icon name="list" />
          </button>
        )}

        {error && <p className="text-sm text-red-400 px-4 pt-4">{error}</p>}

        {screen === "vault" && (
          <Vault username={username} onSelectSite={setSelectedSiteId} refreshSignal={vaultRefreshSignal} />
        )}
        {screen === "health" && (
          <HealthDashboard
            onBack={goVault}
            onNavigateToSite={(siteId) => {
              setScreen("vault");
              setSelectedSiteId(siteId);
            }}
          />
        )}
        {screen === "generator" && <Generator onBack={goVault} />}
        {screen === "trash" && <Trash onBack={goVault} />}
        {screen === "data" && <Data onBack={goVault} />}
        {screen === "settings" && <Settings onBack={goVault} username={username} />}
      </div>

      {selectedSiteId !== null && (
        <SiteDetail
          siteId={selectedSiteId}
          onBack={() => {
            setSelectedSiteId(null);
            setVaultRefreshSignal((n) => n + 1);
          }}
        />
      )}
    </div>
  );
}
