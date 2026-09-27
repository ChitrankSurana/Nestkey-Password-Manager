// J2: mirrors core/src/totp.rs's normalize_secret (strip whitespace/dashes/padding, uppercase)
// plus a base32-alphabet check, so the frontend can reject an obviously-bad TOTP secret before
// starting a multi-step add/edit sequence - previously a bad secret's rejection arrived only after
// the site (and, in edit mode, the credential) had already been created, leaving an orphaned item
// behind on retry. Charset-only, not a full decodability check (odd lengths etc. are rare and
// still caught server-side exactly as before) - this is a fast, cheap first filter, not a
// replacement for the backend's own validation.
const BASE32_CHARS = /^[A-Z2-7]+$/;

export function isPlausibleTotpSecret(input: string): boolean {
  const cleaned = input.replace(/[\s-=]/g, "").toUpperCase();
  return cleaned.length > 0 && BASE32_CHARS.test(cleaned);
}
