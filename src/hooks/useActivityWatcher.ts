import { useEffect, useRef } from "react";
import { api } from "../lib/api";

const THROTTLE_MS = 5000;

/**
 * Calls the `touch` IPC command (resetting the backend idle timer) on user activity, throttled so
 * a burst of mousemove/keydown events doesn't flood IPC. Only active while `enabled` (i.e. while
 * logged in) - no point touching a session that isn't unlocked.
 */
export function useActivityWatcher(enabled: boolean) {
  const lastSent = useRef(0);

  useEffect(() => {
    if (!enabled) return;

    const onActivity = () => {
      const now = Date.now();
      if (now - lastSent.current < THROTTLE_MS) return;
      lastSent.current = now;
      api.touch().catch((e) => {
        // `touch` is infallible on the Rust side, so a rejection here is always an IPC-transport
        // break or a panic, never a benign "session already expired" - log it rather than swallow.
        console.error("touch failed:", e);
      });
    };

    window.addEventListener("mousemove", onActivity);
    window.addEventListener("keydown", onActivity);
    window.addEventListener("mousedown", onActivity);

    return () => {
      window.removeEventListener("mousemove", onActivity);
      window.removeEventListener("keydown", onActivity);
      window.removeEventListener("mousedown", onActivity);
    };
  }, [enabled]);
}
