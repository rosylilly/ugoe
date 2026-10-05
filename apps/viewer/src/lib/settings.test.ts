import { loadLocalePreference } from "@ugoe/i18n";
import { MemoryStorage } from "@ugoe/i18n/testing";
import { loadThemePreference } from "@ugoe/ui";
import { describe, expect, it } from "vitest";
import { LOCALE_STORAGE_KEY, THEME_STORAGE_KEY } from "./settings";

describe("storage keys", () => {
  it("keeps the keys the viewer has always used for the language and the theme", () => {
    // キーを変えると、覚えた設定が読めなくなる (T0925)。index.html の <head> のスクリプトもテーマのキーを読む。
    expect(LOCALE_STORAGE_KEY).toBe("ugoe-viewer.locale");
    expect(THEME_STORAGE_KEY).toBe("ugoe-viewer.theme");
  });

  it("reads the language and the theme the viewer kept before the shared helpers (T0937)", () => {
    // 前の版が書いた値をそのまま置き、共有の読み書き (@ugoe/i18n・@ugoe/ui) で読めることを確かめる。
    const storage = new MemoryStorage();
    storage.setItem("ugoe-viewer.locale", "en");
    storage.setItem("ugoe-viewer.theme", "light");
    expect(loadLocalePreference(LOCALE_STORAGE_KEY, storage)).toBe("en");
    expect(loadThemePreference(THEME_STORAGE_KEY, storage)).toBe("light");
  });
});
