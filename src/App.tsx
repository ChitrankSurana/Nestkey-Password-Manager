import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { Login } from "./screens/Login";
import { Register } from "./screens/Register";
import { AppShell } from "./screens/AppShell";
import { ToastHost } from "./components/ToastHost";
import { useActivityWatcher } from "./hooks/useActivityWatcher";
import { api, isLockRedirectHeld, SESSION_LOCKED_EVENT } from "./lib/api";
import { clearRepromptGrace } from "./lib/reprompt";
import "./App.css";

type Screen = "login" | "register" | "vault";

function App() {
  const [screen, setScreen] = useState<Screen>("login");
  const [username, setUsername] = useState<string | null>(null);

  useActivityWatcher(screen === "vault");

  // N1: a fresh install had no way to know it was fresh, so it always opened on Login even though
  // there was no account yet to log into (M1.1/M2.1). Only flips to Register if still on the
  // default "login" screen when this resolves.
  useEffect(() => {
    api
      .vaultExists()
      .then((exists) => {
        if (!exists) setScreen((s) => (s === "login" ? "register" : s));
      })
      .catch(() => {});
  }, []);

  useEffect(() => {
    // Defaults to dark/indigo (this app's original look) until config loads, matching the
    // hardcoded class index.html used to set unconditionally - no unstyled flash either way.
    api
      .getConfig()
      .then((c) => {
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
      if (isLockRedirectHeld()) return;
      setScreen("login");
      setUsername(null);
      clearRepromptGrace();
    });
    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  useEffect(() => {
    function onSessionLocked() {
      if (isLockRedirectHeld()) return;
      setScreen("login");
      setUsername(null);
      clearRepromptGrace();
    }
    window.addEventListener(SESSION_LOCKED_EVENT, onSessionLocked);
    return () => window.removeEventListener(SESSION_LOCKED_EVENT, onSessionLocked);
  }, []);

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
    content = <AppShell username={username!} />;
  }

  return (
    <>
      {content}
      <ToastHost />
    </>
  );
}

export default App;
