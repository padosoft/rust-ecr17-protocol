import { describe, expect, it } from "vitest";
import { isFailure, maskPan, safeDetail } from "./results";

describe("maskPan", () => {
  it("keeps only the last 4 digits", () => {
    expect(maskPan("4111111111111111")).toBe("************1111");
    expect(maskPan("123")).toBe("****");
  });
});

describe("isFailure", () => {
  it("treats void/no-status as success", () => {
    expect(isFailure(undefined)).toBe(false);
    expect(isFailure(null)).toBe(false);
    expect(isFailure({})).toBe(false);
  });
  it("uses outcome for transaction results", () => {
    expect(isFailure({ outcome: "ok" })).toBe(false);
    expect(isFailure({ outcome: "ko" })).toBe(true);
    expect(isFailure({ outcome: "cardNotPresent" })).toBe(true);
  });
  it("uses responseId for VAS results", () => {
    expect(isFailure({ responseId: "0" })).toBe(false);
    expect(isFailure({ responseId: "1" })).toBe(true);
  });
});

describe("safeDetail", () => {
  it("masks a nested PAN and preserves other fields", () => {
    const json = safeDetail({ outcome: "ok", pan: "4111111111111111", authCode: "AB" });
    const parsed = JSON.parse(json);
    expect(parsed.pan).toBe("************1111");
    expect(parsed.authCode).toBe("AB");
    expect(parsed.outcome).toBe("ok");
  });
  it("returns 'ok' for undefined", () => {
    expect(safeDetail(undefined)).toBe("ok");
  });
});
