// 訳の表 (loadError.ts・loadWarnings.ts・dressUp.ts) が、ugoe-wasm のすべての `code` (crates/ugoe-wasm/codes.json。
// @ugoe/runtime の `LOAD_ERROR_CODES`・`LOAD_WARNING_CODES`・`DRESS_UP_ERROR_CODES`・`RENDER_ERROR_CODES` と同じ一覧) と補足の値の組を、
// すべての言語で文にするか (T0934)。wasm に種類を足して訳を足し忘れると、このテストが落ちる。
import { LANGUAGES, SOURCE_LOCALE, setLocale } from "@ugoe/i18n";
import { afterEach, describe, expect, it } from "vitest";
import codes from "../../../crates/ugoe-wasm/codes.json";
import { describeDressUpError, describeLoadWarnings } from "./dressUp";
import { describeLoadError, describeRenderError } from "./loadError";
import { readLoadWarnings } from "./loadWarnings";

/** wasm の文 (訳が無いと、これがそのまま出る)。 */
const WASM_TEXT = "wasm text";

/** 補足の値のうち文字列で、文を選ぶもの (取りうる値をすべて試す) と、決まった形のもの。 */
const STRING_FIELDS: Record<string, string[]> = {
  image: ["texture", "normal_map"],
  limit: ["chunk_decompressed", "total_decompressed"],
  chunk: ["TEXR"],
  features: ["x.a, x.b"],
};

/** 補足の値のうち数のもの (ほかは名前の文字列)。 */
const NUMBER_FIELDS = new Set([
  "actual",
  "errors",
  "found",
  "height",
  "max",
  "required",
  "supported",
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

/** ugoe-wasm の `Error` と同じ形 (`name`・`code`・補足の値)。 */
const codedError = (name: string, code: string, fields: Record<string, unknown>) =>
  Object.assign(new Error(WASM_TEXT), { name, code, ...fields });

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
      expect(text, label).not.toMatch(/^(loadError|loadWarning|dressUp)\./);
      expect(text, label).not.toContain("{{");
      if (language !== SOURCE_LOCALE) expect(text, label).not.toBe(source);
    }
  }
}

afterEach(() => setLocale("en"));

describe("translation tables against crates/ugoe-wasm/codes.json", () => {
  it("lists the codes of ugoe-wasm", () => {
    expect(Object.keys(codes.load_error).length).toBeGreaterThan(5);
    expect(Object.keys(codes.load_warning).length).toBeGreaterThan(5);
    expect(Object.keys(codes.dress_up_error).length).toBeGreaterThan(5);
  });

  it("translates every load error code", () => {
    const samples = Object.entries(codes.load_error).flatMap(([code, fields]) =>
      combos(fields).map((f): [string, unknown] => [code, codedError("LoadError", code, f)]),
    );
    expectTranslated(samples, describeLoadError);
  });

  it("translates every dress-up error code", () => {
    const samples = Object.entries(codes.dress_up_error).flatMap(([code, fields]) =>
      combos(fields).map((f): [string, unknown] => [code, codedError("DressUpError", code, f)]),
    );
    expectTranslated(samples, describeDressUpError);
  });

  it("translates every render error code", () => {
    const samples = Object.entries(codes.render_error).flatMap(([code, fields]) =>
      combos(fields).map((f): [string, unknown] => [code, codedError("RenderError", code, f)]),
    );
    expectTranslated(samples, describeRenderError);
  });

  it("translates every load warning code", () => {
    const samples = Object.entries(codes.load_warning).flatMap(([code, fields]) =>
      combos(fields).map((f): [string, unknown] => {
        // 外したモジュールの `error` は着せ替えの失敗の形 (`code` と `message`)。
        const error =
          f.error === undefined ? {} : { error: { code: "unknown_slot", message: "x" } };
        return [code, { ...f, ...error, code, warning: WASM_TEXT }];
      }),
    );
    expectTranslated(samples, (sample) => {
      const texts = describeLoadWarnings([WASM_TEXT], readLoadWarnings([sample]));
      return texts[0] ?? "";
    });
  });
});
