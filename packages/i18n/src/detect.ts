// 言語の検出 (ADR 0029 の 3)。OS (または WebView) の優先する言語の一覧を前から見て、最初に対応している言語を使う。

import { LOCALES, type Locale, SOURCE_LOCALE } from "./languages";

/** BCP 47 のタグの言語の部分を小文字で返す (`ja-JP` → `ja`、`zh_Hant_TW` → `zh`)。 */
export function languageOf(tag: string): string {
  return tag.trim().split(/[-_]/, 1)[0]?.toLowerCase() ?? "";
}

/**
 * 優先する言語の一覧 (`navigator.languages` や OS の言語) から、最初に対応している言語を選ぶ。
 * どれも対応していなければ原文の言語 (英語)。
 */
export function resolveLocale(
  preferred: readonly string[],
  supported: readonly Locale[] = LOCALES,
  fallback: Locale = SOURCE_LOCALE,
): Locale {
  for (const tag of preferred) {
    const language = languageOf(tag);
    const found = supported.find((l) => l === language);
    if (found) return found;
  }
  return fallback;
}

/** WebView (ブラウザ) が知らせる優先する言語の一覧。取れない環境 (Node) では空。 */
export function browserLanguages(): readonly string[] {
  if (typeof navigator === "undefined") return [];
  if (navigator.languages?.length) return navigator.languages;
  return navigator.language ? [navigator.language] : [];
}
