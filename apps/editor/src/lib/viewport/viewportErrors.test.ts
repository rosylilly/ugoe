// ビューポートの読み込みと作成の失敗の訳 (viewportErrors.ts。読み込みは @ugoe/player の describeLoadError。T1611・T0941) が、
// ugoe-wasm の `LoadError`・`CreateError` のすべての `code` (crates/ugoe-wasm/codes.json の `load_error`・`create_error`) と
// 補足の値の組を、エディタの言語のすべてで文にするか。
import { LANGUAGES, SOURCE_LOCALE, setLocale } from "@ugoe/i18n";
import { afterEach, describe, expect, it } from "vitest";
import codes from "../../../../../crates/ugoe-wasm/codes.json";
import { t } from "../i18n";
import { createErrorText, loadErrorText } from "./viewportErrors";

/** 補足の値のうち文字列のもの (ほかは数)。`image`・`limit` は文を選ぶので、取りうる値をすべて試す。 */
const STRING_FIELDS: Record<string, string[]> = {
  image: ["texture", "normal_map"],
  limit: ["chunk_decompressed", "total_decompressed"],
  chunk: ["TEXR"],
  features: ["x.a, x.b"],
};

/** `code` の `LoadError` を、補足の値の取りうる組ごとに作る。 */
function samples(code: string, fields: string[], name = "LoadError"): Error[] {
  let combos: Record<string, string | number>[] = [{}];
  for (const field of fields) {
    const values = STRING_FIELDS[field] ?? [1234];
    combos = combos.flatMap((combo) => values.map((value) => ({ ...combo, [field]: value })));
  }
  return combos.map((combo) => Object.assign(new Error("wasm text"), { name, code, ...combo }));
}

afterEach(() => setLocale("en"));

describe("load error catalog", () => {
  const entries = Object.entries(codes.load_error);

  it("lists the load error codes of ugoe-wasm", () => {
    expect(entries.length).toBeGreaterThan(5);
  });

  it.each(LANGUAGES.map((l) => l.tag))("translates every load error code in %s", (language) => {
    for (const [code, fields] of entries) {
      for (const error of samples(code, fields)) {
        setLocale(SOURCE_LOCALE);
        const source = loadErrorText(error);
        setLocale(language);
        const text = loadErrorText(error);
        expect(text, code).not.toBe("wasm text");
        expect(text, code).not.toMatch(/^loadError\./);
        expect(text, code).not.toContain("{{");
        // 訳が欠けると原文 (英語) に戻るので、原文の言語のほかは原文と違う文になる。
        if (language !== SOURCE_LOCALE) expect(text, code).not.toBe(source);
      }
    }
  });
});

describe("create error catalog", () => {
  const entries = Object.entries(codes.create_error);

  it.each(LANGUAGES.map((l) => l.tag))("translates every create error code in %s", (language) => {
    for (const [code, fields] of entries) {
      for (const error of samples(code, fields, "CreateError")) {
        setLocale(SOURCE_LOCALE);
        const source = createErrorText(error);
        setLocale(language);
        const text = createErrorText(error);
        expect(text, code).not.toBe("wasm text");
        expect(text, code).not.toMatch(/^viewport\./);
        if (language !== SOURCE_LOCALE) expect(text, code).not.toBe(source);
      }
    }
  });
});

describe("viewport error texts", () => {
  const coded = (name: string, code: string) =>
    Object.assign(new Error("wasm text"), { name, code });

  it("uses the editor's text when the GPU stopped while loading", () => {
    setLocale("ja");
    expect(loadErrorText(coded("LoadError", "device_lost"))).toBe(
      "GPU が止まりました。描き直すには、エディタを起動し直してください。",
    );
    setLocale("en");
    expect(loadErrorText(coded("LoadError", "device_lost"))).toBe(t("viewport.deviceLost"));
  });

  it("asks to wait or close windows when no GPU context is available", () => {
    setLocale("ja");
    expect(createErrorText(coded("CreateError", "context_unavailable"))).toBe(
      "GPU のコンテキストを作れないため、描画を始められません。ほかのウィンドウを閉じるか、少し待ってから「作り直す」を押してください",
    );
  });

  it("keeps the exception text of unknown codes and other errors", () => {
    expect(createErrorText(coded("CreateError", "something_new"))).toBe("wasm text");
    expect(createErrorText(new Error("plain"))).toBe("plain");
    expect(loadErrorText(coded("LoadError", "something_new"))).toBe("wasm text");
  });
});
