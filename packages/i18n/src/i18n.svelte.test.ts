import { flushSync } from "svelte";
import { afterEach, describe, expect, it } from "vitest";
import {
  getLocale,
  getLocalePreference,
  hasTranslation,
  registerCatalogs,
  setLocale,
  setSystemLanguages,
  translator,
} from "./i18n.svelte";
import en from "./testdata/en.json";
import ja from "./testdata/ja.json";

declare module "./index" {
  interface CatalogTypes {
    test: typeof en;
  }
}

registerCatalogs("test", { en, ja });
const t = translator("test");

afterEach(() => {
  setSystemLanguages(["en-US"]);
  setLocale("system");
});

describe("translator", () => {
  it("interpolates named arguments", () => {
    setLocale("en");
    expect(t("greeting", { name: "ugoe" })).toBe("Hello, ugoe");
    setLocale("ja");
    expect(t("greeting", { name: "ugoe" })).toBe("こんにちは、ugoe");
  });

  it("picks the CLDR plural form", () => {
    setLocale("en");
    expect(t("files.count", { count: 1 })).toBe("1 file");
    expect(t("files.count", { count: 3 })).toBe("3 files");
    setLocale("ja");
    expect(t("files.count", { count: 1 })).toBe("1 個のファイル");
  });

  it("falls back to English for missing or empty translations", () => {
    setLocale("ja");
    expect(t("only.english")).toBe("Only in English");
    expect(hasTranslation("test", "only.english")).toBe(true);
    expect(hasTranslation("test", "nothing.here")).toBe(false);
  });

  it("formats numbers inside the catalog with the current locale", () => {
    setLocale("en");
    expect(t("price", { value: 1234.5 })).toBe("Total: 1,234.5");
  });

  it("rejects unknown keys at the type level", () => {
    setLocale("en");
    // @ts-expect-error: キーは原文のカタログにあるものだけ
    expect(t("no.such.key")).toBe("no.such.key");
    // @ts-expect-error: 入れ子の途中 (文字列でないもの) は引けない
    t("files");
  });

  it("does not escape HTML (Svelte escapes text)", () => {
    setLocale("en");
    expect(t("greeting", { name: "<b>" })).toBe("Hello, <b>");
  });
});

describe("locale", () => {
  it("follows the system languages by default", () => {
    setSystemLanguages(["fr-FR", "ja-JP", "en"]);
    expect(getLocalePreference()).toBe("system");
    expect(getLocale()).toBe("ja");
    setSystemLanguages(["fr-FR"]);
    expect(getLocale()).toBe("en");
  });

  it("can be overridden", () => {
    setSystemLanguages(["ja-JP"]);
    setLocale("en");
    expect(getLocale()).toBe("en");
    setLocale("system");
    expect(getLocale()).toBe("ja");
  });

  it("re-evaluates derived text when switched", () => {
    setLocale("en");
    const seen: string[] = [];
    const cleanup = $effect.root(() => {
      const text = $derived(t("greeting", { name: "A" }));
      $effect(() => {
        seen.push(text);
      });
    });
    flushSync();
    setLocale("ja");
    flushSync();
    cleanup();
    expect(seen).toEqual(["Hello, A", "こんにちは、A"]);
  });
});
