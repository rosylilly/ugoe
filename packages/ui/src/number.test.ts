import { describe, expect, it } from "vitest";
import { clamp, decimalsOf, formatEditable, parseEditable, roundTo, stepValue } from "./number";

describe("number", () => {
  it("clamps to the range", () => {
    expect(clamp(5, 0, 1)).toBe(1);
    expect(clamp(-5, 0, 1)).toBe(0);
    expect(clamp(0.5)).toBe(0.5);
  });

  it("counts the decimals of a step", () => {
    expect(decimalsOf(1)).toBe(0);
    expect(decimalsOf(0.1)).toBe(1);
    expect(decimalsOf(0.05)).toBe(2);
    expect(decimalsOf(1e-7)).toBe(7);
    expect(decimalsOf(0)).toBe(0);
  });

  it("rounds away floating point noise", () => {
    expect(roundTo(0.1 + 0.2, 2)).toBe(0.3);
  });

  it("formats without grouping or trailing zeros", () => {
    expect(formatEditable(1234.5, 2)).toBe("1234.5");
    expect(formatEditable(2, 2)).toBe("2");
    expect(formatEditable(100, 1)).toBe("100");
    expect(formatEditable(-0.0001, 2)).toBe("0");
    expect(formatEditable(Number.NaN, 2)).toBe("");
  });

  it("parses edited text", () => {
    expect(parseEditable(" 1.5 ")).toBe(1.5);
    expect(parseEditable("-.5")).toBe(-0.5);
    expect(parseEditable("１２．５")).toBe(12.5);
    expect(parseEditable("0,25")).toBe(0.25);
    expect(parseEditable("−3")).toBe(-3);
    expect(parseEditable("1e3")).toBe(1000);
    expect(parseEditable("")).toBeNull();
    expect(parseEditable("abc")).toBeNull();
    expect(parseEditable("1.2.3")).toBeNull();
  });

  it("steps within the range", () => {
    expect(stepValue(0.2, 1, { step: 0.1 })).toBe(0.3);
    expect(stepValue(0.95, 1, { min: 0, max: 1, step: 0.1 })).toBe(1);
    expect(stepValue(0, -10, { min: -5, step: 1 })).toBe(-5);
  });
});
