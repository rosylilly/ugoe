// ビューアが覚える、モデルによらない設定 (localStorage)。モデルごとに覚えるものは @ugoe/player にある。
import { isLocale, type LocalePreference } from "@ugoe/i18n";

/** 画面の言語の設定のキー (「OS に合わせる」は `system`)。 */
export const LOCALE_STORAGE_KEY = "ugoe-viewer.locale";

/** 覚えた言語の設定。覚えていない・読めない・知らない値なら「OS に合わせる」。 */
export function loadLocalePreference(storage?: Storage): LocalePreference {
  try {
    const v = (storage ?? globalThis.localStorage)?.getItem(LOCALE_STORAGE_KEY);
    return v === "system" || isLocale(v) ? v : "system";
  } catch {
    return "system";
  }
}

/** 言語の設定を覚える。覚えられなくても動く (次に起動したときは OS に合わせる)。 */
export function saveLocalePreference(value: LocalePreference, storage?: Storage): void {
  try {
    (storage ?? globalThis.localStorage)?.setItem(LOCALE_STORAGE_KEY, value);
  } catch {
    // 使えない localStorage (プライベートモードなど) では覚えない。
  }
}
