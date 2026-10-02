import { describe, expect, it } from "vitest";
import { languageOf, resolveLocale } from "./detect";
import { LANGUAGES, LOCALES, SOURCE_LOCALE } from "./languages";
import languages from "./languages.json";

describe("languageOf", () => {
  it("keeps only the language subtag", () => {
    expect(languageOf("ja-JP")).toBe("ja");
    expect(languageOf("zh_Hant_TW")).toBe("zh");
    expect(languageOf("EN")).toBe("en");
  });
});

describe("resolveLocale", () => {
  it("uses the first supported language", () => {
    expect(resolveLocale(["de-DE", "ja-JP", "en-US"])).toBe("ja");
    expect(resolveLocale(["en-GB", "ja"])).toBe("en");
  });

  it("falls back to English", () => {
    expect(resolveLocale(["de", "fr"])).toBe("en");
    expect(resolveLocale([])).toBe("en");
  });
});

describe("languages", () => {
  it("matches languages.json", () => {
    expect(SOURCE_LOCALE).toBe("en");
    expect(LOCALES).toEqual(languages.languages.map((l) => l.tag));
    expect(LANGUAGES.find((l) => l.tag === "ja")?.complete).toBe(true);
  });
});
