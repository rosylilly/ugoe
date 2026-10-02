// 画面の言語の設定 (設定の「General」。docs/editor.md#言語)。既定は OS に合わせる。localStorage に覚える (使えない環境では既定)。
// OS の言語は WebView の navigator.languages ではなく、tauri-plugin-os の locale() で OS から取る
// (WebView によっては OS の言語と違う値を返すため。packages/i18n/README.md の「OS の言語」)。
import { isLocale, type LocalePreference, setLocale, setSystemLanguages } from "@ugoe/i18n";

/** 画面の言語の設定のキー (ビューアの `ugoe-viewer.locale`・スタジオの `ugoe-studio.locale` と同じ形)。 */
export const LOCALE_KEY = "ugoe-editor.locale";

export function loadLocalePreference(): LocalePreference {
  try {
    const saved = globalThis.localStorage?.getItem(LOCALE_KEY) ?? null;
    return saved === "system" || isLocale(saved) ? saved : "system";
  } catch {
    return "system";
  }
}

export function saveLocalePreference(value: LocalePreference): void {
  try {
    globalThis.localStorage?.setItem(LOCALE_KEY, value);
  } catch {
    // 保存できなくても、この画面では今の言語を使い続ける。
  }
}

/** 言語を切り替えて覚える。 */
export function changeLocale(value: LocalePreference): void {
  setLocale(value);
  saveLocalePreference(value);
}

/**
 * 起動時に 1 回: 覚えた設定を使い、OS の言語を Tauri から取って渡す。Tauri の外 (ヘッドレスのブラウザでの確認) では
 * navigator.languages のまま。
 */
export function initLocale(): void {
  setLocale(loadLocalePreference());
  if (!("__TAURI_INTERNALS__" in globalThis)) return;
  void import("@tauri-apps/plugin-os")
    .then(({ locale }) => locale())
    .then((tag) => {
      if (tag !== null) setSystemLanguages([tag]);
    })
    .catch(() => {
      // 取れなければ navigator.languages のまま。
    });
}
