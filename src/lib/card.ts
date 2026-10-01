// N4/N5/N6: small pure helpers for the payment-card form. All local/offline - no network lookup
// for brand detection, matching the "no network calls beyond HIBP" constraint.

// N4: formats free-typed digits into "MM/YY" as the user types, inserting the "/" automatically
// rather than requiring them to type it. Caps at 4 digits (MMYY).
export function formatCardExpiry(input: string): string {
  const digits = input.replace(/\D/g, "").slice(0, 4);
  if (digits.length <= 2) return digits;
  return `${digits.slice(0, 2)}/${digits.slice(2)}`;
}

// N5: digits only, Luhn checksum, and a plausible length (13-19 covers every major card network -
// Amex is 15, most others 16, some debit/other networks run up to 19). Not a guarantee the number
// is real, just that it isn't an obvious typo like the UAT's 5-digit "41111".
export function isValidCardNumber(input: string): boolean {
  const digits = input.replace(/\D/g, "");
  if (digits.length < 13 || digits.length > 19) return false;
  let sum = 0;
  let double = false;
  for (let i = digits.length - 1; i >= 0; i--) {
    let d = digits.charCodeAt(i) - 48;
    if (double) {
      d *= 2;
      if (d > 9) d -= 9;
    }
    sum += d;
    double = !double;
  }
  return sum % 10 === 0;
}

export function lastFourDigits(input: string): string {
  const digits = input.replace(/\D/g, "");
  return digits.slice(-4);
}

// Ordered so a longer/more specific prefix (Discover's 65, Amex's 34/37) is checked before a
// shorter catch-all range would otherwise misfire - IIN ranges genuinely do overlap across
// networks, so order matters here, not just prefix length.
const IIN_RANGES: { brand: string; test: (digits: string) => boolean }[] = [
  { brand: "Visa", test: (d) => d.startsWith("4") },
  { brand: "American Express", test: (d) => /^3[47]/.test(d) },
  { brand: "Discover", test: (d) => /^6(?:011|5)/.test(d) },
  { brand: "Diners Club", test: (d) => /^3(?:0[0-5]|[68])/.test(d) },
  { brand: "JCB", test: (d) => /^35(?:2[89]|[3-8])/.test(d) },
  { brand: "Mastercard", test: (d) => /^5[1-5]/.test(d) || /^2(?:2[2-9]|[3-6]|7[01]|720)/.test(d) },
];

export function deriveCardBrand(input: string): string | null {
  const digits = input.replace(/\D/g, "");
  for (const { brand, test } of IIN_RANGES) {
    if (test(digits)) return brand;
  }
  return null;
}
