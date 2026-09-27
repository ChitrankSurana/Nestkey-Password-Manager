import { describe, it, expect } from "vitest";
import { formatCardExpiry, isValidCardNumber, lastFourDigits, deriveCardBrand } from "./card";

describe("formatCardExpiry", () => {
  it("inserts the slash after 2 digits", () => {
    expect(formatCardExpiry("1225")).toBe("12/25");
    expect(formatCardExpiry("12")).toBe("12");
    expect(formatCardExpiry("1")).toBe("1");
  });

  it("strips non-digits and caps at MMYY", () => {
    expect(formatCardExpiry("12/25/99")).toBe("12/25");
    expect(formatCardExpiry("ab12cd25ef99")).toBe("12/25");
  });
});

describe("isValidCardNumber", () => {
  it("accepts known-valid test numbers (Luhn-valid, plausible length)", () => {
    expect(isValidCardNumber("4111111111111111")).toBe(true); // Visa test number
    expect(isValidCardNumber("378282246310005")).toBe(true); // Amex test number
  });

  it("rejects a short number like the UAT's 5-digit '41111'", () => {
    expect(isValidCardNumber("41111")).toBe(false);
  });

  it("rejects a Luhn-invalid number of plausible length", () => {
    expect(isValidCardNumber("4111111111111112")).toBe(false);
  });
});

describe("lastFourDigits", () => {
  it("returns the last 4 digits, ignoring formatting", () => {
    expect(lastFourDigits("4111 1111 1111 1111")).toBe("1111");
  });
});

describe("deriveCardBrand", () => {
  it("identifies Visa, Amex, Mastercard, Discover by IIN prefix", () => {
    expect(deriveCardBrand("4111111111111111")).toBe("Visa");
    expect(deriveCardBrand("378282246310005")).toBe("American Express");
    expect(deriveCardBrand("5500000000000004")).toBe("Mastercard");
    expect(deriveCardBrand("6011000000000004")).toBe("Discover");
  });

  it("returns null for an unrecognized prefix", () => {
    expect(deriveCardBrand("9999999999999999")).toBeNull();
  });
});
