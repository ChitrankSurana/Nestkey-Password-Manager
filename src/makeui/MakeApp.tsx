import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, isCommandError, type CredentialDto } from "../lib/api";
import { clearRepromptGrace } from "../lib/reprompt";
import { useActivityWatcher } from "../hooks/useActivityWatcher";
import { Login } from "../screens/Login";
import { Register } from "../screens/Register";
import { CredentialEditor } from "../screens/CredentialEditor";
import { Generator } from "../screens/Generator";
import { HealthDashboard } from "../screens/HealthDashboard";
import { Data } from "../screens/Data";
import { Trash } from "../screens/Trash";
import { Settings } from "../screens/Settings";
import { ToastHost } from "../components/ToastHost";
import { useMakeVault } from "./hooks/useMakeVault";
import { Sidebar, type NavTarget } from "./components/Sidebar";
import { ItemList } from "./components/ItemList";
import { DetailPanel } from "./components/DetailPanel";
import { ItemDrawer } from "./components/ItemDrawer";
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
  const vault = useMakeVault();

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

  function goVault() {
    setNav("all");
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

  return (
    <div className="h-screen overflow-hidden bg-mk-base text-mk-fg flex font-mk-sans">
      <Sidebar active={nav} onNavigate={setNav} sites={vault.sites} folders={vault.folders} dark={dark} onToggleTheme={toggleTheme} onLock={handleLock} />

      {error && <p className="absolute top-2 left-1/2 -translate-x-1/2 z-40 rounded bg-mk-red px-3 py-1 text-xs text-white">{error}</p>}

      {isVaultSection && (
        <>
          <ItemList vault={vault} section={nav as Section} onAdd={() => setAddingSite(true)} onSelect={vault.selectSite} />
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
          />
        </>
      )}
      {nav === "health" && (
        <HealthDashboard
          onBack={goVault}
          onNavigateToSite={(siteId) => {
            setNav("all");
            vault.selectSite(siteId);
          }}
        />
      )}
      {nav === "generator" && <Generator onBack={goVault} />}
      {nav === "trash" && <Trash onBack={goVault} onRestored={refreshAfterSave} />}
      {nav === "data" && <Data onBack={goVault} />}
      {nav === "settings" && <Settings onBack={goVault} username={username!} />}

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
