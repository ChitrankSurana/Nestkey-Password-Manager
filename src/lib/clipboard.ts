export const CLIPBOARD_CLEAR_MS = 30_000;

/**
 * Web-mode clipboard copy with a 30s auto-clear, mirroring (not sharing) the Rust
 * `copy_to_clipboard`'s `CLIPBOARD_CLEAR_SECS` behavior. `clipboard` is injectable for testing.
 * Check-before-clear: only wipes if the clipboard still holds exactly what we wrote, so a value the
 * user copied in the meantime is left alone.
 */
export async function webCopyToClipboard(
  text: string,
  clipboard: Pick<Clipboard, "writeText" | "readText"> = navigator.clipboard,
  // N9: usernames/URLs/non-secret custom fields were auto-clearing (and claiming the 30s notice)
  // exactly like passwords - that behavior should only apply to secrets. Defaults to true so every
  // existing call site (all of which copy a secret) keeps its current behavior unchanged; only
  // CopyButton (non-secret copies) passes false.
  autoClear = true,
): Promise<void> {
  await clipboard.writeText(text);
  if (!autoClear) return;
  setTimeout(async () => {
    try {
      if ((await clipboard.readText()) === text) {
        await clipboard.writeText("");
      }
    } catch {
      // Clipboard unreadable (e.g. permissions) - best-effort, matching the Rust side ignoring errors.
    }
  }, CLIPBOARD_CLEAR_MS);
}
