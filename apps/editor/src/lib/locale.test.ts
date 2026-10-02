import { getLocalePreference } from "@ugoe/i18n";
import { beforeEach, describe, expect, it } from "vitest";
import { changeLocale, LOCALE_KEY, loadLocalePreference, saveLocalePreference } from "./locale";

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

describe("locale preference", () => {
  it("defaults to the OS language", () => {
    expect(loadLocalePreference()).toBe("system");
  });

  it("remembers a chosen language", () => {
    saveLocalePreference("ja");
    expect(loadLocalePreference()).toBe("ja");
    saveLocalePreference("system");
    expect(loadLocalePreference()).toBe("system");
  });

  it("ignores an unknown saved value", () => {
    localStorage.setItem(LOCALE_KEY, "fr");
    expect(loadLocalePreference()).toBe("system");
  });

  it("switches and remembers", () => {
    changeLocale("ja");
    expect(getLocalePreference()).toBe("ja");
    expect(loadLocalePreference()).toBe("ja");
    changeLocale("en");
  });
});
