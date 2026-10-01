import { describe, it, expect } from "vitest";
import { splitCredentials, otherLoginsLabel, strengthSegments, strengthTone } from "./credentialLayout";
import type { CredentialDto } from "../../lib/api";

function credential(id: number): CredentialDto {
  return {
    id,
    siteId: 1,
    username: `user${id}`,
    secret: "secret",
    remarks: null,
    customFields: [],
    repromptExempt: false,
    hasTotp: false,
  };
}

describe("splitCredentials", () => {
  it("returns { primary: null, others: [] } for an empty list", () => {
    expect(splitCredentials([])).toEqual({ primary: null, others: [] });
  });

  it("returns the sole entry as primary with no others", () => {
    const a = credential(1);
    expect(splitCredentials([a])).toEqual({ primary: a, others: [] });
  });

  it("returns the first entry as primary and the rest as others, preserving input order", () => {
    const a = credential(1);
    const b = credential(2);
    const c = credential(3);
    expect(splitCredentials([a, b, c])).toEqual({ primary: a, others: [b, c] });
  });
});

describe("otherLoginsLabel", () => {
  it("returns the total count minus the one already expanded", () => {
    expect(otherLoginsLabel(3)).toBe("Other logins (2)");
    expect(otherLoginsLabel(2)).toBe("Other logins (1)");
  });
});

describe("strengthSegments", () => {
  it("maps the backend's 0..4 score onto 1..5 lit segments", () => {
    expect(strengthSegments(0)).toBe(1);
    expect(strengthSegments(4)).toBe(5);
  });

  it("clamps out-of-range scores into the 1..5 range", () => {
    expect(strengthSegments(-3)).toBe(1);
    expect(strengthSegments(99)).toBe(5);
  });
});

describe("strengthTone", () => {
  it("maps the two lowest scores to danger", () => {
    expect(strengthTone(0)).toBe("danger");
    expect(strengthTone(1)).toBe("danger");
  });

  it("maps the middle score to warning", () => {
    expect(strengthTone(2)).toBe("warning");
  });

  it("maps the two highest scores to strong", () => {
    expect(strengthTone(3)).toBe("strong");
    expect(strengthTone(4)).toBe("strong");
  });
});
