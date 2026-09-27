import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, isCommandError, type CredentialDto } from "../lib/api";
import { clearRepromptGrace } from "../lib/reprompt";
import { useActivityWatcher } from "../hooks/useActivityWatcher";
import { Login } from "../screens/Login";
import { Register } from "../screens/Register";
import { CredentialEditor } from "../screens/CredentialEditor";
import { ToastHost } from "../components/ToastHost";
import { useMakeVault } from "./hooks/useMakeVault";
import { Sidebar, type NavTarget } from "./components/Sidebar";
import { ItemList } from "./components/ItemList";
import { DetailPanel } from "./components/DetailPanel";
import { ItemDrawer } from "./components/ItemDrawer";
import { TrashView } from "./components/TrashView";
import { HealthDashboard } from "./components/HealthDashboard";
import { Generator } from "./components/Generator";
import { DataHub } from "./components/DataHub";
import { Settings } from "./components/Settings";
import { IconMenu, IconShield, IconX } from "./icons";
import type { Section } from "./lib/vaultSelectors";
import "./makeui.css";

type Screen = "login" | "register" | "vault";

// Real Make shell (step 6): auth (Login/Register reused unrestyled, same precedent as
// src/newui/NewApp.tsx - D-J's username field is already satisfied by the existing multi-user
// screens) wired to a three-pane vault view built for real. Health/Generator/Data/Trash/Settings
// are the same direct passthrough src/newui/screens/*Pane.tsx already uses (D-11) - pixel-fidelity
// restyling for those is the step 10 visual-diff pass, not this step.
export default function MakeApp() {
  const [screen, setScreen] = useState<Screen>("login");
  const [username, setUsername] = useState<string | null>(null);
  const [nav, setNav] = useState<NavTarget>("all");
  const [dark, setDark] = useState(true);
  const [addingSite, setAddingSite] = useState(false);
  const [editingSite, setEditingSite] = useState(false);
  const [editingCredential, setEditingCredential] = useState<CredentialDto | "new" | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [mobileNavOpen, setMobileNavOpen] = useState(false);
  const vault = useMakeVault();

  // B1: useMakeVault mounts (and fires its one-shot list_sites/list_folders fetch) as soon as
  // MakeApp mounts, while the vault is still locked - the initial fetch is guaranteed to fail with
  // "Vault is locked", and nothing previously refetched after a successful login. Refetch here,
  // keyed on the screen flip, so the item list is populated for the session that's actually
  // unlocked instead of the pre-login one.
  useEffect(() => {
    if (screen === "vault") {
      vault.refreshSites();
      vault.refreshFolders();
    }
  }, [screen]);

  useActivityWatcher(screen === "vault");

  useEffect(() => {
    api
      .getConfig()
      .then((c) => {
        setDark(c.theme === "dark");
        document.documentElement.classList.toggle("dark", c.theme === "dark");
        document.documentElement.setAttribute("data-accent", c.accentColor);
      })
      .catch(() => {
        document.documentElement.classList.add("dark");
        document.documentElement.setAttribute("data-accent", "indigo");
      });
  }, []);

  // A deleted folder must not leave `nav` pointing at a section that no longer exists - falls
  // back to "all" the moment vault.folders no longer contains it (Sidebar's own delete handler
  // has no reason to know what `nav` currently is, so this lives here instead).
  useEffect(() => {
    if (nav.startsWith("folder:") && !vault.folders.some((f) => `folder:${f.id}` === nav)) {
      setNav("all");
    }
  }, [nav, vault.folders]);

  useEffect(() => {
    const unlistenPromise = listen("vault-locked", () => {
      setScreen("login");
      setUsername(null);
      clearRepromptGrace();
    });
    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  function refreshAfterSave() {
    vault.refreshSites();
    vault.refreshFolders();
    if (vault.selectedSiteId !== null) vault.refreshCredentials(vault.selectedSiteId);
  }

  async function toggleTheme() {
    const next = !dark;
    setDark(next);
    document.documentElement.classList.toggle("dark", next);
    try {
      await api.updateTheme(next ? "dark" : "light");
    } catch {
      // best-effort persistence, matches Settings.tsx's own theme toggle
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

  if (screen === "login") {
    return (
      <>
        <Login
          onSuccess={(u) => {
            setUsername(u);
            setScreen("vault");
          }}
          onSwitchToRegister={() => setScreen("register")}
        />
        <ToastHost />
      </>
    );
  }

  if (screen === "register") {
    return (
      <>
        <Register
          onSuccess={(u) => {
            setUsername(u);
            setScreen("vault");
          }}
          onSwitchToLogin={() => setScreen("login")}
        />
        <ToastHost />
      </>
    );
  }

  const isVaultSection = nav === "all" || nav === "favorites" || nav === "logins" || nav === "notes" || nav === "cards" || nav === "identities" || nav.startsWith("folder:");

  // B3-8: MakeApp is the sole breakpoint owner (mirrors src/newui/NewShell.tsx's D-18 rule) - no
  // other file under src/makeui/ branches on viewport width. One breakpoint (md:, 768px), not
  // newui's two-tier split: the prototype has zero responsive chrome to draw a collapsed/icon-rail
  // Sidebar from (figma-make/src/App.tsx:479,616 are fixed widths), so there's no design to port
  // for a middle tier - Sidebar here has only its one full-width form. >=768px: Sidebar always
  // visible, ItemList+DetailPanel side by side. <768px: Sidebar becomes a slide-over triggered by
  // a top-bar menu button, and ItemList/DetailPanel collapse to whichever one selectedSiteId
  // implies, exactly like newui's own <640px tier.
  function navigateAndCloseMobileNav(target: NavTarget) {
    setNav(target);
    setMobileNavOpen(false);
  }

  return (
    <div className="h-screen overflow-hidden bg-mk-base text-mk-fg flex flex-col md:flex-row font-mk-sans">
      <div className="md:hidden h-12 flex items-center gap-3 px-3 border-b border-mk-border bg-mk-sidebar flex-shrink-0">
        <button onClick={() => setMobileNavOpen(true)} title="Open menu" className="p-1 text-mk-muted">
          <IconMenu size={18} />
        </button>
        <div className="w-6 h-6 rounded-lg bg-mk-accent flex items-center justify-center flex-shrink-0">
          <IconShield size={12} className="text-white" />
        </div>
        <span className="text-sm font-semibold text-mk-fg">LocalVault</span>
      </div>

      <div className="hidden md:flex">
        <Sidebar
          active={nav}
          onNavigate={setNav}
          sites={vault.sites}
          folders={vault.folders}
          onFoldersChanged={() => {
            vault.refreshFolders();
            vault.refreshSites();
          }}
          dark={dark}
          onToggleTheme={toggleTheme}
          onLock={handleLock}
        />
      </div>

      {mobileNavOpen && (
        <div className="md:hidden fixed inset-0 z-50 flex">
          <div className="absolute inset-0 bg-black/50" onClick={() => setMobileNavOpen(false)} />
          <div className="relative h-full">
            <Sidebar
              active={nav}
              onNavigate={navigateAndCloseMobileNav}
              sites={vault.sites}
              folders={vault.folders}
              onFoldersChanged={() => {
                vault.refreshFolders();
                vault.refreshSites();
              }}
              dark={dark}
              onToggleTheme={toggleTheme}
              onLock={handleLock}
            />
            <button
              onClick={() => setMobileNavOpen(false)}
              title="Close menu"
              className="absolute top-3 -right-10 w-8 h-8 flex items-center justify-center rounded-lg bg-mk-sidebar text-mk-muted"
            >
              <IconX size={16} />
            </button>
          </div>
        </div>
      )}

      {error && <p className="absolute top-2 left-1/2 -translate-x-1/2 z-40 rounded bg-mk-red px-3 py-1 text-xs text-white">{error}</p>}

      <div className="flex-1 min-h-0 flex overflow-hidden">
        {isVaultSection && (
          <>
            <ItemList
              vault={vault}
              section={nav as Section}
              onAdd={() => setAddingSite(true)}
              onSelect={vault.selectSite}
              className={`${vault.selectedSiteId === null ? "flex" : "hidden"} md:flex w-full md:w-[280px]`}
            />
            <div className={`${vault.selectedSiteId !== null ? "flex" : "hidden"} md:flex min-h-0 flex-1 flex-col`}>
              <DetailPanel
                site={vault.selectedSite}
                folders={vault.folders}
                credentials={vault.credentials}
                credentialsLoading={vault.credentialsLoading}
                error={vault.error}
                onEditSite={() => setEditingSite(true)}
                onAddCredential={() => setEditingCredential("new")}
                onEditCredential={(c) => setEditingCredential(c)}
                onCredentialDeleted={refreshAfterSave}
                onOpenHealth={() => setNav("health")}
                onSiteDeleted={() => {
                  vault.selectSite(null);
                  vault.refreshSites();
                }}
                onFavoriteToggled={refreshAfterSave}
                onBack={() => vault.selectSite(null)}
              />
            </div>
          </>
        )}
        {nav === "health" && (
          <HealthDashboard
            onNavigateToSite={(siteId) => {
              setNav("all");
              vault.selectSite(siteId);
            }}
          />
        )}
        {nav === "generator" && <Generator />}
        {nav === "trash" && <TrashView onRestored={refreshAfterSave} />}
        {nav === "data" && <DataHub />}
        {nav === "settings" && <Settings username={username!} />}
      </div>

      {addingSite && (
        <ItemDrawer
          mode="add"
          existingSite={null}
          existingCredentials={null}
          folders={vault.folders}
          onClose={() => setAddingSite(false)}
          onSaved={() => {
            setAddingSite(false);
            refreshAfterSave();
          }}
        />
      )}

      {editingSite && vault.selectedSite && (
        <ItemDrawer
          mode="edit"
          existingSite={vault.selectedSite}
          existingCredentials={vault.credentials}
          folders={vault.folders}
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

      <ToastHost />
    </div>
  );
}
