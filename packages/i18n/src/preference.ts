// 言語の設定を覚える・起動時に戻す (T0925)。テーマ (@ugoe/ui の theme.svelte.ts) と同じ流儀で、設定は localStorage に
// アプリが渡すキー (`<アプリ>.locale`。例: `ugoe-viewer.locale`) で覚える。
// OS の言語の取り方 (Tauri の tauri-plugin-os など) はアプリが関数で渡す (このパッケージは Tauri に依存しない。README の「OS の言語」)。
import { setLocale, setSystemLanguages } from "./i18n.svelte";
import { isLocale, type LocalePreference } from "./languages";

/** 覚えた言語の設定。覚えていない・読めない・知らない値なら「OS に合わせる」(`"system"`)。 */
export function loadLocalePreference(key: string, storage?: Storage): LocalePreference {
  try {
    const v = (storage ?? globalThis.localStorage)?.getItem(key);
    return v === "system" || isLocale(v) ? v : "system";
  } catch {
    return "system";
  }
}

/** 言語の設定を覚える。覚えられなくても動く (次に起動したときは OS に合わせる)。 */
export function saveLocalePreference(
  key: string,
  value: LocalePreference,
  storage?: Storage,
): void {
  try {
    (storage ?? globalThis.localStorage)?.setItem(key, value);
  } catch {
    // 使えない localStorage (プライベートモードなど) では覚えない。
  }
}

/** OS の言語を取る関数。BCP 47 のタグか、その一覧 (前ほど優先)。取れなければ `null`。 */
export type SystemLanguagesSource = () => Promise<string | readonly string[] | null>;

let storageKey: string | null = null;
/** `initLocale()` の世代。待つ間に呼び直されたら、古い呼び出しが取った OS の言語は捨てる。 */
let generation = 0;

/**
 * 起動時に 1 回 (mount の前): `key` で覚えた設定を使い、`systemLanguages` があれば OS の言語を取って
 * `setSystemLanguages()` に渡す。返す Promise は OS の言語を渡し終えると (取れなくても) 解決し、拒否はしない
 * (最初の画面から OS の言語で出したいアプリは待ってから描く)。`systemLanguages` を省くか取れなければ `navigator.languages` のまま。
 */
export async function initLocale(
  key: string,
  systemLanguages?: SystemLanguagesSource,
): Promise<void> {
  storageKey = key;
  const current = ++generation;
  setLocale(loadLocalePreference(key));
  if (!systemLanguages) return;
  try {
    const languages = await systemLanguages();
    if (languages === null || current !== generation) return;
    const list = typeof languages === "string" ? [languages] : languages;
    if (list.length > 0) setSystemLanguages(list);
  } catch {
    // 取れなければ navigator.languages のまま。
  }
}

/** 言語を切り替えて覚える (`initLocale()` のキーで。呼んでいなければ覚えない)。 */
export function setLocalePreference(value: LocalePreference): void {
  setLocale(value);
  if (storageKey !== null) saveLocalePreference(storageKey, value);
}
