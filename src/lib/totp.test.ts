import { describe, it, expect } from "vitest";
import { isPlausibleTotpSecret } from "./totp";

describe("isPlausibleTotpSecret", () => {
  it("accepts a clean base32 secret", () => {
    expect(isPlausibleTotpSecret("JBSWY3DPEHPK3PXP")).toBe(true);
  });

  it("accepts lowercase, spaces, dashes and padding", () => {
    expect(isPlausibleTotpSecret(" jbsw-y3dp ehpk3pxp=\t\n")).toBe(true);
  });

  it("rejects an empty or whitespace-only value", () => {
    expect(isPlausibleTotpSecret("")).toBe(false);
    expect(isPlausibleTotpSecret("   ")).toBe(false);
  });

  it("rejects characters outside the base32 alphabet", () => {
    expect(isPlausibleTotpSecret("not valid base32 !!!")).toBe(false);
    // '1', '0', '8', '9' are not in the base32 alphabet (A-Z2-7).
    expect(isPlausibleTotpSecret("JBSWY3DP18890")).toBe(false);
  });
});
