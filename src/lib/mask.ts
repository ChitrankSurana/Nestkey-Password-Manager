// N8: a masked secret previously always rendered exactly 8 bullet characters - fixed-length
// masking still leaks information indirectly, since two credentials with different real password
// lengths are indistinguishable at 8 bullets, but any password that happens to BE 8 characters
// leaks its length by omission (an observer who knows the mask is fixed-length learns nothing new
// about an 8-char password, but *would* about anything else). A random length in a plausible
// range, deliberately never equal to the real length, removes that correlation without the mask
// ever being suspiciously short/long compared to typical passwords.
const MIN_MASK_LENGTH = 8;
const MAX_MASK_LENGTH = 16;

export function randomMaskLength(realLength: number): number {
  let length: number;
  do {
    length = MIN_MASK_LENGTH + Math.floor(Math.random() * (MAX_MASK_LENGTH - MIN_MASK_LENGTH + 1));
  } while (length === realLength);
  return length;
}

export function maskString(length: number): string {
  return "•".repeat(length);
}
