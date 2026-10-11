// ビューアが localStorage に覚える値のキーの一覧 (apps/viewer/README.md の「覚える値」)。ビューアが自分で読み書きするキーはここから使う。
// 形は `ugoe.viewer.<名前>`。モデルごとに覚えるものなど Player が覚えるものは、同じ接頭辞で @ugoe/player が読み書きする
// (`STORAGE_PREFIX` は @ugoe/player の既定の接頭辞と同じ。storageKeys.test.ts で形と重なりを検査する)。
// index.html の <head> のスクリプトは import できないので、テーマのキーを直に書く (同じテストで一致を検査する)。

/** キーの接頭辞 (Player が覚えるものにも使う)。 */
export const STORAGE_PREFIX = "ugoe.viewer";

/** ビューアが Player の外で覚える値のキー。 */
export const STORAGE_KEYS = {
  /** 画面の言語の設定 (@ugoe/i18n の initLocale。「OS に合わせる」は `system`)。 */
  locale: "ugoe.viewer.locale",
  /** テーマ (@ugoe/ui の initTheme。index.html も読む)。 */
  theme: "ugoe.viewer.theme",
  /** フェイストラッキングの調整 (感度と境目。JSON。@ugoe/face-tracking の tuningStorage.ts。T0738)。 */
  "tracking.tuning": "ugoe.viewer.tracking.tuning",
  /** フェイストラッキングのほかの設定 (平滑化・左右の入れ替え・目パチを止める・体。JSON。@ugoe/face-tracking の settingsStorage.ts。T0739)。 */
  "tracking.settings": "ugoe.viewer.tracking.settings",
} as const;

export type StorageKey = (typeof STORAGE_KEYS)[keyof typeof STORAGE_KEYS];
