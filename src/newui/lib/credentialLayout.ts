import type { CredentialDto } from "../../lib/api";

// Pure credential-repeater and strength-meter logic for the new UI's detail column: no React,
// no `api` import (type-only import of CredentialDto is fine and keeps this file loadable under
// vitest's node environment, matching src/newui/lib/vaultSelectors.ts's convention).

/**
 * Implements D-08's zero/one/many rule for a site's credential list: nothing selected as primary
 * when the list is empty, the sole entry as primary when there is one, and the first entry as
 * primary with the remainder as `others` when there are more. Input order is preserved. Whether
 * `others` is expanded is local UI state in the caller and needs no persistence here.
 */
export function splitCredentials(credentials: CredentialDto[]): {
  primary: CredentialDto | null;
  others: CredentialDto[];
} {
  if (credentials.length === 0) {
    return { primary: null, others: [] };
  }
  const [primary, ...others] = credentials;
  return { primary, others };
}

/** Collapse-toggle copy for a site with `totalCount` credentials - the count minus the one primary block already expanded. */
export function otherLoginsLabel(totalCount: number): string {
  return `Other logins (${totalCount - 1})`;
}

/**
 * Maps the backend's 0..=4 zxcvbn score onto the five-segment strength meter by adding one and
 * clamping to the 1..5 range - an empty or very weak secret still shows one lit segment rather
 * than an all-dark bar indistinguishable from "not measured".
 */
export function strengthSegments(score: number): number {
  const clamped = Math.min(4, Math.max(0, score));
  return clamped + 1;
}

/**
 * Maps the backend's 0..=4 score onto the three colour bands the UI-SPEC Color section defines:
 * the two lowest scores are danger, the middle is warning, the two highest are strong. Stays free
 * of CSS/design-token names so it is testable under the node environment - callers translate
 * these three names into the danger/warning/green design tokens themselves.
 */
export function strengthTone(score: number): "danger" | "warning" | "strong" {
  const clamped = Math.min(4, Math.max(0, score));
  if (clamped <= 1) return "danger";
  if (clamped === 2) return "warning";
  return "strong";
}
