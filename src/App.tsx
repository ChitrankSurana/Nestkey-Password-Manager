import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { Login } from "./screens/Login";
import { Register } from "./screens/Register";
import { Vault } from "./screens/Vault";
import { useActivityWatcher } from "./hooks/useActivityWatcher";
import "./App.css";

type Screen = "login" | "register" | "vault";

function App() {
  const [screen, setScreen] = useState<Screen>("login");
  const [username, setUsername] = useState<string | null>(null);

  useActivityWatcher(screen === "vault");

  useEffect(() => {
    const unlistenPromise = listen("vault-locked", () => {
      setScreen("login");
      setUsername(null);
    });
    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  if (screen === "login") {
    return (
      <Login
        onSuccess={(u) => {
          setUsername(u);
          setScreen("vault");
        }}
        onSwitchToRegister={() => setScreen("register")}
      />
    );
  }

  if (screen === "register") {
    return (
      <Register
        onSuccess={(u) => {
          setUsername(u);
          setScreen("vault");
        }}
        onSwitchToLogin={() => setScreen("login")}
      />
    );
  }

  return <Vault username={username!} />;
}

export default App;
