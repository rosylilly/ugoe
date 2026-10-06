import { BrokenStorage, MemoryStorage, useLocalStorage } from "@ugoe/i18n/testing";
import { describe, expect, it } from "vitest";
import {
  clampSplit,
  DEFAULT_SPLIT,
  isNarrowWindow,
  LEFT_WIDTH,
  loadSize,
  loadSplit,
  MAX_SPLIT,
  MIN_SPLIT,
  NARROW_WINDOW_WIDTH,
  RIGHT_WIDTH,
  SPLIT_STEP,
  saveSize,
  saveSplit,
  sizeByKey,
  splitAt,
  splitByKey,
  TIMELINE_HEIGHT,
  timelineOpen,
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

describe("column widths and the timeline height", () => {
  it("moves a column separator with the left and right arrows", () => {
    expect(sizeByKey(LEFT_WIDTH, 240, "ArrowRight", "vertical")).toBe(240 + LEFT_WIDTH.step);
    expect(sizeByKey(LEFT_WIDTH, 240, "ArrowLeft", "vertical")).toBe(240 - LEFT_WIDTH.step);
    expect(sizeByKey(LEFT_WIDTH, 240, "ArrowDown", "vertical")).toBeNull();
    expect(sizeByKey(LEFT_WIDTH, 240, "Home", "vertical")).toBe(LEFT_WIDTH.min);
    expect(sizeByKey(LEFT_WIDTH, 240, "End", "vertical")).toBe(LEFT_WIDTH.max);
  });

  it("grows the area when its separator before it moves back", () => {
    // 右の列の左端の区切りを左へ動かすと、右の列は広くなる
    expect(sizeByKey(RIGHT_WIDTH, 320, "ArrowLeft", "vertical", true)).toBe(320 + RIGHT_WIDTH.step);
    // タイムラインの上端の区切りを上へ動かすと、タイムラインは高くなる
    expect(sizeByKey(TIMELINE_HEIGHT, 300, "ArrowUp", "horizontal", true)).toBe(
      300 + TIMELINE_HEIGHT.step,
    );
  });

  it("keeps the size in the range", () => {
    expect(sizeByKey(LEFT_WIDTH, LEFT_WIDTH.max, "ArrowRight", "vertical")).toBe(LEFT_WIDTH.max);
    expect(sizeByKey(LEFT_WIDTH, LEFT_WIDTH.min, "ArrowLeft", "vertical")).toBe(LEFT_WIDTH.min);
  });

  it("remembers sizes and goes back to the default when reset", () => {
    const storage = new MemoryStorage();
    useLocalStorage(storage);
    expect(loadSize(LEFT_WIDTH)).toBe(LEFT_WIDTH.default);
    expect(loadSize(TIMELINE_HEIGHT)).toBeNull();
    saveSize(LEFT_WIDTH, 300);
    saveSize(TIMELINE_HEIGHT, 400);
    expect(loadSize(LEFT_WIDTH)).toBe(300);
    expect(loadSize(TIMELINE_HEIGHT)).toBe(400);
    saveSize(TIMELINE_HEIGHT, null);
    expect(loadSize(TIMELINE_HEIGHT)).toBeNull();
    storage.setItem(LEFT_WIDTH.key, "9999");
    expect(loadSize(LEFT_WIDTH)).toBe(LEFT_WIDTH.max);
    storage.setItem(LEFT_WIDTH.key, "wide");
    expect(loadSize(LEFT_WIDTH)).toBe(LEFT_WIDTH.default);
  });
});

describe("narrow windows and empty projects", () => {
  it("treats the 200% zoom as narrow but not the lower limit", () => {
    expect(isNarrowWindow(720)).toBe(true);
    expect(isNarrowWindow(1024)).toBe(false);
    expect(isNarrowWindow(NARROW_WINDOW_WIDTH)).toBe(false);
    expect(isNarrowWindow(1440)).toBe(false);
    // 測れない幅 (0・NaN) は狭くないとする
    expect(isNarrowWindow(0)).toBe(false);
    expect(isNarrowWindow(Number.NaN)).toBe(false);
  });

  it("collapses the timeline of an empty project or in a narrow window until the user toggles it", () => {
    const context = { empty: true, narrow: false, touched: false };
    expect(timelineOpen(true, context)).toBe(false);
    expect(timelineOpen(true, { ...context, touched: true })).toBe(true);
    expect(timelineOpen(false, { ...context, touched: true })).toBe(false);
    expect(timelineOpen(true, { ...context, empty: false })).toBe(true);
    expect(timelineOpen(false, { ...context, empty: false })).toBe(false);
    expect(timelineOpen(true, { empty: false, narrow: true, touched: false })).toBe(false);
    expect(timelineOpen(true, { empty: false, narrow: true, touched: true })).toBe(true);
  });
});
