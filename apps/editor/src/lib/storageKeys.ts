// エディタが localStorage に覚える値のキーの一覧 (docs/editor.md#覚える値)。キーはすべてここから使う。
// 形は `ugoe.editor.<名前>` (名前は camelCase)。storageKeys.test.ts で形と重なりを検査する。
// index.html の <head> のスクリプトは import できないので、テーマのキーを直に書く (同じテストで一致を検査する)。

/** キーの接頭辞。 */
export const STORAGE_PREFIX = "ugoe.editor";

/** 覚える値のキー。 */
export const STORAGE_KEYS = {
  /** 画面の言語の設定 (@ugoe/i18n の initLocale。lib/locale.ts)。 */
  locale: "ugoe.editor.locale",
  /** テーマ (@ugoe/ui の initTheme。index.html も読む)。 */
  theme: "ugoe.editor.theme",
  /** 未保存の変更がなければ外での書き換えを自動で開き直す (lib/externalChange.ts)。 */
  autoReload: "ugoe.editor.autoReload",
  /** タイムラインのパネルを開いておくか (panels/timeline/timeline.ts)。 */
  timelineOpen: "ugoe.editor.timelineOpen",
  /** パラメータの一覧の閉じた区切り (グループの ID の JSON の配列。panels/parameters/sections.ts)。 */
  parameterSectionsClosed: "ugoe.editor.parameterSectionsClosed",
  /** 左の列の幅 (lib/layout.ts)。 */
  leftWidth: "ugoe.editor.leftWidth",
  /** 右の列の幅 (lib/layout.ts)。 */
  rightWidth: "ugoe.editor.rightWidth",
  /** タイムラインの高さ (lib/layout.ts)。 */
  timelineHeight: "ugoe.editor.timelineHeight",
  /** 右の列の上下の分け方 (lib/layout.ts)。 */
  rightSplit: "ugoe.editor.rightSplit",
  /** プレビューのフェイストラッキングの調整 (感度と境目。JSON。preview/preview.svelte.ts。T1678)。 */
  trackingTuning: "ugoe.editor.trackingTuning",
} as const;

export type StorageKey = (typeof STORAGE_KEYS)[keyof typeof STORAGE_KEYS];
