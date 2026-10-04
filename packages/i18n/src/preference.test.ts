import { afterEach, describe, expect, it } from "vitest";
import { getLocale, getLocalePreference, setLocale, setSystemLanguages } from "./i18n.svelte";
import {
  initLocale,
  loadLocalePreference,
  saveLocalePreference,
  setLocalePreference,
} from "./preference";

const KEY = "ugoe-test.locale";

/** 最小の Storage (getItem と setItem だけ使う)。 */
function memoryStorage(): Storage {
  const map = new Map<string, string>();
  return {
    getItem: (key: string) => map.get(key) ?? null,
    setItem: (key: string, value: string) => void map.set(key, value),
  } as Storage;
}

const brokenStorage = {
  getItem() {
    throw new Error("blocked");
  },
  setItem() {
    throw new Error("blocked");
  },
} as unknown as Storage;

function useLocalStorage(storage: Storage | undefined) {
  Object.defineProperty(globalThis, "localStorage", { value: storage, configurable: true });
}

afterEach(() => {
  useLocalStorage(undefined);
  setSystemLanguages(["en-US"]);
  setLocale("system");
});

describe("loadLocalePreference / saveLocalePreference", () => {
  it("follows the OS until a language is chosen, and remembers the choice under the key", () => {
    const storage = memoryStorage();
    expect(loadLocalePreference(KEY, storage)).toBe("system");
    saveLocalePreference(KEY, "ja", storage);
    expect(storage.getItem(KEY)).toBe("ja");
    expect(loadLocalePreference(KEY, storage)).toBe("ja");
    expect(loadLocalePreference("ugoe-other.locale", storage)).toBe("system");
    saveLocalePreference(KEY, "system", storage);
    expect(loadLocalePreference(KEY, storage)).toBe("system");
  });

  it("reads the values the apps saved before T0925", () => {
    // 前からの値 ("system"・言語のタグ) をそのまま読む (覚えた設定を引き継ぐ)。
    const storage = memoryStorage();
    for (const value of ["system", "en", "ja"]) {
      storage.setItem(KEY, value);
      expect(loadLocalePreference(KEY, storage)).toBe(value);
    }
  });

  it("ignores unknown values", () => {
    const storage = memoryStorage();
    for (const value of ["fr", "ja-JP", "", "JA"]) {
      storage.setItem(KEY, value);
      expect(loadLocalePreference(KEY, storage)).toBe("system");
    }
  });

  it("works without a usable localStorage", () => {
    expect(loadLocalePreference(KEY, brokenStorage)).toBe("system");
    expect(() => saveLocalePreference(KEY, "en", brokenStorage)).not.toThrow();
    useLocalStorage(undefined);
    expect(loadLocalePreference(KEY)).toBe("system");
    expect(() => saveLocalePreference(KEY, "en")).not.toThrow();
  });

  it("uses globalThis.localStorage when no storage is given", () => {
    const storage = memoryStorage();
    useLocalStorage(storage);
    saveLocalePreference(KEY, "ja");
    expect(storage.getItem(KEY)).toBe("ja");
    expect(loadLocalePreference(KEY)).toBe("ja");
  });
});

describe("initLocale / setLocalePreference", () => {
  it("applies the remembered preference and remembers changes under the same key", async () => {
    const storage = memoryStorage();
    storage.setItem(KEY, "ja");
    useLocalStorage(storage);
    await initLocale(KEY);
    expect(getLocalePreference()).toBe("ja");
    expect(getLocale()).toBe("ja");
    setLocalePreference("en");
    expect(getLocale()).toBe("en");
    expect(storage.getItem(KEY)).toBe("en");
    setLocalePreference("system");
    expect(storage.getItem(KEY)).toBe("system");
  });

  it("passes the OS language from the app", async () => {
    useLocalStorage(memoryStorage());
    await initLocale(KEY, async () => "ja-JP");
    expect(getLocalePreference()).toBe("system");
    expect(getLocale()).toBe("ja");
    await initLocale(KEY, async () => ["fr-FR", "en-GB"]);
    expect(getLocale()).toBe("en");
  });

  it("keeps navigator.languages when the OS language is unavailable", async () => {
    useLocalStorage(memoryStorage());
    setSystemLanguages(["ja-JP"]);
    await initLocale(KEY, async () => null);
    expect(getLocale()).toBe("ja");
    await expect(
      initLocale(KEY, async () => {
        throw new Error("not in Tauri");
      }),
    ).resolves.toBeUndefined();
    expect(getLocale()).toBe("ja");
    await initLocale(KEY, async () => []);
    expect(getLocale()).toBe("ja");
  });

  it("drops the OS language of an earlier call that finished late", async () => {
    useLocalStorage(memoryStorage());
    let finishFirst: (tag: string) => void = () => {};
    const first = initLocale(KEY, () => new Promise<string>((resolve) => (finishFirst = resolve)));
    await initLocale(KEY, async () => "en-US");
    finishFirst("ja-JP");
    await first;
    expect(getLocale()).toBe("en");
  });
});
