// WRY-based Tauri v2 webviews (WebView2/WKWebView/WebKitGTK) do not reliably support the browser
// global `window.confirm()` - it silently renders no visible dialog (G-08-5, 08-UAT.md). This
// module is the one seam every destructive-action confirm in this app routes through: on desktop
// it awaits the native `@tauri-apps/plugin-dialog` confirm(), which is already proven working for
// the file pickers elsewhere in this codebase; in web mode the browser global works fine (there's
// no webview quirk to work around there), so the seam branches on `isTauri` rather than replacing
// one working implementation with another that doesn't work in the other mode.
import { confirm as tauriConfirm } from "@tauri-apps/plugin-dialog";
import { isTauri } from "./api";

export async function confirmDialog(message: string, title?: string): Promise<boolean> {
  if (isTauri) {
    return tauriConfirm(message, { title: title ?? "Confirm", kind: "warning" });
  }
  return window.confirm(message);
}

// The UAT's expected copy for the no-credential case is byte-exact and must not drift:
// `Delete "{name}"? This cannot be undone.`. A `null` count means "caller does not know" and
// takes the same branch as zero - never claims a cascade the caller can't actually confirm.
export function siteDeleteMessage(name: string, credentialCount: number | null): string {
  if (credentialCount !== null && credentialCount > 0) {
    const plural = credentialCount === 1 ? "credential" : "credentials";
    return `Delete "${name}"? Its ${credentialCount} saved ${plural} will move to Trash. This cannot be undone.`;
  }
  return `Delete "${name}"? This cannot be undone.`;
}
