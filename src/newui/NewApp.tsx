import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { Login } from "../screens/Login";
import { Register } from "../screens/Register";
import { ToastHost } from "../components/ToastHost";
import { useActivityWatcher } from "../hooks/useActivityWatcher";
import { api, isCommandError, type SiteDto } from "../lib/api";
import "../App.css";

type Screen = "login" | "register" | "vault";

function VaultTracer({ username }: { username: string }) {
  const [sites, setSites] = useState<SiteDto[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api
      .listSites()
      .then((s) => setSites(s))
      .catch((err) => {
        setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
      });
  }, []);

  return (
    <main className="bg-[var(--bg)] text-[var(--fg)] min-h-screen p-4">
      <div className="bg-[var(--surface)] border border-[var(--border)] rounded-lg p-4">
        <h1 className="text-xl font-semibold">Vault — {username}</h1>
        {error && <p className="text-sm text-[var(--danger)]">{error}</p>}
        <p className="text-sm opacity-70">{sites.length} site{sites.length === 1 ? "" : "s"}</p>
        <ul>
          {sites.map((s) => (
            <li key={s.id}>{s.name}</li>
          ))}
        </ul>
      </div>
    </main>
  );
}

function NewApp() {
  const [screen, setScreen] = useState<Screen>("login");
  const [username, setUsername] = useState<string | null>(null);

  useActivityWatcher(screen === "vault");

  useEffect(() => {
    // Defaults to dark/indigo (this app's original look) until config loads, matching the
    // hardcoded class newui.html used to set unconditionally - no unstyled flash either way.
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
    // Tracer proof that this root reaches the real backend through src/lib/api.ts.
    // Plan 08-03 replaces this with <NewShell username={username!} />.
    content = <VaultTracer username={username!} />;
  }

  return (
    <>
      {content}
      <ToastHost />
    </>
  );
}

export default NewApp;
