import { describe, expect, it } from "vitest";
import {
  clampSplit,
  DEFAULT_SPLIT,
  MAX_SPLIT,
  MIN_SPLIT,
  SPLIT_STEP,
  splitAt,
  splitByKey,
} from "./layout";

describe("right column split", () => {
  it("clamps the ratio", () => {
    expect(clampSplit(0.5)).toBe(0.5);
    expect(clampSplit(0)).toBe(MIN_SPLIT);
    expect(clampSplit(1)).toBe(MAX_SPLIT);
    expect(clampSplit(Number.NaN)).toBe(DEFAULT_SPLIT);
  });

  it("follows the pointer", () => {
    expect(splitAt(300, 600)).toBe(0.5);
    expect(splitAt(10, 600)).toBe(MIN_SPLIT);
    expect(splitAt(10, 0)).toBe(DEFAULT_SPLIT);
  });

  it("moves with the keyboard", () => {
    expect(splitByKey(0.5, "ArrowUp")).toBeCloseTo(0.5 - SPLIT_STEP);
    expect(splitByKey(0.5, "ArrowDown")).toBeCloseTo(0.5 + SPLIT_STEP);
    expect(splitByKey(0.5, "Home")).toBe(MIN_SPLIT);
    expect(splitByKey(0.5, "End")).toBe(MAX_SPLIT);
    expect(splitByKey(0.5, "a")).toBeNull();
  });
});
