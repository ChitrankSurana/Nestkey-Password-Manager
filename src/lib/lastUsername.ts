// B5: "remember the last-used username" is a per-device UI convenience, not vault data - a
// username is not a secret (it's shown in plaintext on the Login screen's own input), so
// localStorage is the right place for it, unlike anything that touches the vault or master
// password. Off by default: an opt-in Settings toggle, not an ambient behavior.
const REMEMBER_KEY = "pwmgr:rememberLastUsername";
const USERNAME_KEY = "pwmgr:lastUsername";

function readLocalStorage(key: string): string | null {
  try {
    return window.localStorage.getItem(key);
  } catch {
    // Private window, blocked storage, etc. - fail closed to "not remembered", never throw.
    return null;
  }
}

function writeLocalStorage(key: string, value: string | null): void {
  try {
    if (value === null) window.localStorage.removeItem(key);
    else window.localStorage.setItem(key, value);
  } catch {
    // Nothing to do if storage is unavailable - this is a convenience feature, not a requirement.
  }
}

export function getRememberLastUsername(): boolean {
  return readLocalStorage(REMEMBER_KEY) === "true";
}

export function setRememberLastUsername(remember: boolean): void {
  writeLocalStorage(REMEMBER_KEY, remember ? "true" : "false");
  if (!remember) writeLocalStorage(USERNAME_KEY, null);
}

export function getLastUsername(): string {
  return readLocalStorage(USERNAME_KEY) ?? "";
}

export function recordLastUsername(username: string): void {
  if (getRememberLastUsername()) writeLocalStorage(USERNAME_KEY, username);
}
