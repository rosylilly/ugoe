import { describe, expect, it } from "vitest";
import { LOCALE_STORAGE_KEY, loadLocalePreference, saveLocalePreference } from "./settings";

/** 最小の Storage (getItem と setItem だけ使う)。 */
function memoryStorage(): Storage {
  const map = new Map<string, string>();
  return {
    getItem: (key: string) => map.get(key) ?? null,
    setItem: (key: string, value: string) => void map.set(key, value),
  } as Storage;
}

describe("locale preference", () => {
  it("follows the OS until a language is chosen, and remembers the choice", () => {
    const storage = memoryStorage();
    expect(loadLocalePreference(storage)).toBe("system");
    saveLocalePreference("ja", storage);
    expect(storage.getItem(LOCALE_STORAGE_KEY)).toBe("ja");
    expect(loadLocalePreference(storage)).toBe("ja");
    saveLocalePreference("system", storage);
    expect(loadLocalePreference(storage)).toBe("system");
  });

  it("ignores unknown values and unusable storage", () => {
    const storage = memoryStorage();
    storage.setItem(LOCALE_STORAGE_KEY, "fr");
    expect(loadLocalePreference(storage)).toBe("system");
    const broken = {
      getItem() {
        throw new Error("blocked");
      },
      setItem() {
        throw new Error("blocked");
      },
    } as unknown as Storage;
    expect(loadLocalePreference(broken)).toBe("system");
    expect(() => saveLocalePreference("en", broken)).not.toThrow();
  });
});
