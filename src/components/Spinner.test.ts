import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

// Source-invariant gate (no React renderer in this repo's vitest setup - environment: "node",
// src/**/*.test.ts only) proving the Spinner loading affordance stays wired at all four call
// sites: its own composition (Icon + animate-spin) plus the three slow-derivation paths (Login,
// Register, SecretField's reprompt confirm). Mirrors src/newui/lib/parity.test.ts's approach:
// read files from disk with node:fs, strip comments so a comment can never satisfy an assertion,
// then regex-assert against the stripped source.

const COMPONENTS_DIR = dirname(fileURLToPath(import.meta.url));
const SRC_DIR = join(COMPONENTS_DIR, "..");

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

function readStripped(relPath: string): string {
  return stripComments(readFileSync(join(SRC_DIR, relPath), "utf-8"));
}

describe("Spinner wiring invariants (G-08-7)", () => {
  it("Spinner.tsx renders <Icon and uses the animate-spin utility", () => {
    const code = readStripped("components/Spinner.tsx");
    expect(code, "src/components/Spinner.tsx must reference the animate-spin utility").toMatch(/animate-spin/);
    expect(code, "src/components/Spinner.tsx must render <Icon").toMatch(/<Icon/);
  });

  it("Login.tsx renders <Spinner at least twice (submit + Windows Hello)", () => {
    const code = readStripped("screens/Login.tsx");
    const count = (code.match(/<Spinner/g) ?? []).length;
    expect(count, "src/screens/Login.tsx must render <Spinner at least twice").toBeGreaterThanOrEqual(2);
  });

  it("Register.tsx renders <Spinner at least once", () => {
    const code = readStripped("screens/Register.tsx");
    expect(code, "src/screens/Register.tsx must render <Spinner").toMatch(/<Spinner/);
  });

  it("SecretField.tsx renders <Spinner at least once", () => {
    const code = readStripped("components/SecretField.tsx");
    expect(code, "src/components/SecretField.tsx must render <Spinner").toMatch(/<Spinner/);
  });
});
