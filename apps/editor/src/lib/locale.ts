// 画面の言語の設定 (設定の「General」。docs/editor.md#言語)。覚える・戻すのは @ugoe/i18n の initLocale・setLocalePreference。
// OS の言語は WebView の navigator.languages ではなく、tauri-plugin-os の locale() で OS から取る
// (WebView によっては OS の言語と違う値を返すため。packages/i18n/README.md の「OS の言語」)。

/** 画面の言語の設定のキー (ビューアの `ugoe-viewer.locale`・スタジオの `ugoe-studio.locale` と同じ形)。 */
export const LOCALE_KEY = "ugoe-editor.locale";

/** OS の言語 (initLocale に渡す)。Tauri の外 (ヘッドレスのブラウザでの確認) では `null` (navigator.languages のまま)。 */
export async function systemLocale(): Promise<string | null> {
  if (!("__TAURI_INTERNALS__" in globalThis)) return null;
  const { locale } = await import("@tauri-apps/plugin-os");
  return locale();
}
