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

export const api = {
  registerUser: (username: string, masterPassword: string) =>
    invoke<AuthSuccess>("register_user", { username, masterPassword }),
  login: (username: string, masterPassword: string) =>
    invoke<AuthSuccess>("login", { username, masterPassword }),
  lock: () => invoke<void>("lock"),
  touch: () => invoke<void>("touch"),
  unlockStatus: () => invoke<boolean>("unlock_status"),
};
