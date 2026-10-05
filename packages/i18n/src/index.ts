// 画面の文言の翻訳 (ADR 0029、docs/ui.md#文言の書き方)。i18next を包み、言語の検出と切り替え・書式・カタログの型を出す。
// アプリと部品は i18next を直接 import せず、ここから出すものだけを使う。
export { browserLanguages, languageOf, resolveLocale } from "./detect";
export { formatDate, formatList, formatNumber, formatRelativeTime } from "./format";
export {
  type Catalog,
  getLocale,
  getLocalePreference,
  hasTranslation,
  type LocaleCatalogs,
  registerCatalogs,
  setLocale,
  setSystemLanguages,
  translator,
} from "./i18n.svelte";
export {
  isLocale,
  LANGUAGES,
  type Language,
  LOCALES,
  type Locale,
  type LocalePreference,
  SOURCE_LOCALE,
} from "./languages";
export {
  initLocale,
  loadLocalePreference,
  type SystemLanguagesSource,
  saveLocalePreference,
  setLocalePreference,
} from "./preference";
export { default as RichText } from "./RichText.svelte";
export { parseRich, type RichPart } from "./rich";
export { readStoredItem, removeStoredItem, writeStoredItem } from "./storage";

/**
 * 名前空間 → 原文 (英語) のカタログの型。パッケージは自分のカタログを足し、`translator()` の `t()` のキーを型で検査させる:
 *
 * ```ts
 * import en from "../locales/en.json";
 * declare module "@ugoe/i18n" {
 *   interface CatalogTypes {
 *     "face-tracking": typeof en;
 *   }
 * }
 * ```
 */
// biome-ignore lint/suspicious/noEmptyInterface: パッケージが宣言の結合で足す
export interface CatalogTypes {}

declare module "i18next" {
  // i18next の名前空間の型の登録 (複数のパッケージがそれぞれ足せる形)。CatalogTypes をそのまま渡す。
  interface ResourceNamespaceMap extends CatalogTypes {}
}
