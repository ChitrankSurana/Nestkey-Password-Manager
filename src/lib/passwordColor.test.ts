import { describe, it, expect } from "vitest";
import { classifyChar } from "./passwordColor";

describe("classifyChar", () => {
  it("buckets digits, letters, and symbols correctly", () => {
    expect(classifyChar("5")).toBe("digit");
    expect(classifyChar("a")).toBe(null);
    expect(classifyChar("Z")).toBe(null);
    expect(classifyChar("!")).toBe("symbol");
    expect(classifyChar("_")).toBe("symbol");
  });
});
