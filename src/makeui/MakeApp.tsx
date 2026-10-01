import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "../lib/api";
import { clearRepromptGrace } from "../lib/reprompt";
import { useActivityWatcher } from "../hooks/useActivityWatcher";
import { Login } from "../screens/Login";
import { Register } from "../screens/Register";
import { ToastHost } from "../components/ToastHost";
import { MakeVaultShell } from "./MakeVaultShell";
import "./makeui.css";

type Screen = "login" | "register" | "vault";

// Real Make shell (step 6): auth (Login/Register reused unrestyled, same precedent as
// src/newui/NewApp.tsx - D-J's username field is already satisfied by the existing multi-user
// screens) wired to a three-pane vault view built for real. Health/Generator/Data/Trash/Settings
// are the same direct passthrough src/newui/screens/*Pane.tsx already uses (D-11) - pixel-fidelity
// restyling for those is the step 10 visual-diff pass, not this step.
//
// B1 Option B: mirrors src/App.tsx and src/newui/NewApp.tsx exactly now - this component owns only
// auth/session/theme, and the actual vault UI (MakeVaultShell, which owns useMakeVault()) mounts
// solely for screen === "vault". No vault-related fetch can fire while locked anymore, so there is
// nothing left for a refetch-on-screen-flip effect to compensate for.
export default function MakeApp() {
  const [screen, setScreen] = useState<Screen>("login");
  const [username, setUsername] = useState<string | null>(null);
  const [dark, setDark] = useState(true);

  // N1: a fresh install had no way to know it was fresh, so it always opened on Login even though
  // there was no account yet to log into (M1.1/M2.1). Only flips to Register if still on the
  // default "login" screen when this resolves - never overrides a screen the user already
  // navigated to/from in the meantime.
  useEffect(() => {
    api
      .vaultExists()
      .then((exists) => {
        if (!exists) setScreen((s) => (s === "login" ? "register" : s));
      })
      .catch(() => {});
  }, []);

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

  let content;
  if (screen === "login") {
    content = (
      <Login
        onSuccess={(u) => {
          setUsername(u);
          setScreen("vault");
        }}
        onSwitchToRegister={() => setScreen("register")}
      />
    );
  } else if (screen === "register") {
    content = (
      <Register
        onSuccess={(u) => {
          setUsername(u);
          setScreen("vault");
        }}
        onSwitchToLogin={() => setScreen("login")}
      />
    );
  } else {
    content = <MakeVaultShell key={username} username={username!} dark={dark} onToggleTheme={toggleTheme} />;
  }

  return (
    <>
      {content}
      <ToastHost />
    </>
  );
}
