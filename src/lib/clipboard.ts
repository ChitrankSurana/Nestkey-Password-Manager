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
): Promise<void> {
  await clipboard.writeText(text);
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
