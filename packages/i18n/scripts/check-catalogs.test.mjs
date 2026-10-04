import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { checkLocales, placeholders, pluralForm, sourceStrings } from "./check-catalogs.mjs";

const LANGUAGES = {
  source: "en",
  languages: [
    { tag: "en", name: "English" },
    { tag: "ja", name: "日本語", complete: true },
    { tag: "fr", name: "Français" },
  ],
};

let dir;
afterEach(() => dir && rmSync(dir, { recursive: true, force: true }));

/** `locales/` と `src/` を作り、検査の結果を返す。 */
function check(catalogs, source) {
  dir = mkdtempSync(join(tmpdir(), "ugoe-i18n-"));
  mkdirSync(join(dir, "locales"));
  mkdirSync(join(dir, "src"));
  for (const [tag, json] of Object.entries(catalogs)) {
    writeFileSync(join(dir, "locales", `${tag}.json`), JSON.stringify(json));
  }
  writeFileSync(join(dir, "src", "main.ts"), source);
  return checkLocales(join(dir, "locales"), { languages: LANGUAGES }).map((e) =>
    e.slice(e.indexOf("locales")),
  );
}

// biome-ignore lint/suspicious/noTemplateCurlyInString: 検査するソースの文字列
const USE_ALL = 't("a.title"); t("files", { count }); t(`kind.${k}`);';

describe("checkLocales", () => {
  const en = {
    a: { title: "Title {{name}}" },
    files_one: "{{count}} file",
    files_other: "{{count}} files",
    kind: { x: "X" },
  };
  const ja = { a: { title: "題 {{name}}" }, files_other: "{{count}} 個", kind: { x: "エックス" } };

  it("accepts consistent catalogs", () => {
    expect(check({ en, ja, fr: { a: { title: "Titre {{name}}" } } }, USE_ALL)).toEqual([]);
  });

  it("reports missing and extra keys", () => {
    const errors = check({ en, ja: { ...ja, kind: {}, extra: "余り" } }, USE_ALL);
    expect(errors).toEqual([
      "locales/ja.json: extra: 原文 (en.json) に無いキー",
      "locales/ja.json: kind.x: 訳が無い (ja は欠けのない訳にする)",
    ]);
  });

  it("checks plural categories per language", () => {
    const errors = check(
      { en: { ...en, files_one: undefined }, ja: { ...ja, files_one: "1 個" } },
      USE_ALL,
    );
    expect(errors).toEqual([
      "locales/en.json: files: 複数形の区分 _one が無い",
      "locales/ja.json: files_one: ja に無い複数形の区分 (other)",
    ]);
  });

  it("checks placeholders and empty strings", () => {
    const errors = check(
      { en, ja: { ...ja, a: { title: "題 {{title}}" }, kind: { x: "" } } },
      USE_ALL,
    );
    expect(errors).toEqual([
      "locales/ja.json: a.title: 引数が原文と違う (無い: name。原文に無い: title)",
      "locales/ja.json: kind.x: 空の文字列",
    ]);
  });

  it("reports unused keys", () => {
    expect(check({ en, ja }, 't("a.title");')).toEqual([
      "locales/en.json: files: 使われていないキー",
      "locales/en.json: kind.x: 使われていないキー",
    ]);
  });

  it("reports a catalog that is not an object", () => {
    expect(check({ en, ja: null, fr: [] }, USE_ALL)).toEqual([
      "locales/fr.json: カタログは JSON のオブジェクトにする",
      "locales/ja.json: カタログは JSON のオブジェクトにする",
    ]);
  });

  it("requires the source catalog and known languages", () => {
    expect(check({ ja, de: {} }, "")).toEqual([
      "locales/de.json: 対応する言語ではない (packages/i18n/src/languages.json に無い)",
      "locales: 原文のカタログ en.json が無い",
    ]);
  });
});

describe("helpers", () => {
  it("parses plural suffixes and placeholders", () => {
    expect(pluralForm("a.b_other")).toEqual({ base: "a.b", form: "other" });
    expect(pluralForm("a.other")).toEqual({ base: "a.other", form: null });
    expect([...placeholders("{{a}} {{ b, number }} {{a}}")]).toEqual(["a", "b"]);
  });

  it("finds string literals and template prefixes", () => {
    // biome-ignore lint/suspicious/noTemplateCurlyInString: 検査するソースの文字列
    const found = sourceStrings('t(\'x.y\'); t("z"); t(`kind.${k}`); `plain`; `${t("in")} ·`');
    expect([...found.literals]).toEqual(["x.y", "z", "plain", "in"]);
    expect([...found.prefixes]).toEqual(["kind."]);
  });
});
