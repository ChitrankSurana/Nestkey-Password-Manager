// Character classification for colorized password reveal (Bitwarden's bit-color-password idea).
// Native regex - no dependency. null means "inherit default text color" (letters).
export type CharClass = "digit" | "symbol" | null;

export function classifyChar(ch: string): CharClass {
  if (/[0-9]/.test(ch)) return "digit";
  if (/[a-zA-Z]/.test(ch)) return null;
  return "symbol";
}
