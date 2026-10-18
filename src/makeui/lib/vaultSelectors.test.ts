import { describe, expect, it } from "vitest";
import { addDefaultsForSection } from "./vaultSelectors";

describe("addDefaultsForSection", () => {
  it("opens the matching tab for type sections", () => {
    expect(addDefaultsForSection("logins").itemType).toBe("login");
    expect(addDefaultsForSection("notes").itemType).toBe("note");
    expect(addDefaultsForSection("cards").itemType).toBe("card");
    expect(addDefaultsForSection("identities").itemType).toBe("identity");
  });

  it("falls back to Login for All, Favorites and folders, preselecting favorite/folder", () => {
    expect(addDefaultsForSection("all")).toEqual({ itemType: "login", folderId: null, favorite: false });
    expect(addDefaultsForSection("favorites")).toEqual({ itemType: "login", folderId: null, favorite: true });
    expect(addDefaultsForSection("folder:7")).toEqual({ itemType: "login", folderId: 7, favorite: false });
  });
});
