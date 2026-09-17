import { describe, it, expect } from "vitest";
import { repromptRequired, DEFAULT_REPROMPT_POLICY, type RepromptPolicy } from "./repromptPolicy";

const PER_CREDENTIAL: RepromptPolicy = { mode: "perCredential", graceSeconds: 30 };
const GLOBAL_30S: RepromptPolicy = { mode: "global", graceSeconds: 30 };

describe("repromptRequired", () => {
  it("perCredential mode, flag false -> reveals immediately (false)", () => {
    expect(repromptRequired(PER_CREDENTIAL, false, null, 0)).toBe(false);
  });

  it("perCredential mode, flag true -> prompts (true)", () => {
    expect(repromptRequired(PER_CREDENTIAL, true, null, 0)).toBe(true);
  });

  it("perCredential mode, flag true, lastSuccessAt 1ms ago -> still prompts (no grace window in this mode)", () => {
    const now = 100_000;
    expect(repromptRequired(PER_CREDENTIAL, true, now - 1, now)).toBe(true);
  });

  it("global mode, lastSuccessAt null, flag false -> prompts (true)", () => {
    expect(repromptRequired(GLOBAL_30S, false, null, 0)).toBe(true);
  });

  it("global mode, lastSuccessAt null, flag true -> prompts (true)", () => {
    expect(repromptRequired(GLOBAL_30S, true, null, 0)).toBe(true);
  });

  it("global mode, 10s elapsed inside a 30s window, flag false -> no prompt (false)", () => {
    const now = 100_000;
    expect(repromptRequired(GLOBAL_30S, false, now - 10_000, now)).toBe(false);
  });

  it("global mode, 10s elapsed inside a 30s window, flag true -> no prompt (false)", () => {
    const now = 100_000;
    expect(repromptRequired(GLOBAL_30S, true, now - 10_000, now)).toBe(false);
  });

  it("global mode, elapsed exactly equal to the window -> prompts again (true, boundary expires)", () => {
    const now = 130_000;
    expect(repromptRequired(GLOBAL_30S, false, now - 30_000, now)).toBe(true);
  });

  it("global mode, elapsed beyond the window -> prompts (true)", () => {
    const now = 200_000;
    expect(repromptRequired(GLOBAL_30S, false, now - 31_000, now)).toBe(true);
  });

  it("global mode, lastSuccessAt in the future (clock moved backwards) -> window treated as open (false)", () => {
    const now = 100_000;
    expect(repromptRequired(GLOBAL_30S, false, now + 5_000, now)).toBe(false);
  });

  it("DEFAULT_REPROMPT_POLICY matches today's exact behavior (perCredential, 30s)", () => {
    expect(DEFAULT_REPROMPT_POLICY).toEqual({ mode: "perCredential", graceSeconds: 30 });
  });
});
