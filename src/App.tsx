import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

function App() {
  const [unlocked, setUnlocked] = useState<boolean | null>(null);

  useEffect(() => {
    invoke<boolean>("unlock_status").then(setUnlocked);
  }, []);

  return (
    <div className="dark:bg-slate-900 dark:text-slate-100 min-h-screen flex items-center justify-center">
      <p>Slice 0 smoke test — unlock_status: {String(unlocked)}</p>
    </div>
  );
}

export default App;
