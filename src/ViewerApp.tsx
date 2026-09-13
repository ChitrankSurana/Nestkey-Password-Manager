import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { Login } from "./screens/Login";
import { ViewerVault } from "./screens/ViewerVault";
import { ToastHost } from "./components/ToastHost";
import { useActivityWatcher } from "./hooks/useActivityWatcher";
import { api } from "./lib/api";
import "./App.css";

type Screen = "login" | "vault";

export function ViewerApp() {
  const [screen, setScreen] = useState<Screen>("login");
  const [username, setUsername] = useState<string | null>(null);

  useActivityWatcher(screen === "vault");

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
      setScreen("login");
      setUsername(null);
    });
    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  return (
    <>
      {screen === "login" ? (
        <Login
          onSuccess={(u) => {
            setUsername(u);
            setScreen("vault");
          }}
        />
      ) : (
        <ViewerVault username={username!} />
      )}
      <ToastHost />
    </>
  );
}
