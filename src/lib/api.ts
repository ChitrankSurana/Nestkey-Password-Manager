import { invoke } from "@tauri-apps/api/core";

export interface AuthSuccess {
  username: string;
}

export interface CommandError {
  code: string;
  message: string;
}

export function isCommandError(e: unknown): e is CommandError {
  return typeof e === "object" && e !== null && "code" in e && "message" in e;
}

export interface SiteDto {
  id: number;
  name: string;
  url: string | null;
  notes: string | null;
}

export interface CredentialDto {
  id: number;
  siteId: number;
  username: string | null;
  secret: string;
  remarks: string | null;
}

export interface SearchResultDto {
  siteId: number;
  siteName: string;
  credentialId: number;
  username: string | null;
}

export interface TrashedCredentialDto {
  credentialId: number;
  siteId: number;
  siteName: string;
  username: string | null;
  deletedAt: string;
}

export interface ConfigDto {
  theme: string;
  autoLockMinutes: number;
  strengthAlgorithm: string;
}

export type GenerateRequest =
  | { method: "random"; length: number; uppercase: boolean; lowercase: boolean; digits: boolean; symbols: boolean }
  | { method: "memorable"; words: number; separator: string }
  | { method: "pattern"; template: string }
  | { method: "pronounceable"; length: number };

export interface StrengthDto { score: number; guessesLog10: number; label: string; }

export type ExportMode =
  | { mode: "thisDevice" }
  | { mode: "withPassword"; password: string }
  | { mode: "plaintext" };

export const api = {
  registerUser: (username: string, masterPassword: string) =>
    invoke<AuthSuccess>("register_user", { username, masterPassword }),
  login: (username: string, masterPassword: string) =>
    invoke<AuthSuccess>("login", { username, masterPassword }),
  lock: () => invoke<void>("lock"),
  touch: () => invoke<void>("touch"),
  unlockStatus: () => invoke<boolean>("unlock_status"),
  listSites: () => invoke<SiteDto[]>("list_sites"),
  addSite: (name: string, url: string | null, notes: string | null) =>
    invoke<number>("add_site", { name, url, notes }),
  addCredential: (siteId: number, username: string | null, secret: string, remarks: string | null) =>
    invoke<number>("add_credential", { siteId, username, secret, remarks }),
  updateCredential: (credentialId: number, username: string | null, secret: string, remarks: string | null) =>
    invoke<void>("update_credential", { credentialId, username, secret, remarks }),
  listCredentialsForSite: (siteId: number) =>
    invoke<CredentialDto[]>("list_credentials_for_site", { siteId }),
  search: (query: string) => invoke<SearchResultDto[]>("search", { query }),
  softDeleteCredential: (credentialId: number) =>
    invoke<void>("soft_delete_credential", { credentialId }),
  listTrash: () => invoke<TrashedCredentialDto[]>("list_trash"),
  restoreCredential: (credentialId: number) => invoke<void>("restore_credential", { credentialId }),
  purgeTrash: () => invoke<number>("purge_trash"),
  getConfig: () => invoke<ConfigDto>("get_config"),
  updateAutoLockMinutes: (minutes: number) => invoke<void>("update_auto_lock_minutes", { minutes }),
  updateStrengthAlgorithm: (algorithm: string) =>
    invoke<void>("update_strength_algorithm", { algorithm }),
  generatePassword: (request: GenerateRequest) => invoke<string>("generate_password", { request }),
  analyzeStrength: (password: string) => invoke<StrengthDto>("analyze_strength", { password }),
  copyToClipboard: (text: string) => invoke<void>("copy_to_clipboard", { text }),
  createBackup: () => invoke<string>("create_backup"),
  restoreBackup: (path: string, confirmed: boolean) =>
    invoke<void>("restore_backup", { path, confirmed }),
  exportCsv: (path: string, mode: ExportMode, confirmed: boolean) =>
    invoke<void>("export_csv", { path, mode, confirmed }),
};
