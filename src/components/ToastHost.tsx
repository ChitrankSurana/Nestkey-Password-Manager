import { useEffect, useState } from "react";
import { subscribeToast } from "../lib/toast";

const DISMISS_MS = 2000;

// Mounted once per app root (App.tsx, ViewerApp.tsx) - renders whatever toast() last fired.
export function ToastHost() {
  const [message, setMessage] = useState<string | null>(null);

  useEffect(() => {
    return subscribeToast((m) => setMessage(m));
  }, []);

  useEffect(() => {
    if (message === null) return;
    const id = setTimeout(() => setMessage(null), DISMISS_MS);
    return () => clearTimeout(id);
  }, [message]);

  if (message === null) return null;

  return (
    <div
      role="status"
      className="fixed bottom-4 right-4 z-[70] rounded bg-slate-800 px-3 py-2 text-sm text-slate-100 shadow-lg dark:border dark:border-slate-600"
    >
      {message}
    </div>
  );
}
