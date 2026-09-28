import { describe, expect, it } from "vitest";
import { duplicateKeyError, formatValue, parseValue } from "./parameterValue";

const range = { min: -30, max: 30 };

describe("formatValue", () => {
  it("rounds to three decimals and drops trailing zeros", () => {
    expect(formatValue(0.5)).toBe("0.5");
    expect(formatValue(12.34567)).toBe("12.346");
    expect(formatValue(30)).toBe("30");
    expect(formatValue(-0.0001)).toBe("0");
  });
});

describe("parseValue", () => {
  it("accepts numbers within the range, including the ends", () => {
    expect(parseValue(" 12.5 ", range)).toEqual({ ok: true, value: 12.5 });
    expect(parseValue("-30", range)).toEqual({ ok: true, value: -30 });
    expect(parseValue("30", range)).toEqual({ ok: true, value: 30 });
  });

  it("rejects text that is not a number", () => {
    for (const text of ["", "abc", "1e999", "NaN"]) {
      expect(parseValue(text, range).ok).toBe(false);
    }
  });

  it("rejects values outside the range", () => {
    const result = parseValue("31", range);
    expect(result).toEqual({ ok: false, error: "31 is outside the range -30 to 30" });
    expect(parseValue("-30.5", range).ok).toBe(false);
  });
});

describe("duplicateKeyError", () => {
  it("reports an existing key within the tolerance", () => {
    expect(duplicateKeyError(0.5, [0, 0.5, 1], { min: 0, max: 1 })).toBe("0.5 is already a key");
    expect(duplicateKeyError(0.5 + 1e-8, [0.5], { min: 0, max: 1 })).toBe("0.5 is already a key");
  });

  it("allows a new value", () => {
    expect(duplicateKeyError(15, [-30, 0, 30], range)).toBeNull();
    expect(duplicateKeyError(0, [], range)).toBeNull();
  });
});
