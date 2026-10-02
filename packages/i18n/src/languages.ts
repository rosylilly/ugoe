// 対応する言語の一覧 (ADR 0029 の 3)。一覧の正は languages.json で、カタログの検査
// (scripts/check-catalogs.mjs) も同じファイルを読む。言語を足すときは、languages.json と下の `Locale` に加え、
// 各パッケージの `locales/<言語>.json` を足す。
import languages from "./languages.json";

/** 対応する言語のタグ (BCP 47 の言語の部分)。 */
export type Locale = "en" | "ja";

/** 言語の設定: OS に合わせる (`"system"`) か、言語を決める。 */
export type LocalePreference = "system" | Locale;

export interface Language {
  tag: Locale;
  /** その言語自身で書いた名前 (「日本語」「English」)。言語の切り替えの選択肢に出す。 */
  name: string;
  /** 常に欠けのない訳として検査する言語か (`just i18n`)。 */
  complete: boolean;
}

/** 原文の言語。訳が欠けたらこの言語を出す。 */
export const SOURCE_LOCALE = languages.source as Locale;

/** 対応する言語 (languages.json の順)。 */
export const LANGUAGES: readonly Language[] = languages.languages.map((l) => ({
  tag: l.tag as Locale,
  name: l.name,
  complete: "complete" in l && l.complete === true,
}));

/** 対応する言語のタグ。 */
export const LOCALES: readonly Locale[] = LANGUAGES.map((l) => l.tag);

/** `value` が対応する言語のタグか。 */
export function isLocale(value: unknown): value is Locale {
  return typeof value === "string" && (LOCALES as readonly string[]).includes(value);
}
