// 翻訳の実行 (ADR 0029 の 2・3)。i18next の 1 つのインスタンスを包み、今の言語を `$state` で持つ。
// `translator()` が返す `t()` は今の言語を読むので、言語を切り替えると `t()` を使う式 (テンプレート・$derived) が再評価される。
//
// - カタログは各パッケージ・アプリの `locales/<言語>.json` (i18next JSON v4) を `registerCatalogs()` で渡す。全言語をバンドルに含め、HTTP では読まない。
// - 言語は「OS に合わせる」(既定) か、`setLocale()` で決める。OS の言語は既定で WebView の `navigator.languages`、
//   アプリが OS から取った一覧を `setSystemLanguages()` で渡すとそれを使う。

import i18next, { type TFunction } from "i18next";
import { browserLanguages, resolveLocale } from "./detect";
import type { CatalogTypes } from "./index";
import { LOCALES, type Locale, type LocalePreference, SOURCE_LOCALE } from "./languages";

/** カタログ (i18next JSON v4)。値は文字列か、入れ子のカタログ。 */
export interface Catalog {
  [key: string]: string | Catalog;
}

/** 名前空間のカタログ。原文の言語 (英語) は必須で、ほかの言語は欠けてよい (欠けたキーは英語を出す)。 */
export type LocaleCatalogs = { en: Catalog } & Partial<Record<Locale, Catalog>>;

const instance = i18next.createInstance();
void instance.init({
  initAsync: false,
  lng: SOURCE_LOCALE,
  fallbackLng: SOURCE_LOCALE,
  supportedLngs: LOCALES,
  resources: {},
  ns: [],
  defaultNS: false,
  // Svelte が文字をエスケープするので、i18next ではエスケープしない (docs/ui.md#引数)。
  interpolation: { escapeValue: false },
  // 空の訳 (翻訳サービスで未訳のまま書き出された値) は、原文を出す。
  returnEmptyString: false,
  returnNull: false,
});

let preference = $state<LocalePreference>("system");
let systemLanguages = $state<readonly string[]>(browserLanguages());
/** アプリが OS の言語を渡したか (渡していれば、WebView の言語の変化を追わない)。 */
let systemFromApp = false;

/** 今の言語。`$state` を読むので、`$derived` やテンプレートの中で読むと言語の切り替えに追従する。 */
export function getLocale(): Locale {
  return preference === "system" ? resolveLocale(systemLanguages) : preference;
}

/** 今の言語の設定 (`"system"` は OS に合わせる)。 */
export function getLocalePreference(): LocalePreference {
  return preference;
}

/** 言語を切り替える。`"system"` で OS の言語に戻す。設定を覚えるのはアプリの役目。 */
export function setLocale(next: LocalePreference): void {
  preference = next;
  applyLocale();
}

/**
 * OS の優先する言語の一覧 (BCP 47 のタグ。前ほど優先) を渡す。WebView によっては `navigator.languages` が
 * OS の言語と違う値を返すので、アプリが OS から取れる場合に使う (README の「OS の言語」)。
 */
export function setSystemLanguages(languages: readonly string[]): void {
  systemFromApp = true;
  systemLanguages = [...languages];
  applyLocale();
}

/** i18next と `<html lang>` を今の言語に合わせる。 */
function applyLocale(): void {
  const locale = getLocale();
  if (instance.language !== locale) void instance.changeLanguage(locale);
  if (typeof document !== "undefined" && document.documentElement) {
    document.documentElement.lang = locale;
  }
}

// テストの仮の window (addEventListener を持たない) でも読み込めるようにする。
if (typeof window !== "undefined" && typeof window.addEventListener === "function") {
  window.addEventListener("languagechange", () => {
    if (systemFromApp) return;
    systemLanguages = browserLanguages();
    applyLocale();
  });
}
applyLocale();

/**
 * 名前空間 `ns` のカタログを登録する。パッケージは自分の `locales/*.json` を読み込む所で 1 回呼ぶ。
 * 同じ名前空間を登録し直すと、後のカタログで置き換える。
 */
export function registerCatalogs(ns: string, catalogs: LocaleCatalogs): void {
  for (const [locale, catalog] of Object.entries(catalogs)) {
    if (catalog) instance.addResourceBundle(locale, ns, catalog, true, true);
  }
}

type LooseT = (key: string | readonly string[], options?: Record<string, unknown>) => string;

/**
 * 名前空間 `ns` の `t()` を返す。キーと戻り値の型は、パッケージが `Catalogs` に足した原文のカタログから決まる
 * (README の「型」)。`t()` は呼ぶたびに今の言語を読むので、Svelte のテンプレートでそのまま使える。
 */
export function translator<const Ns extends keyof CatalogTypes & string>(ns: Ns): TFunction<Ns> {
  const t = instance.t as unknown as LooseT;
  const translate: LooseT = (key, options) => t(key, { ...options, lng: getLocale(), ns });
  return translate as unknown as TFunction<Ns>;
}

/** キーが今の言語 (欠けていれば英語) のカタログにあるか。バックエンドのエラーのコードを訳せるかに使う。 */
export function hasTranslation(ns: string, key: string): boolean {
  const exists = instance.exists as unknown as (key: string, options: object) => boolean;
  return exists(key, { lng: getLocale(), ns });
}
