# Coding Conventions

**Analysis Date:** 2026-09-13

## Naming Patterns

**Files:**
- Components: PascalCase (e.g., `SecretField.tsx`, `PasswordGeneratorPanel.tsx`)
- Utilities/helpers: camelCase (e.g., `clipboard.ts`, `passwordColor.ts`, `strength.ts`, `toast.ts`)
- Hooks: camelCase prefixed with `use` (e.g., `useActivityWatcher.ts`)
- Test files: same name as source with `.test.ts` suffix (e.g., `clipboard.test.ts`)

**Functions:**
- camelCase for all functions (e.g., `simpleEntropyBits`, `entropyBand`, `classifyChar`, `webCopyToClipboard`)
- Handler callbacks use `handle*` prefix (e.g., `handleSubmit`, `handleBiometricUnlock`, `handlePickFile`)
- Request/action callbacks use `request*` or `run*` prefix (e.g., `requestAction`, `runAction`)
- Async functions use same naming convention as sync functions

**Variables:**
- camelCase for all variables and state (e.g., `username`, `masterPassword`, `reauthError`, `verifying`)
- React state: `useState` returns `[state, setState]` pairs using camelCase (e.g., `[revealed, setRevealed]`)
- Boolean state variables use verb prefixes: `is*`, `has*`, `can*`, `should*` (e.g., `pinAvailable`, `usePin`, `verifying`)
- Ref variables: descriptive names prefixed with context (e.g., `lastSent` for throttle tracking)

**Types:**
- Interfaces: PascalCase (e.g., `AuthSuccess`, `CommandError`, `SiteDto`, `CustomField`)
- Type unions: discriminated patterns with descriptive labels (e.g., `{ method: "random" }`, `{ format: "chromium" }`)
- Data Transfer Objects: PascalCase with `Dto` suffix (e.g., `SiteDto`, `CredentialDto`, `ConfigDto`, `HealthEntryDto`)
- Type guards: use `is*` prefix for type guard functions (e.g., `isCommandError`)
- Custom types for unions: PascalCase without `Dto` (e.g., `CustomFieldType`, `CharClass`, `ExportMode`, `ImportFormat`)

**Constants:**
- UPPER_SNAKE_CASE for module-level constants (e.g., `CLIPBOARD_CLEAR_MS = 30_000`, `THROTTLE_MS = 5000`)
- Constants may use numeric separators for readability (e.g., `30_000` instead of `30000`)
- CSS class maps and configuration objects: camelCase (e.g., `CLASS_COLOR`, `TARGET_FIELDS`)

## Code Style

**Formatting:**
- No external formatter configured (ESLint/Prettier absent)
- TypeScript compiler handles linting via `tsconfig.json` strict mode
- 2-space indentation (standard for JavaScript/TypeScript projects)
- Line breaks around JSX render blocks for readability
- Arrow functions preferred in React components for conciseness

**Linting:**
- TypeScript compiler enforces strict mode:
  - `strict: true` - enables all strict type-checking options
  - `noUnusedLocals: true` - flags unused variables
  - `noUnusedParameters: true` - flags unused function parameters
  - `noFallthroughCasesInSwitch: true` - requires explicit handling in switch cases
- No external ESLint configuration present
- Build process: `tsc && vite build` enforces TypeScript compilation

**JSX/TSX Style:**
- Functional components only (no class components)
- Props typed with inline object destructuring and TypeScript type annotations
- Event handlers inline via arrow functions
- Conditional rendering using ternary operators and `&&` for simple cases
- Tailwind CSS utility classes for styling (no CSS modules)

## Import Organization

**Order:**
1. External library imports (`react`, `@tauri-apps/api`, etc.)
2. Internal lib imports (`../lib/api`, `../lib/toast`, etc.)
3. Internal component imports (`../components/Icon`, etc.)
4. Internal hook imports (`../hooks/useActivityWatcher`)
5. Type-only imports appear inline with regular imports

**Path Aliases:**
- No path aliases configured (tsconfig.json lacks `paths`)
- Relative imports only (e.g., `../lib/api`, `../components/Tooltip`)
- Always use `./` or `../` for local imports

**Example from `SecretField.tsx`:**
```typescript
import { useState } from "react";
import { api } from "../lib/api";
import { toast } from "../lib/toast";
import { classifyChar } from "../lib/passwordColor";
import { Icon } from "./Icon";
```

## Error Handling

**Pattern: Generic error messages for security:**
- User-facing errors always use generic fallback message to prevent information leakage (TRD §3.2)
- Distinguish command errors from transport errors via `isCommandError` type guard:
  ```typescript
  try {
    const result = await api.login(username, password);
  } catch (err) {
    setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
  }
  ```

**Error type guard:**
- Defined in `src/lib/api.ts`: `isCommandError(e: unknown): e is CommandError`
- Checks for object with `code` and `message` properties
- Enables safe property access on errors

**Silent catches with console logging:**
- Infrastructure errors logged to console (e.g., IPC failures, transport breaks)
- Comment explains why error is logged vs. suppressed:
  ```typescript
  api.touch().catch((e) => {
    // `touch` is infallible on the Rust side, so a rejection here is always an IPC-transport
    // break or a panic, never a benign "session already expired" - log it rather than swallow.
    console.error("touch failed:", e);
  });
  ```

**Best-effort error handling:**
- Some operations swallow errors silently when degradation is acceptable:
  ```typescript
  setTimeout(async () => {
    try {
      if ((await clipboard.readText()) === text) {
        await clipboard.writeText("");
      }
    } catch {
      // Clipboard unreadable (e.g. permissions) - best-effort, matching the Rust side ignoring errors.
    }
  }, CLIPBOARD_CLEAR_MS);
  ```

## Logging

**Framework:** `console` methods only (no external logging library)

**Patterns:**
- `console.error()` for critical failures requiring visibility (IPC transport errors, panics)
- Errors logged with context comment explaining why it's not swallowed
- No `console.log()` in production code (avoid noise)
- Comments explain logging decisions for maintainability

## Comments

**When to Comment:**
- Complex logic or non-obvious algorithm choices
- Security-related decisions (e.g., error message generalization for oracle prevention)
- Workarounds and explanations of why something was done a certain way
- References to design documents or threat models (e.g., "TRD §3.2")
- Clarification of component behavior (e.g., why clipboard is injectable for testing)

**JSDoc/TSDoc:**
- Used for public functions and components with complex signatures
- Describes purpose, parameters, and behavior
- Example from `Tooltip.tsx`:
  ```typescript
  /**
   * Wraps a single focusable child (a <button> or <select>) and shows a small text tooltip on both
   * mouse hover AND keyboard focus. Deliberate replacement for `title=` across every icon/button —
   * `title` is hover-only in most webviews and doesn't reliably appear on keyboard Tab focus.
   *
   * Handlers live on the wrapper span, not cloned onto the child: this also makes hover work when
   * the child is disabled (a disabled form control's own mouse events are unreliable across
   * browsers, but hit-testing still resolves to this wrapper once pointer-events on the disabled
   * child are suppressed by the browser's UA stylesheet).
   */
  ```

**Inline comments:**
- Single-line comments explain "why" not "what"
- Example: "Defaults to dark/indigo (this app's original look) until config loads, matching the hardcoded class index.html used to set unconditionally - no unstyled flash either way."

## Function Design

**Size:**
- Prefer small, focused functions (most < 50 lines in `src/lib/`)
- Complex screens break logic into smaller helper functions (e.g., `Login.tsx` breaks auth into `handleSubmit`, `handleBiometricUnlock`, `checkQuickUnlockAvailable`)

**Parameters:**
- Props destructured inline for React components
- API methods: destructured object parameters with named fields (e.g., `{ username, masterPassword }`)
- Utility functions: typed parameters with explicit types

**Return Values:**
- Async functions return typed Promises (e.g., `Promise<AuthSuccess>`)
- Type-guarded functions use TypeScript predicates (e.g., `(e: unknown): e is CommandError`)
- React components return `ReactElement` or JSX

**Event Handlers:**
- Typed as inline arrow functions in component props
- Form handlers: `handleSubmit(e: FormEvent)` with `e.preventDefault()`
- State setters: inline lambdas with state update logic (e.g., `setRevealed((r) => !r)`)

## Module Design

**Exports:**
- Named exports for utilities and components (no default exports for consistency)
- Type exports via `export interface`, `export type`, `export const`
- Single export from `.tsx` components (the component function)

**Barrel Files:**
- Not used; imports reference files directly
- Example: `import { Icon } from "./Icon";` not from an `index.ts`

**Module-level singletons:**
- Toast system: module-level subscriber list instead of React context for multi-root support
  ```typescript
  // Module-level subscriber list, not a React context - this app has two separate roots (App.tsx,
  // ViewerApp.tsx) that would otherwise each need their own provider threaded through every screen.
  type Listener = (message: string) => void;
  const listeners = new Set<Listener>();
  ```

**Separation of concerns:**
- `lib/api.ts`: all backend communication and type definitions
- `lib/clipboard.ts`: browser clipboard integration with auto-clear
- `lib/strength.ts`: password strength calculation algorithms
- `lib/toast.ts`: event-based notification system
- `components/`: reusable UI primitives
- `hooks/`: custom React hooks for cross-component logic
- `screens/`: top-level page/view components

---

*Convention analysis: 2026-09-13*
