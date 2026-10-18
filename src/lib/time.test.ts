import { describe, expect, it } from "vitest";
import { formatUtcLocal } from "./time";

describe("formatUtcLocal", () => {
  it("treats the SQLite string as UTC and renders it in local time", () => {
    const expected = new Date(Date.UTC(2026, 9, 18, 14, 2, 11)).toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });
    expect(formatUtcLocal("2026-10-18 14:02:11")).toBe(expected);
  });

  it("returns unparseable input unchanged", () => {
    expect(formatUtcLocal("not a date")).toBe("not a date");
  });
});
