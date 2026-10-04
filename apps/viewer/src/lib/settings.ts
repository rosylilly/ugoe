// ビューアが覚える、モデルによらない設定 (localStorage)。モデルごとに覚えるものは @ugoe/player にある。

/** 画面の言語の設定のキー (@ugoe/i18n の initLocale に渡す。「OS に合わせる」は `system`)。 */
export const LOCALE_STORAGE_KEY = "ugoe-viewer.locale";
/** テーマの設定のキー (@ugoe/ui の initTheme に渡す。index.html の <head> のスクリプトも同じキーを読む)。 */
export const THEME_STORAGE_KEY = "ugoe-viewer.theme";
