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

// A union rather than `string` so the reprompt gate (plan 08-12) cannot typo a mode and silently
// fall through to the permissive branch.
export type RepromptMode = "perCredential" | "global";

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
  hasTotp: boolean;
}

export interface TotpCodeDto {
  code: string;
  secondsRemaining: number;
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
  biometricEnabled: boolean;
  sshAgentEnabled: boolean;
  uiVariant: string;
  repromptMode: RepromptMode;
  repromptGraceSeconds: number;
}

export interface SshKeyDto {
  id: number;
  label: string;
  publicKey: string;
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
  | { format: "lastPass" }
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
// marker (see the plan's Finding 2). Exported so screens can gate desktop-only features (e.g.
// biometric unlock, which has no web-mode route - Windows Hello can't run server-side).
export const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

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

// Web-only: posts `args` like `httpPost`, but the response body is the file itself, not JSON. Reads
// the filename off `content-disposition` (falling back to `fallbackName`), then drives the browser's
// native download via a programmatic anchor click - no server-side path is ever involved (SEC-01).
async function httpDownload(
  command: string,
  args: Record<string, unknown>,
  fallbackName: string,
): Promise<void> {
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
  const disposition = res.headers.get("content-disposition");
  const match = disposition ? /filename="?([^";]+)"?/.exec(disposition) : null;
  const filename = match?.[1] ?? fallbackName;

  const blob = await res.blob();
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  a.remove();
  URL.revokeObjectURL(url);
}

// Web-only: posts a raw file body (not JSON) to a session-scoped upload endpoint - the file's bytes
// land on the requesting browser's own session cache (SEC-01/D-01/D-03/D-04), reused by every
// downstream step. Follows the same CommandError-parsing branch `httpPost` uses.
async function httpUpload(command: string, file: File | Blob): Promise<void> {
  const res = await fetch(`/api/${command}`, {
    method: "POST",
    headers: { "Content-Type": "application/octet-stream", "X-PWMGR-CSRF": "1" },
    credentials: "same-origin",
    body: file,
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
}

export const api = {
  registerUser: (username: string, masterPassword: string) =>
    call<AuthSuccess>("register_user", { username, masterPassword }),
  login: (username: string, masterPassword: string) =>
    call<AuthSuccess>("login", { username, masterPassword }),
  hasPin: (username: string) => call<boolean>("has_pin", { username }),
  unlockWithPin: (username: string, pin: string) =>
    call<AuthSuccess>("unlock_with_pin", { username, pin }),
  setPin: (pin: string) => call<void>("set_pin", { pin }),
  clearPin: () => call<void>("clear_pin"),
  hasBiometric: (username: string) => call<boolean>("has_biometric", { username }),
  unlockWithBiometric: (username: string) =>
    call<AuthSuccess>("unlock_with_biometric", { username }),
  setBiometric: () => call<void>("set_biometric"),
  clearBiometric: () => call<void>("clear_biometric"),
  addSshKey: (label: string, privateKey: string) =>
    call<number>("add_ssh_key", { label, privateKey }),
  listSshKeys: () => call<SshKeyDto[]>("list_ssh_keys"),
  deleteSshKey: (id: number) => call<void>("delete_ssh_key", { id }),
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
  setTotpSecret: (credentialId: number, secret: string) =>
    call<void>("set_totp_secret", { credentialId, secret }),
  clearTotpSecret: (credentialId: number) => call<void>("clear_totp_secret", { credentialId }),
  totpCode: (credentialId: number) => call<TotpCodeDto | null>("totp_code", { credentialId }),
  listTrash: () => call<TrashedCredentialDto[]>("list_trash"),
  restoreCredential: (credentialId: number) => call<void>("restore_credential", { credentialId }),
  purgeTrash: () => call<number>("purge_trash"),
  getConfig: () => call<ConfigDto>("get_config"),
  updateAutoLockMinutes: (minutes: number) => call<void>("update_auto_lock_minutes", { minutes }),
  updateStrengthAlgorithm: (algorithm: string) =>
    call<void>("update_strength_algorithm", { algorithm }),
  updateNavCollapsed: (collapsed: boolean) => call<void>("update_nav_collapsed", { collapsed }),
  updateMobileNavStyle: (style: string) => call<void>("update_mobile_nav_style", { style }),
  updateUiVariant: (variant: string) => call<void>("update_ui_variant", { variant }),
  updateRepromptMode: (mode: RepromptMode) => call<void>("update_reprompt_mode", { mode }),
  updateRepromptGraceSeconds: (seconds: number) =>
    call<void>("update_reprompt_grace_seconds", { seconds }),
  updateTheme: (theme: string) => call<void>("update_theme", { theme }),
  updateAccentColor: (color: string) => call<void>("update_accent_color", { color }),
  generatePassword: (request: GenerateRequest) => call<string>("generate_password", { request }),
  analyzeStrength: (password: string) => call<StrengthDto>("analyze_strength", { password }),
  // No HTTP endpoint for clipboard - the browser has its own API. Desktop keeps the Tauri command.
  copyToClipboard: (text: string) =>
    isTauri ? invoke<void>("copy_to_clipboard", { text }) : webCopyToClipboard(text),
  createBackup: (destinationPath: string | null = null) =>
    call<string>("create_backup", { destinationPath }),
  restoreBackup: (path: string, confirmed: boolean) =>
    call<void>("restore_backup", { path, confirmed }),
  // Desktop and web legitimately diverge here (D-02): desktop still names a path from the native
  // save dialog; web mode never names a destination and instead downloads the bytes directly.
  exportCsv: (path: string | null, mode: ExportMode, confirmed: boolean) =>
    isTauri
      ? invoke<void>("export_csv", { path, mode, confirmed })
      : httpDownload(
          "export_csv",
          { mode, confirmed },
          mode.mode === "plaintext" ? "vault-export.csv" : "vault-export.pwenc",
        ),
  // Desktop and web legitimately diverge here (D-02): desktop still names a path from the native
  // open dialog; web mode has already uploaded the file bytes via `uploadCsv` and reads them back
  // from its own session cache, so no path is ever sent.
  uploadCsv: (file: File) => httpUpload("upload_csv", file),
  readCsvHeaders: (path: string | null) =>
    isTauri ? invoke<string[]>("read_csv_headers", { path }) : httpPost<string[]>("read_csv_headers", {}),
  importCsvPreview: (path: string | null, format: ImportFormat) =>
    isTauri
      ? invoke<ImportRecordDto[]>("import_csv_preview", { path, format })
      : httpPost<ImportRecordDto[]>("import_csv_preview", { format }),
  importCsvCommit: (records: ImportRecordDto[]) =>
    call<ImportSummaryDto>("import_csv_commit", { records }),
  healthReport: () => call<HealthReportDto>("health_report"),
  checkBreach: () => call<BreachReportDto>("check_breach"),
  updateHibpOptIn: (enabled: boolean) => call<void>("update_hibp_opt_in", { enabled }),
  updateBiometricEnabled: (enabled: boolean) => call<void>("update_biometric_enabled", { enabled }),
  updateSshAgentEnabled: (enabled: boolean) => call<void>("update_ssh_agent_enabled", { enabled }),
  updateOldPasswordDays: (days: number) => call<void>("update_old_password_days", { days }),
};
