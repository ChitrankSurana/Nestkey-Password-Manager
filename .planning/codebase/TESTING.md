# Testing Patterns

**Analysis Date:** 2026-09-13

## Test Framework

**Runner:**
- Vitest 4.1.10
- Config: `vitest.config.ts`
- Environment: `node` (for unit testing utilities, not browser DOM)

**Assertion Library:**
- Vitest's built-in expect API (same as Jest)

**Run Commands:**
```bash
npm test                  # Run all tests (vitest run)
npm run dev              # Development mode with Vite (tests run separately)
```

**Test configuration** (`vitest.config.ts`):
```typescript
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "node",
    include: ["src/**/*.test.ts"],
  },
});
```

## Test File Organization

**Location:**
- Co-located with source files in same directory
- Pattern: `[filename].test.ts` alongside `[filename].ts`

**Current test coverage:**
- `src/lib/clipboard.test.ts` - tests for `clipboard.ts`
- `src/lib/passwordColor.test.ts` - tests for `passwordColor.ts`

**Naming:**
- Test files: `*.test.ts` (not `.spec.ts`)
- Match the source filename exactly

## Test Structure

**Suite Organization:**
```typescript
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { webCopyToClipboard, CLIPBOARD_CLEAR_MS } from "./clipboard";

describe("webCopyToClipboard", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it("writes immediately and clears after 30s when the clipboard is unchanged", async () => {
    // test code
  });

  it("does not clear if the clipboard changed before 30s elapsed", async () => {
    // test code
  });
});
```

**Patterns:**
- Single `describe` block per function/component
- Descriptive `it` statements (full sentence describing behavior)
- Setup with `beforeEach` / `afterEach` for isolated test state
- No nesting of `describe` blocks (flat structure)

## Mocking

**Framework:** Vitest's `vi` object

**Function mocking pattern:**
```typescript
const clipboard = {
  writeText: vi.fn(async (t: string) => { value = t; }),
  readText: vi.fn(async () => value),
};
```

**Timer mocking pattern:**
```typescript
beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());

// In test:
await vi.advanceTimersByTimeAsync(CLIPBOARD_CLEAR_MS);
```

**What to Mock:**
- External dependencies passed as constructor arguments (e.g., `clipboard` parameter in `webCopyToClipboard`)
- Time-based operations (setTimeout, setInterval)
- Browser APIs when testing in isolation

**What NOT to Mock:**
- Internal utility functions (import and call directly)
- Core algorithm logic (test the real implementation)
- Module-level constants

**Dependency injection for testability:**
- Functions accept injectable dependencies as parameters with sensible defaults
- Example: `webCopyToClipboard(text: string, clipboard = navigator.clipboard)`
- Allows tests to pass mock implementations without modifying the function

## Fixtures and Factories

**Test Data:**
- Inline primitive values used in test assertions
- No fixture or factory files currently used

**Example from `passwordColor.test.ts`:**
```typescript
it("buckets digits, letters, and symbols correctly", () => {
  expect(classifyChar("5")).toBe("digit");
  expect(classifyChar("a")).toBe(null);
  expect(classifyChar("Z")).toBe(null);
  expect(classifyChar("!")).toBe("symbol");
  expect(classifyChar("_")).toBe("symbol");
});
```

**Location:**
- Test data defined inline in test functions
- No separate fixtures directory

## Coverage

**Requirements:** Not enforced (no coverage configuration)

**View Coverage:**
- No coverage setup configured in `vitest.config.ts`
- To add coverage: extend config with `coverage: { provider: "v8" }` and run `vitest --coverage`

**Observed test gaps:**
- No tests for React components (`SecretField.tsx`, `Tooltip.tsx`, etc.)
- No tests for API communication (`api.ts`)
- No tests for hooks (`useActivityWatcher.ts`)
- No integration tests
- Only 2 test files for 40+ source files

## Test Types

**Unit Tests:**
- Scope: Pure functions in `src/lib/` (clipboard, passwordColor)
- Approach: Direct function calls with mocked external dependencies
- No DOM or React rendering
- Example: `classifyChar("5") → "digit"`

**Integration Tests:**
- Not present in codebase
- Would test API communication + UI interactions

**E2E Tests:**
- Not present in codebase
- Desktop app would require Tauri test harness or manual testing
- Manual test checklists documented in `docs/` (e.g., `FIGMA_UI_VS_CURRENT_FEATURES.md`)

## Common Patterns

**Async Testing:**
```typescript
async function checkQuickUnlockAvailable() {
  // ...
  setPinAvailable(await api.hasPin(username));
}

// Test pattern:
it("writes immediately and clears after 30s when the clipboard is unchanged", async () => {
  let value = "";
  const clipboard = {
    writeText: vi.fn(async (t: string) => { value = t; }),
    readText: vi.fn(async () => value),
  };

  await webCopyToClipboard("s3cret", clipboard);
  expect(value).toBe("s3cret");

  await vi.advanceTimersByTimeAsync(CLIPBOARD_CLEAR_MS);
  expect(value).toBe("");
});
```

**State verification in time-based tests:**
- Advance fake timers with `vi.advanceTimersByTimeAsync()`
- Verify state changes after timer fires
- Especially useful for debounced/throttled operations

## Gaps and Recommendations

**Missing test coverage for:**
- React component rendering and interactions (`SecretField.tsx`, `Tooltip.tsx`, etc.)
- API communication layer (`api.ts`)
- Custom hooks (`useActivityWatcher.ts`)
- Error handling paths
- Form submission workflows

**To add component tests:**
```typescript
import { render, screen, fireEvent } from "@testing-library/react";
import { SecretField } from "./SecretField";

describe("SecretField", () => {
  it("reveals password on button click", () => {
    render(<SecretField secret="test123" />);
    // assertions...
  });
});
```

**To add component testing framework:**
1. Install: `npm install -D @testing-library/react`
2. Update vitest config to use `environment: "jsdom"`
3. Create tests alongside component files

**Recommended minimal coverage targets:**
- All utility functions in `src/lib/` (currently 1/6 covered)
- All hooks in `src/hooks/`
- Critical component interactions (copy, reveal, form submission)

---

*Testing analysis: 2026-09-13*
