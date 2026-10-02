import { beforeEach, describe, expect, it } from "vitest";
import {
  DEFAULT_OUTPUT_SIZE,
  DEFAULT_TICK_RATE,
  LOCALE_STORAGE_KEY,
  loadLocalePreference,
  loadOutputSize,
  loadTickRate,
  OUTPUT_SIZE_STORAGE_KEY,
  presetOf,
  presetSize,
  saveLocalePreference,
  saveOutputSize,
  saveTickRate,
  TICK_RATE_STORAGE_KEY,
} from "./settings";

class MemoryStorage {
  #items = new Map<string, string>();
  getItem(key: string) {
    return this.#items.get(key) ?? null;
  }
  setItem(key: string, value: string) {
    this.#items.set(key, value);
  }
}

beforeEach(() => {
  Object.defineProperty(globalThis, "localStorage", {
    value: new MemoryStorage(),
    configurable: true,
  });
});

describe("output size presets", () => {
  it("maps presets to sizes and back", () => {
    expect(presetSize("1080x1920")).toEqual([1080, 1920]);
    expect(presetSize("free")).toBeNull();
    expect(presetOf(1280, 720)).toBe("1280x720");
    expect(presetOf(1000, 700)).toBe("free");
  });

  it("remembers the preset and ignores unknown values", () => {
    expect(loadOutputSize()).toBe(DEFAULT_OUTPUT_SIZE);
    saveOutputSize("1080x1080");
    expect(loadOutputSize()).toBe("1080x1080");
    localStorage.setItem(OUTPUT_SIZE_STORAGE_KEY, "4x4");
    expect(loadOutputSize()).toBe(DEFAULT_OUTPUT_SIZE);
  });
});

describe("locale preference", () => {
  it("defaults to the OS and remembers the choice", () => {
    expect(loadLocalePreference()).toBe("system");
    saveLocalePreference("ja");
    expect(loadLocalePreference()).toBe("ja");
    localStorage.setItem(LOCALE_STORAGE_KEY, "xx");
    expect(loadLocalePreference()).toBe("system");
  });
});

describe("tick rate", () => {
  it("defaults to 60 and remembers 30 / 60 / 120 only", () => {
    expect(loadTickRate()).toBe(DEFAULT_TICK_RATE);
    saveTickRate(120);
    expect(loadTickRate()).toBe(120);
    localStorage.setItem(TICK_RATE_STORAGE_KEY, "144");
    expect(loadTickRate()).toBe(DEFAULT_TICK_RATE);
  });
});
