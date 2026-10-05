import { loadLocalePreference } from "@ugoe/i18n";
import { BrokenStorage, MemoryStorage, useLocalStorage } from "@ugoe/i18n/testing";
import { loadThemePreference } from "@ugoe/ui";
import { beforeEach, describe, expect, it } from "vitest";
import {
  DEFAULT_OUTPUT_SIZE,
  DEFAULT_TICK_RATE,
  forgetLegacySettings,
  LEGACY_STORAGE_KEYS,
  LOCALE_STORAGE_KEY,
  PLAYER_STORAGE_PREFIX,
  parseOutputSize,
  parseTickRate,
  presetOf,
  presetSize,
  THEME_STORAGE_KEY,
} from "./settings";

beforeEach(() => useLocalStorage(new MemoryStorage()));

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

describe("storage keys", () => {
  it("keeps the keys the studio has always used for the language and the theme", () => {
    // キーを変えると、覚えた設定が読めなくなる (T0925)。
    expect(LOCALE_STORAGE_KEY).toBe("ugoe-studio.locale");
    expect(THEME_STORAGE_KEY).toBe("ugoe-studio.theme");
  });

  it("reads the language and the theme the studio kept before the shared helpers (T0937)", () => {
    // 前の版が書いた値をそのまま置き、共有の読み書き (@ugoe/i18n・@ugoe/ui) で読めることを確かめる。
    localStorage.setItem("ugoe-studio.locale", "ja");
    localStorage.setItem("ugoe-studio.theme", "dark");
    expect(loadLocalePreference(LOCALE_STORAGE_KEY)).toBe("ja");
    expect(loadThemePreference(THEME_STORAGE_KEY)).toBe("dark");
  });
});

describe("tick rate", () => {
  it("reads 30 / 60 / 120 from the profile and defaults to 60", () => {
    expect(parseTickRate(120)).toBe(120);
    expect(parseTickRate(144)).toBe(DEFAULT_TICK_RATE);
    expect(parseTickRate(undefined)).toBe(DEFAULT_TICK_RATE);
  });
});

describe("legacy settings", () => {
  it("forgets the tick rate and the virtual camera kept before the profile had them", () => {
    localStorage.setItem("ugoe-studio.tick-rate", "120");
    localStorage.setItem("ugoe-studio.virtual-camera", "{}");
    localStorage.setItem(LOCALE_STORAGE_KEY, "ja");
    forgetLegacySettings();
    for (const key of LEGACY_STORAGE_KEYS) expect(localStorage.getItem(key)).toBeNull();
    expect(localStorage.getItem(LOCALE_STORAGE_KEY)).toBe("ja");
  });

  it("forgets what the output window's player kept under the viewer's prefix (T0930)", () => {
    localStorage.setItem("ugoe-viewer.idle-motion.models", "{}");
    localStorage.setItem("ugoe-viewer.lighting", "{}");
    localStorage.setItem(`${PLAYER_STORAGE_PREFIX}.lighting`, "{}");
    forgetLegacySettings();
    expect(localStorage.getItem("ugoe-viewer.idle-motion.models")).toBeNull();
    expect(localStorage.getItem("ugoe-viewer.lighting")).toBeNull();
    expect(localStorage.getItem(`${PLAYER_STORAGE_PREFIX}.lighting`)).toBe("{}");
  });

  it("does not throw when localStorage cannot be used", () => {
    useLocalStorage(new BrokenStorage());
    expect(() => forgetLegacySettings()).not.toThrow();
  });
});
