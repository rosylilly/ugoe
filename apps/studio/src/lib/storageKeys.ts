// スタジオが localStorage に覚える値のキーの一覧 (docs/studio.md#覚える値)。キーはすべてここから使う。
// 形は `ugoe.studio.<名前>`。出力ウィンドウの Player (@ugoe/player) が覚えるものは `ugoe.studio.player.<名前>`
// (storageKeys.test.ts で形と重なりを検査する)。
// index.html の <head> のスクリプトは import できないので、テーマのキーを直に書く (同じテストで一致を検査する)。

/** キーの接頭辞。 */
export const STORAGE_PREFIX = "ugoe.studio";

/** 操作ウィンドウが覚える値のキー。 */
export const STORAGE_KEYS = {
  /** 画面の言語の設定 (@ugoe/i18n の initLocale)。 */
  locale: "ugoe.studio.locale",
  /** テーマ (@ugoe/ui の initTheme。index.html も読む)。出力ウィンドウには効かせない。 */
  theme: "ugoe.studio.theme",
} as const;

export type StorageKey = (typeof STORAGE_KEYS)[keyof typeof STORAGE_KEYS];

/** 出力ウィンドウの Player (@ugoe/player) が覚えるもののキーの接頭辞 (`PlayerHost.storagePrefix`。T0930)。 */
export const PLAYER_STORAGE_PREFIX = "ugoe.studio.player";
