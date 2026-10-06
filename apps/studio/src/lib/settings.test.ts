import { describe, expect, it } from "vitest";
import {
  DEFAULT_OUTPUT_SIZE,
  DEFAULT_TICK_RATE,
  parseOutputSize,
  parseTickRate,
  presetOf,
  presetSize,
} from "./settings";

describe("output size presets", () => {
  it("maps presets to sizes and back", () => {
    expect(presetSize("1080x1920")).toEqual([1080, 1920]);
    expect(presetSize("free")).toBeNull();
    expect(presetOf(1280, 720)).toBe("1280x720");
    expect(presetOf(1000, 700)).toBe("free");
  });

  it("reads the preset from the profile and ignores unknown values", () => {
    expect(parseOutputSize("1080x1080")).toBe("1080x1080");
    expect(parseOutputSize("4x4")).toBe(DEFAULT_OUTPUT_SIZE);
    expect(parseOutputSize(undefined)).toBe(DEFAULT_OUTPUT_SIZE);
  });
});

describe("tick rate", () => {
  it("reads 30 / 60 / 120 from the profile and defaults to 60", () => {
    expect(parseTickRate(120)).toBe(120);
    expect(parseTickRate(144)).toBe(DEFAULT_TICK_RATE);
    expect(parseTickRate(undefined)).toBe(DEFAULT_TICK_RATE);
  });
});
