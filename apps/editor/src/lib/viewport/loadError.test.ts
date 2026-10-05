// ビューポートの読み込みの失敗の訳 (@ugoe/player の describeLoadError。T1611) が、ugoe-wasm の `LoadError` の
// すべての `code` (crates/ugoe-wasm/codes.json の `load_error`) と補足の値の組を、エディタの言語のすべてで文にするか。
import { LANGUAGES, SOURCE_LOCALE, setLocale } from "@ugoe/i18n";
import { describeLoadError } from "@ugoe/player";
import { afterEach, describe, expect, it } from "vitest";
import codes from "../../../../../crates/ugoe-wasm/codes.json";

/** 補足の値のうち文字列のもの (ほかは数)。`image`・`limit` は文を選ぶので、取りうる値をすべて試す。 */
const STRING_FIELDS: Record<string, string[]> = {
  image: ["texture", "normal_map"],
  limit: ["chunk_decompressed", "total_decompressed"],
  chunk: ["TEXR"],
  features: ["x.a, x.b"],
};

/** `code` の `LoadError` を、補足の値の取りうる組ごとに作る。 */
function samples(code: string, fields: string[]): Error[] {
  let combos: Record<string, string | number>[] = [{}];
  for (const field of fields) {
    const values = STRING_FIELDS[field] ?? [1234];
    combos = combos.flatMap((combo) => values.map((value) => ({ ...combo, [field]: value })));
  }
  return combos.map((combo) =>
    Object.assign(new Error("wasm text"), { name: "LoadError", code, ...combo }),
  );
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
        const source = describeLoadError(error);
        setLocale(language);
        const text = describeLoadError(error);
        expect(text, code).not.toBe("wasm text");
        expect(text, code).not.toMatch(/^loadError\./);
        expect(text, code).not.toContain("{{");
        // 訳が欠けると原文 (英語) に戻るので、原文の言語のほかは原文と違う文になる。
        if (language !== SOURCE_LOCALE) expect(text, code).not.toBe(source);
      }
    }
  });
});
