import React from "react";
import ReactDOM from "react-dom/client";
import NewApp from "./newui/NewApp";

// Runtime-failure escape hatch (checkpoint decision, Task 2 of 08-01-PLAN.md: "asset-guard,
// extended"). The Rust-side asset-guard in src-tauri/src/lib.rs only covers a missing bundled
// asset; it cannot help once the New UI's own JS is loaded and throws at startup or during
// render. This listener is deliberately vanilla DOM, not part of <NewApp/>'s own React tree,
// state, or routing - it must keep working even if NewApp.tsx itself is what's broken. It is
// registered BEFORE the app attempts to mount, so it also catches errors thrown while importing
// NewApp.tsx (module-evaluation time), not just errors during React's render/commit phases.
//
// Deliberate exception to D-12 ("no direct IPC import under src/newui/"): this file lives
// alongside, not inside, src/newui/, and the whole point of this fallback is that it must not
// depend on src/lib/api.ts or any other New-UI code path that might itself be the thing that's
// broken.
let fallbackShown = false;

// CR-02: these listeners exist to catch failures during import/first-mount only (see comment
// above). Once the initial mount succeeds, `bootstrapped` flips to true and later, unrelated,
// recoverable runtime errors anywhere in the running app (e.g. a transient clipboard-copy
// rejection) no longer tear down the whole vault UI.
let bootstrapped = false;

function showClassicFallback(message: string) {
  if (fallbackShown) return;
  fallbackShown = true;

  const root = document.getElementById("root");
  if (!root) return;
  root.innerHTML = "";

  const wrap = document.createElement("div");
  wrap.style.cssText =
    "display:flex;flex-direction:column;gap:12px;align-items:center;justify-content:center;" +
    "min-height:100vh;padding:24px;text-align:center;background:#0f172a;color:#f1f5f9;" +
    "font-family:system-ui,sans-serif;";

  const heading = document.createElement("h1");
  heading.textContent = "Something went wrong loading the new interface.";
  heading.style.cssText = "font-size:1.25rem;font-weight:600;margin:0;";

  const body = document.createElement("p");
  body.textContent = message;
  body.style.cssText = "font-size:0.875rem;opacity:0.8;margin:0;max-width:32rem;";

  const button = document.createElement("button");
  button.textContent = "Switch to Classic UI";
  button.style.cssText =
    "padding:8px 16px;border-radius:6px;background:#6366f1;color:white;font-weight:600;" +
    "border:none;cursor:pointer;";
  button.onclick = async () => {
    button.disabled = true;
    button.textContent = "Switching...";
    // Best-effort: persist the recovery choice so the desktop app's next launch also opens
    // Classic (src-tauri/src/lib.rs reads ui_variant at boot). Navigating to index.html below
    // unblocks the CURRENT session regardless of whether this call succeeds - e.g. under the web
    // portal, whose /api/update_ui_variant route ships in plan 08-02, not this one.
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("update_ui_variant", { variant: "classic" });
    } catch {
      // Ignored - the navigation below still recovers the current session either way.
    }
    window.location.href = "/index.html";
  };

  wrap.appendChild(heading);
  wrap.appendChild(body);
  wrap.appendChild(button);
  root.appendChild(wrap);
}

window.addEventListener("error", (e) => {
  if (bootstrapped) return;
  showClassicFallback(e.message || "An unexpected error occurred.");
});
window.addEventListener("unhandledrejection", (e) => {
  if (bootstrapped) return;
  const reason = e.reason;
  showClassicFallback(reason instanceof Error ? reason.message : "An unexpected error occurred.");
});

try {
  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <NewApp />
    </React.StrictMode>,
  );
  bootstrapped = true;
} catch (e) {
  showClassicFallback(e instanceof Error ? e.message : "An unexpected error occurred.");
}
