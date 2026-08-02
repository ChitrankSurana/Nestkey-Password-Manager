import { invoke } from "@tauri-apps/api/core";
import { webCopyToClipboard } from "./clipboard";

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

export type CustomFieldType = "text" | "hidden" | "boolean";

export interface CustomField {
  label: string;
  value: string;
  fieldType: CustomFieldType;
}

export interface CredentialDto {
  id: number;
  siteId: number;
  username: string | null;
  secret: string;
  remarks: string | null;
  customFields: CustomField[];
  reprompt: boolean;
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
  accentColor: string;
  oldPasswordDays: number;
  hibpOptIn: boolean;
  navCollapsed: boolean;
  mobileNavStyle: string;
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

export type ImportFormat =
  | { format: "chromium" }
  | { format: "firefox" }
  | { format: "generic"; mapping: Record<string, string> };

export interface ImportRecordDto {
  siteName: string;
  url: string | null;
  notes: string | null;
  username: string | null;
  secret: string;
  remarks: string | null;
}

export interface ImportSummaryDto { imported: number; duplicates: number; }

export interface HealthEntryDto {
  credentialId: number;
  siteId: number;
  siteName: string;
  username: string | null;
}

export interface ReusedGroupDto { entries: HealthEntryDto[]; }

export interface HealthReportDto {
  weak: HealthEntryDto[];
  reused: ReusedGroupDto[];
  old: HealthEntryDto[];
  unsecured: HealthEntryDto[];
}

export interface BreachReportDto { breached: HealthEntryDto[]; }

// Tauri v2 always injects window.__TAURI_INTERNALS__ (that is how `invoke` reaches the backend);
// withGlobalTauri is off in this app, so window.__TAURI__ is NOT present - detect via the internals
// marker (see the plan's Finding 2).
const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function httpPost<T>(command: string, args: Record<string, unknown>): Promise<T> {
  const res = await fetch(`/api/${command}`, {
    method: "POST",
    headers: { "Content-Type": "application/json", "X-PWMGR-CSRF": "1" },
    credentials: "same-origin",
    body: JSON.stringify(args),
  });
  if (!res.ok) {
    let err: CommandError = { code: "internal_error", message: "Something went wrong. Please try again." };
    try {
      err = (await res.json()) as CommandError;
    } catch {
      // Non-JSON error body (e.g. a raw 404 from the static layer) - keep the generic fallback.
    }
    throw err;
  }
  const text = await res.text();
  return (text ? JSON.parse(text) : undefined) as T;
}

// Single dispatch point: Tauri IPC in the desktop app, same-origin HTTP in web mode. The command name
// and camelCase args are identical on both sides.
function call<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  return isTauri ? invoke<T>(command, args) : httpPost<T>(command, args);
}

export const api = {
  registerUser: (username: string, masterPassword: string) =>
    call<AuthSuccess>("register_user", { username, masterPassword }),
  login: (username: string, masterPassword: string) =>
    call<AuthSuccess>("login", { username, masterPassword }),
  lock: () => call<void>("lock"),
  touch: () => call<void>("touch"),
  verifyMasterPassword: (masterPassword: string) =>
    call<boolean>("verify_master_password", { masterPassword }),
  unlockStatus: () => call<boolean>("unlock_status"),
  listSites: () => call<SiteDto[]>("list_sites"),
  addSite: (name: string, url: string | null, notes: string | null) =>
    call<number>("add_site", { name, url, notes }),
  updateSite: (siteId: number, name: string, url: string | null, notes: string | null) =>
    call<void>("update_site", { siteId, name, url, notes }),
  softDeleteSite: (siteId: number) => call<void>("soft_delete_site", { siteId }),
  addCredential: (
    siteId: number,
    username: string | null,
    secret: string,
    remarks: string | null,
    customFields: CustomField[] = [],
    reprompt: boolean = false,
  ) => call<number>("add_credential", { siteId, username, secret, remarks, customFields, reprompt }),
  updateCredential: (
    credentialId: number,
    username: string | null,
    secret: string,
    remarks: string | null,
    customFields: CustomField[] = [],
    reprompt: boolean = false,
  ) => call<void>("update_credential", { credentialId, username, secret, remarks, customFields, reprompt }),
  listCredentialsForSite: (siteId: number) =>
    call<CredentialDto[]>("list_credentials_for_site", { siteId }),
  search: (query: string) => call<SearchResultDto[]>("search", { query }),
  softDeleteCredential: (credentialId: number) =>
    call<void>("soft_delete_credential", { credentialId }),
  listTrash: () => call<TrashedCredentialDto[]>("list_trash"),
  restoreCredential: (credentialId: number) => call<void>("restore_credential", { credentialId }),
  purgeTrash: () => call<number>("purge_trash"),
  getConfig: () => call<ConfigDto>("get_config"),
  updateAutoLockMinutes: (minutes: number) => call<void>("update_auto_lock_minutes", { minutes }),
  updateStrengthAlgorithm: (algorithm: string) =>
    call<void>("update_strength_algorithm", { algorithm }),
  updateNavCollapsed: (collapsed: boolean) => call<void>("update_nav_collapsed", { collapsed }),
  updateMobileNavStyle: (style: string) => call<void>("update_mobile_nav_style", { style }),
  updateTheme: (theme: string) => call<void>("update_theme", { theme }),
  updateAccentColor: (color: string) => call<void>("update_accent_color", { color }),
  generatePassword: (request: GenerateRequest) => call<string>("generate_password", { request }),
  analyzeStrength: (password: string) => call<StrengthDto>("analyze_strength", { password }),
  // No HTTP endpoint for clipboard - the browser has its own API. Desktop keeps the Tauri command.
  copyToClipboard: (text: string) =>
    isTauri ? invoke<void>("copy_to_clipboard", { text }) : webCopyToClipboard(text),
  createBackup: () => call<string>("create_backup"),
  restoreBackup: (path: string, confirmed: boolean) =>
    call<void>("restore_backup", { path, confirmed }),
  exportCsv: (path: string, mode: ExportMode, confirmed: boolean) =>
    call<void>("export_csv", { path, mode, confirmed }),
  readCsvHeaders: (path: string) => call<string[]>("read_csv_headers", { path }),
  importCsvPreview: (path: string, format: ImportFormat) =>
    call<ImportRecordDto[]>("import_csv_preview", { path, format }),
  importCsvCommit: (records: ImportRecordDto[]) =>
    call<ImportSummaryDto>("import_csv_commit", { records }),
  healthReport: () => call<HealthReportDto>("health_report"),
  checkBreach: () => call<BreachReportDto>("check_breach"),
  updateHibpOptIn: (enabled: boolean) => call<void>("update_hibp_opt_in", { enabled }),
  updateOldPasswordDays: (days: number) => call<void>("update_old_password_days", { days }),
};
