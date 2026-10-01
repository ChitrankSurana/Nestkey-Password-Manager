// N18: collapsed/expanded state for the sidebar's Vault and Folders sections - a per-device UI
// convenience, not vault data, so localStorage is the right place (same reasoning as
// lastUsername.ts). Remembered across reloads/relaunches instead of always resetting to the same
// default every time.
const VAULT_SECTION_KEY = "pwmgr:sidebarVaultOpen";
const FOLDERS_SECTION_KEY = "pwmgr:sidebarFoldersOpen";

function readBool(key: string, fallback: boolean): boolean {
  try {
    const raw = window.localStorage.getItem(key);
    return raw === null ? fallback : raw === "true";
  } catch {
    return fallback;
  }
}

function writeBool(key: string, value: boolean): void {
  try {
    window.localStorage.setItem(key, value ? "true" : "false");
  } catch {
    // Private window, blocked storage, etc. - fine, this is a convenience, not a requirement.
  }
}

export function getVaultSectionOpen(): boolean {
  return readBool(VAULT_SECTION_KEY, true);
}

export function setVaultSectionOpen(open: boolean): void {
  writeBool(VAULT_SECTION_KEY, open);
}

export function getFoldersSectionOpen(): boolean {
  return readBool(FOLDERS_SECTION_KEY, false);
}

export function setFoldersSectionOpen(open: boolean): void {
  writeBool(FOLDERS_SECTION_KEY, open);
}
