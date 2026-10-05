// テーマの設定 (OS に合わせる・ライト・ダーク。docs/ui.md#デザイントークン)。アプリに 1 つ。
// 選んだテーマは <html data-theme> に付け、「OS に合わせる」なら属性を外す (tokens.css が prefers-color-scheme で切り替える)。
// 設定は localStorage に `<アプリ>.theme` のキーで覚える (言語の `<アプリ>.locale` と同じ流儀)。
// 起動時のちらつきを避けるため、アプリの index.html の <head> にも同じキーを読んで data-theme を付ける小さなスクリプトを置く
// (README.md の「テーマ」)。
import { readStoredItem, writeStoredItem } from "@ugoe/i18n";

/** テーマの設定。`system` は OS に合わせる。 */
export type ThemePreference = "system" | "light" | "dark";

/** 実際に使っているテーマ。 */
export type Theme = "light" | "dark";

export const THEME_PREFERENCES: readonly ThemePreference[] = ["system", "light", "dark"];

export function isThemePreference(value: unknown): value is ThemePreference {
  return THEME_PREFERENCES.some((p) => p === value);
}

/** 覚えた設定。覚えていない・読めない・知らない値なら「OS に合わせる」。 */
export function loadThemePreference(key: string, storage?: Storage): ThemePreference {
  const v = readStoredItem(key, storage);
  return isThemePreference(v) ? v : "system";
}

/** 設定を覚える。覚えられなくても動く (次に起動したときは OS に合わせる)。 */
export function saveThemePreference(key: string, value: ThemePreference, storage?: Storage): void {
  writeStoredItem(key, value, storage);
}

const DARK_QUERY = "(prefers-color-scheme: dark)";

const state = $state({ preference: "system" as ThemePreference, systemDark: false });
let storageKey: string | null = null;
let stopWatching: (() => void) | null = null;

function apply(root: HTMLElement | undefined = globalThis.document?.documentElement) {
  if (!root) return;
  if (state.preference === "system") delete root.dataset.theme;
  else root.dataset.theme = state.preference;
}

/**
 * 起動時に 1 回 (mount の前): 覚えた設定を <html> に付け、OS のテーマの変化を見張る。`key` は覚えるキー
 * (例: `ugoe-viewer.theme`)。見張りをやめる関数を返す。
 */
export function initTheme(key: string): () => void {
  storageKey = key;
  state.preference = loadThemePreference(key);
  stopWatching?.();
  const media = globalThis.matchMedia?.(DARK_QUERY);
  if (media) {
    state.systemDark = media.matches;
    const onChange = (e: MediaQueryListEvent) => {
      state.systemDark = e.matches;
    };
    media.addEventListener("change", onChange);
    stopWatching = () => media.removeEventListener("change", onChange);
  } else {
    stopWatching = null;
  }
  apply();
  return () => {
    stopWatching?.();
    stopWatching = null;
  };
}

/** 今の設定。テンプレートや `$derived` の中で呼ぶと、変更に追従する。 */
export function getThemePreference(): ThemePreference {
  return state.preference;
}

/**
 * 実際に使っているテーマ (「OS に合わせる」なら OS のテーマ)。OS のテーマの変化にも追従する。
 * CSS の変数を JS で読むところ (canvas の背景など) は、`$effect` の中でこれを読んで読み直す。
 */
export function currentTheme(): Theme {
  if (state.preference !== "system") return state.preference;
  return state.systemDark ? "dark" : "light";
}

/** テーマを切り替えて覚える (`initTheme()` のキーで。呼んでいなければ覚えない)。 */
export function setThemePreference(value: ThemePreference): void {
  state.preference = value;
  apply();
  if (storageKey !== null) saveThemePreference(storageKey, value);
}
