import { BrokenStorage, MemoryStorage, useLocalStorage } from "@ugoe/i18n/testing";
import { describe, expect, it } from "vitest";
import {
  clampSplit,
  DEFAULT_SPLIT,
  loadSplit,
  MAX_SPLIT,
  MIN_SPLIT,
  SPLIT_STEP,
  saveSplit,
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

describe("remembered split", () => {
  it("saves and loads the ratio under the same key and value as before", () => {
    const storage = new MemoryStorage();
    useLocalStorage(storage);
    expect(loadSplit()).toBe(DEFAULT_SPLIT);
    saveSplit(0.4);
    expect(storage.getItem("ugoe.editor.rightSplit")).toBe("0.4");
    expect(loadSplit()).toBe(0.4);
  });

  it("falls back to the default or the range for broken values", () => {
    const storage = new MemoryStorage();
    useLocalStorage(storage);
    storage.setItem("ugoe.editor.rightSplit", "abc");
    expect(loadSplit()).toBe(DEFAULT_SPLIT);
    storage.setItem("ugoe.editor.rightSplit", "2");
    expect(loadSplit()).toBe(MAX_SPLIT);
  });

  it("uses the default when localStorage cannot be used", () => {
    for (const storage of [new BrokenStorage(), undefined]) {
      useLocalStorage(storage);
      expect(loadSplit()).toBe(DEFAULT_SPLIT);
      expect(() => saveSplit(0.4)).not.toThrow();
    }
  });
});
