export function simpleEntropyBits(pw: string): number {
  if (pw.length === 0) return 0;
  let pool = 0;
  if (/[a-z]/.test(pw)) pool += 26;
  if (/[A-Z]/.test(pw)) pool += 26;
  if (/[0-9]/.test(pw)) pool += 10;
  if (/[^a-zA-Z0-9]/.test(pw)) pool += 32; // symbols/other
  return pool === 0 ? 0 : Math.round(pw.length * Math.log2(pool));
}

// Map bits -> a 0..4 band + label so the simple and zxcvbn meters render identically.
export function entropyBand(bits: number): { score: number; label: string } {
  const score = bits < 28 ? 0 : bits < 36 ? 1 : bits < 60 ? 2 : bits < 128 ? 3 : 4;
  return { score, label: ["Very weak", "Weak", "Fair", "Strong", "Very strong"][score] };
}
