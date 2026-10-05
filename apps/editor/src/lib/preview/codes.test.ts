// プレビューの訳の表 (loadWarnings.ts の警告・dressUp.ts の着せ替えの失敗) が、ugoe-wasm のすべての `code`
// (crates/ugoe-wasm/codes.json の `load_warning`・`dress_up_error`) と補足の値の組を、エディタの言語のすべてで
// 文にするか (T0934)。読み込みの失敗 (`load_error`) は viewport/loadError.test.ts が確かめる。
import { LANGUAGES, SOURCE_LOCALE, setLocale } from "@ugoe/i18n";
import { afterEach, describe, expect, it } from "vitest";
import codes from "../../../../../crates/ugoe-wasm/codes.json";
import { describeProblem } from "./dressUp";
import { describeLoadWarnings, readLoadWarnings } from "./loadWarnings";

/** wasm の文 (訳が無いと、これがそのまま出る)。 */
const WASM_TEXT = "wasm text";

/** 補足の値のうち文字列で、文を選ぶもの (取りうる値をすべて試す) と、決まった形のもの。 */
const STRING_FIELDS: Record<string, string[]> = {
  image: ["texture", "normal_map"],
  limit: ["chunk_decompressed", "total_decompressed"],
  chunk: ["TEXR"],
};

/** 補足の値のうち数のもの (ほかは名前の文字列)。 */
const NUMBER_FIELDS = new Set([
  "actual",
  "errors",
  "height",
  "max",
  "required",
  "texture",
  "width",
]);

/** 補足の値の取りうる組 (`fields` は codes.json のキー)。 */
function combos(fields: readonly string[]): Record<string, unknown>[] {
  let list: Record<string, unknown>[] = [{}];
  for (const field of fields) {
    const values: unknown[] = STRING_FIELDS[field] ?? [NUMBER_FIELDS.has(field) ? 1234 : "Name"];
    list = list.flatMap((combo) => values.map((value) => ({ ...combo, [field]: value })));
  }
  return list;
}

/**
 * `describe` が、すべての言語で `samples` のすべてを訳すか。原文の言語のほかは、原文と違う文になる
 * (訳が欠けると原文 (英語) に戻る)。
 */
function expectTranslated(samples: [string, unknown][], describe: (sample: unknown) => string) {
  for (const language of LANGUAGES.map((l) => l.tag)) {
    for (const [code, sample] of samples) {
      setLocale(SOURCE_LOCALE);
      const source = describe(sample);
      setLocale(language);
      const text = describe(sample);
      const label = `${code} (${language})`;
      expect(text, label).not.toBe(WASM_TEXT);
      expect(text, label).not.toMatch(/^(preview|loadError)\./);
      expect(text, label).not.toContain("{{");
      if (language !== SOURCE_LOCALE) expect(text, label).not.toBe(source);
    }
  }
}

afterEach(() => setLocale("en"));

describe("preview translation tables against crates/ugoe-wasm/codes.json", () => {
  it("lists the codes of ugoe-wasm", () => {
    expect(Object.keys(codes.load_warning).length).toBeGreaterThan(5);
    expect(Object.keys(codes.dress_up_error).length).toBeGreaterThan(5);
  });

  it("translates every load warning code", () => {
    // 外したモジュール (`module_detached`) は、着せ替えが `describeProblem` で知らせる (下の it)。
    const entries = Object.entries(codes.load_warning).filter(
      ([code]) => code !== "module_detached",
    );
    const samples = entries.flatMap(([code, fields]) =>
      combos(fields).map((f): [string, unknown] => [code, { ...f, code, warning: WASM_TEXT }]),
    );
    expectTranslated(samples, (sample) => {
      const texts = describeLoadWarnings(readLoadWarnings([sample]), false);
      return texts[0] ?? "";
    });
  });

  it("translates every dress-up error code", () => {
    const samples = Object.entries(codes.dress_up_error).flatMap(([code, fields]) =>
      combos(fields).map((f): [string, unknown] => [
        code,
        Object.assign(new Error(WASM_TEXT), { name: "DressUpError", code, ...f }),
      ]),
    );
    expectTranslated(samples, (sample) => describeProblem(sample).summary);
  });
});
