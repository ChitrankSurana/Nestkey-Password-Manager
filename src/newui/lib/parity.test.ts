import { describe, it, expect } from "vitest";
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, dirname, relative } from "node:path";
import { fileURLToPath } from "node:url";

// Standing invariant gate over the new UI's own sources (T-08-24/T-08-25/T-08-26). Reads every
// .ts/.tsx file under src/newui/ from disk with node:fs and asserts the phase's non-negotiable
// invariants - the clipboard seam, the reprompt gate, the lazy-decryption invariant, and the
// D-10 scope cuts - directly against the source text on every `npm test`, so a later edit that
// silently regresses one of them fails the suite instead of shipping unnoticed (checklist §4
// calls these out as "easy to drop during a rewrite"). Comments are stripped before matching so
// a comment can never satisfy or violate an assertion - several sibling files under src/newui/
// legitimately explain the D-10 scope cuts by name in prose (e.g. NewShell.tsx's own destinations
// comment), which would otherwise self-trip check 7 below.

const NEWUI_ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");

function walk(dir: string): string[] {
  const out: string[] = [];
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) {
      out.push(...walk(full));
    } else if (/\.(ts|tsx)$/.test(entry) && !/\.test\.tsx?$/.test(entry)) {
      // Excludes this file itself, plus every other *.test.ts(x) under src/newui/ - a test
      // asserting these invariants would otherwise trip its own checks by naming the very
      // concepts/APIs it asserts the absence of.
      out.push(full);
    }
  }
  return out;
}

// Strips `//` line comments and `/* ... */` / `{/* ... */}` block comments so a comment can
// never satisfy or violate an invariant. Safe for this codebase: no source file under
// src/newui/ has a string literal containing `//` (verified directly - URLs are only ever
// rendered from fetched data, never hardcoded as string literals) or an unterminated block
// comment, so this naive strip cannot mangle real code.
function stripComments(source: string): string {
  const withoutBlocks = source.replace(/\/\*[\s\S]*?\*\//g, "");
  return withoutBlocks
    .split("\n")
    .map((line) => {
      const idx = line.indexOf("//");
      return idx === -1 ? line : line.slice(0, idx);
    })
    .join("\n");
}

const FILES = walk(NEWUI_ROOT);
const SOURCES = FILES.map((path) => ({ path, code: stripComments(readFileSync(path, "utf-8")) }));

function relPath(path: string): string {
  return relative(process.cwd(), path).replace(/\\/g, "/");
}

describe("new UI source invariants (parity gate, T-08-24/25/26)", () => {
  it("never imports the Tauri core IPC entry point - every backend call goes through src/lib/api.ts (D-12)", () => {
    // The event listener module (@tauri-apps/api/event) is allowed and expected - NewApp and
    // useVault both need it - so this matches the core module specifically, not the whole scope.
    const offenders = SOURCES.filter((f) => /@tauri-apps\/api\/core\b/.test(f.code)).map((f) => relPath(f.path));
    expect(offenders, "files importing @tauri-apps/api/core directly instead of going through src/lib/api.ts").toEqual([]);
  });

  it("never writes the clipboard through the browser global - every copy routes through api.copyToClipboard", () => {
    const offenders = SOURCES.filter((f) => /navigator\.clipboard/.test(f.code)).map((f) => relPath(f.path));
    expect(offenders, "files calling navigator.clipboard directly instead of api.copyToClipboard").toEqual([]);
  });

  it("calls listCredentialsForSite in exactly one file, hooks/useVault.ts, exactly once (D-09 lazy-decryption invariant)", () => {
    const useVaultPath = join(NEWUI_ROOT, "hooks", "useVault.ts");
    const outsideOffenders = SOURCES.filter((f) => f.path !== useVaultPath && /listCredentialsForSite/.test(f.code)).map(
      (f) => relPath(f.path),
    );
    expect(outsideOffenders, "files calling listCredentialsForSite outside hooks/useVault.ts").toEqual([]);

    const useVaultFile = SOURCES.find((f) => f.path === useVaultPath);
    expect(useVaultFile, "hooks/useVault.ts must exist").toBeTruthy();
    const callCount = (useVaultFile!.code.match(/listCredentialsForSite/g) ?? []).length;
    expect(callCount, "listCredentialsForSite call count inside hooks/useVault.ts").toBe(1);
  });

  it("routes every hidden custom field through SecretField in components/DetailField.tsx", () => {
    const filePath = join(NEWUI_ROOT, "components", "DetailField.tsx");
    const file = SOURCES.find((f) => f.path === filePath);
    expect(file, "components/DetailField.tsx must exist").toBeTruthy();
    expect(file!.code, "components/DetailField.tsx must branch on the hidden field type").toMatch(/fieldType\s*===\s*["']hidden["']/);
    expect(file!.code, "components/DetailField.tsx must render SecretField for hidden fields").toMatch(/<SecretField/);
  });

  it("never constructs a mask by repeating a character - SecretField's fixed-width mask is the only mask (D-15)", () => {
    const offenders = SOURCES.filter((f) => /\.repeat\(/.test(f.code)).map((f) => relPath(f.path));
    expect(offenders, "files constructing a length-proportional mask via .repeat(...) instead of reusing SecretField").toEqual([]);
  });

  it("NewShell.tsx's destinations array has exactly the app's six real nav entries", () => {
    const filePath = join(NEWUI_ROOT, "NewShell.tsx");
    const file = SOURCES.find((f) => f.path === filePath);
    expect(file, "NewShell.tsx must exist").toBeTruthy();
    const keys = [...file!.code.matchAll(/key:\s*"([a-z-]+)"/g)].map((m) => m[1]).sort();
    expect(keys, "NewShell.tsx destinations array keys").toEqual(["data", "generator", "health", "settings", "trash", "vault"]);
  });

  it("never mentions any of the five D-10 nav concepts anywhere under src/newui/", () => {
    // The exact five concepts D-10 cuts (favorites star/nav entry, folders sidebar section,
    // item-type filters + the add-item type picker, sync button, the mockup's fabricated
    // History tab) - matched case-insensitively on word boundaries so this catches any spelling
    // (favorite/favourite) or restyled label, not just the literal Figma copy.
    const CUT_CONCEPTS: Array<{ name: string; pattern: RegExp }> = [
      { name: "favorites/favourites nav entry", pattern: /\bfavou?rites?\b/i },
      { name: "folders sidebar section", pattern: /\bfolders?\b/i },
      { name: "item-type filters / add-item type picker", pattern: /\bitem[\s-]?types?\b|\btypes\b/i },
      { name: "sync button", pattern: /\bsync\b/i },
      { name: "History tab", pattern: /\bhistory\b/i },
    ];
    const violations: string[] = [];
    for (const file of SOURCES) {
      for (const concept of CUT_CONCEPTS) {
        if (concept.pattern.test(file.code)) {
          violations.push(`${relPath(file.path)}: ${concept.name}`);
        }
      }
    }
    expect(violations, "D-10 scope cuts must not appear anywhere under src/newui/, in any state (not even as disabled stubs)").toEqual([]);
  });
});
