import { describe, it, expect } from "vitest";
import { randomMaskLength, maskString } from "./mask";

describe("randomMaskLength", () => {
  it("never returns the real secret's length", () => {
    for (const realLength of [1, 6, 8, 10, 12, 16, 20, 30]) {
      for (let i = 0; i < 50; i++) {
        expect(randomMaskLength(realLength)).not.toBe(realLength);
      }
    }
  });

  it("stays within the plausible-password range", () => {
    for (let i = 0; i < 50; i++) {
      const length = randomMaskLength(0);
      expect(length).toBeGreaterThanOrEqual(8);
      expect(length).toBeLessThanOrEqual(16);
    }
  });
});

describe("maskString", () => {
  it("renders exactly that many bullet characters", () => {
    expect(maskString(5)).toBe("•••••");
    expect([...maskString(10)]).toHaveLength(10);
  });
});
