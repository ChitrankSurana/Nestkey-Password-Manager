import { describe, it, expect } from "vitest";
import { repromptRequired, DEFAULT_REPROMPT_POLICY, type RepromptPolicy } from "./repromptPolicy";

const ALWAYS: RepromptPolicy = { mode: "always", graceSeconds: 30 };
const GRACE_30S: RepromptPolicy = { mode: "grace", graceSeconds: 30 };

describe("repromptRequired", () => {
  it("exempt credential never prompts, regardless of mode or grace state", () => {
    expect(repromptRequired(ALWAYS, true, null, 0)).toBe(false);
    expect(repromptRequired(GRACE_30S, true, null, 0)).toBe(false);
    const now = 100_000;
    expect(repromptRequired(GRACE_30S, true, now - 1, now)).toBe(false);
  });

  it("always mode, not exempt -> always prompts, even with a fresh grace success", () => {
    const now = 100_000;
    expect(repromptRequired(ALWAYS, false, null, 0)).toBe(true);
    expect(repromptRequired(ALWAYS, false, now - 1, now)).toBe(true);
  });

  it("grace mode, lastSuccessAt null, not exempt -> prompts (true)", () => {
    expect(repromptRequired(GRACE_30S, false, null, 0)).toBe(true);
  });

  it("grace mode, 10s elapsed inside a 30s window, not exempt -> no prompt (false)", () => {
    const now = 100_000;
    expect(repromptRequired(GRACE_30S, false, now - 10_000, now)).toBe(false);
  });

  it("grace mode, elapsed exactly equal to the window -> prompts again (true, boundary expires)", () => {
    const now = 130_000;
    expect(repromptRequired(GRACE_30S, false, now - 30_000, now)).toBe(true);
  });

  it("grace mode, elapsed beyond the window -> prompts (true)", () => {
    const now = 200_000;
    expect(repromptRequired(GRACE_30S, false, now - 31_000, now)).toBe(true);
  });

  it("grace mode, lastSuccessAt in the future (clock moved backwards) -> window treated as open (false)", () => {
    const now = 100_000;
    expect(repromptRequired(GRACE_30S, false, now + 5_000, now)).toBe(false);
  });

  it("DEFAULT_REPROMPT_POLICY gates every reveal until real config arrives (grace, 30s)", () => {
    expect(DEFAULT_REPROMPT_POLICY).toEqual({ mode: "grace", graceSeconds: 30 });
  });
});
