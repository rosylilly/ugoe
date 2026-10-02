// 数値・日付・一覧の書式 (docs/ui.md#数値日付単位)。`Intl` を今の言語で使う。
// 今の言語を読むので、Svelte のテンプレートや `$derived` の中で呼ぶと言語の切り替えに追従する。

import { getLocale } from "./i18n.svelte";

/** 数値 (`Intl.NumberFormat`)。桁の区切りと小数の記号は今の言語に従う。 */
export function formatNumber(value: number | bigint, options?: Intl.NumberFormatOptions): string {
  return new Intl.NumberFormat(getLocale(), options).format(value);
}

/** 日付と時刻 (`Intl.DateTimeFormat`)。並び順は `dateStyle`・`timeStyle` で決め、自分で組み立てない。 */
export function formatDate(value: Date | number, options?: Intl.DateTimeFormatOptions): string {
  return new Intl.DateTimeFormat(getLocale(), options).format(value);
}

/** 「3 分前」などの相対的な時間 (`Intl.RelativeTimeFormat`)。 */
export function formatRelativeTime(
  value: number,
  unit: Intl.RelativeTimeFormatUnit,
  options?: Intl.RelativeTimeFormatOptions,
): string {
  return new Intl.RelativeTimeFormat(getLocale(), options).format(value, unit);
}

/** 一覧 (「A、B、C」・「A, B, and C」。`Intl.ListFormat`)。 */
export function formatList(items: Iterable<string>, options?: Intl.ListFormatOptions): string {
  return new Intl.ListFormat(getLocale(), options).format(items);
}
