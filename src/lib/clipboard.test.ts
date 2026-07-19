import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { webCopyToClipboard, CLIPBOARD_CLEAR_MS } from "./clipboard";

describe("webCopyToClipboard", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

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

  it("does not clear if the clipboard changed before 30s elapsed", async () => {
    let value = "";
    const clipboard = {
      writeText: vi.fn(async (t: string) => { value = t; }),
      readText: vi.fn(async () => value),
    };

    await webCopyToClipboard("s3cret", clipboard);
    value = "user-copied-something-else";

    await vi.advanceTimersByTimeAsync(CLIPBOARD_CLEAR_MS);
    expect(value).toBe("user-copied-something-else");
  });
});
