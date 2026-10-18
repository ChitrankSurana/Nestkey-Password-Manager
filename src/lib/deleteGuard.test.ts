import { describe, expect, it, vi } from "vitest";

vi.mock("./confirm", () => ({ confirmDialog: vi.fn() }));
vi.mock("../components/MasterPasswordPrompt", () => ({ MasterPasswordPrompt: () => null }));

import { deleteNeedsMasterPassword } from "./deleteGuard";
import type { ConfigDto } from "./api";

const base = {
  deleteRepromptTrashItem: true,
  deleteRepromptCredential: false,
  deleteRepromptEmptyTrash: true,
  deleteRepromptFolder: false,
} as ConfigDto;

describe("deleteNeedsMasterPassword", () => {
  it("reads the toggle matching each delete action", () => {
    expect(deleteNeedsMasterPassword(base, "trashItem")).toBe(true);
    expect(deleteNeedsMasterPassword(base, "deleteCredential")).toBe(false);
    expect(deleteNeedsMasterPassword(base, "emptyTrash")).toBe(true);
    expect(deleteNeedsMasterPassword(base, "deleteFolder")).toBe(false);
  });
});
