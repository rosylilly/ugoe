// 操作ウィンドウが覚える設定 (localStorage。言語・テーマ・刻みの速さ) と、出力ウィンドウの大きさのプリセット (docs/studio.md#出力ウィンドウ)。
// 出力の大きさはプロファイル (lib/profile.ts の `Profile.output`) に持つ (T0802)。刻みの速さもプロファイルへ移すのは後続の候補。
import { isLocale, type LocalePreference } from "@ugoe/i18n";

/** 画面の言語の設定のキー。 */
export const LOCALE_STORAGE_KEY = "ugoe-studio.locale";
/** テーマの設定のキー (@ugoe/ui の initTheme に渡す。index.html の <head> のスクリプトも同じキーを読む)。出力ウィンドウには効かせない。 */
export const THEME_STORAGE_KEY = "ugoe-studio.theme";
/** 刻みの速さのキー。 */
export const TICK_RATE_STORAGE_KEY = "ugoe-studio.tick-rate";

/** 刻みの速さ (回/秒。ADR 0028 の 3.)。出力のフレームレートになる。 */
export type TickRate = 30 | 60 | 120;

export const TICK_RATES: readonly TickRate[] = [30, 60, 120];

/** 既定の刻みの速さ (docs/studio.md の表)。 */
export const DEFAULT_TICK_RATE: TickRate = 60;

/** 出力ウィンドウの大きさのプリセット (物理ピクセル)。`free` は大きさを変えない。 */
export type OutputSizePreset = "1920x1080" | "1280x720" | "1080x1920" | "1080x1080" | "free";

export const OUTPUT_SIZE_PRESETS: readonly OutputSizePreset[] = [
  "1920x1080",
  "1280x720",
  "1080x1920",
  "1080x1080",
  "free",
];

/** 既定の大きさ (docs/studio.md の表)。 */
export const DEFAULT_OUTPUT_SIZE: OutputSizePreset = "1920x1080";

/** プリセットの幅と高さ。`free` なら `null`。 */
export function presetSize(preset: OutputSizePreset): [number, number] | null {
  if (preset === "free") return null;
  const [w, h] = preset.split("x").map(Number);
  return [w ?? 0, h ?? 0];
}

/** 大きさに一致するプリセット。無ければ `free`。 */
export function presetOf(width: number, height: number): OutputSizePreset {
  return OUTPUT_SIZE_PRESETS.find((p) => p === `${width}x${height}`) ?? "free";
}

function read(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function write(key: string, value: string): void {
  try {
    localStorage.setItem(key, value);
  } catch {
    // 覚えられなくても動く (次に起動したときは既定に戻る)。
  }
}

export function loadLocalePreference(): LocalePreference {
  const v = read(LOCALE_STORAGE_KEY);
  return v === "system" || isLocale(v) ? v : "system";
}

export function saveLocalePreference(value: LocalePreference): void {
  write(LOCALE_STORAGE_KEY, value);
}

/** プロファイルに書いたプリセットを読む。知らない値なら既定。 */
export function parseOutputSize(value: string | null | undefined): OutputSizePreset {
  return OUTPUT_SIZE_PRESETS.find((p) => p === value) ?? DEFAULT_OUTPUT_SIZE;
}

export function loadTickRate(): TickRate {
  const v = Number(read(TICK_RATE_STORAGE_KEY));
  return TICK_RATES.find((r) => r === v) ?? DEFAULT_TICK_RATE;
}

export function saveTickRate(value: TickRate): void {
  write(TICK_RATE_STORAGE_KEY, String(value));
}
