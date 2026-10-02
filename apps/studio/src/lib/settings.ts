// 操作ウィンドウが覚える設定 (localStorage) と、出力ウィンドウの大きさのプリセット・刻みの速さ (docs/studio.md#出力ウィンドウ)。
// プロファイル (T0802) ができたら、出力の設定はプロファイルへ移す。
import { isLocale, type LocalePreference } from "@ugoe/i18n";

/** 画面の言語の設定のキー。 */
export const LOCALE_STORAGE_KEY = "ugoe-studio.locale";
/** 出力ウィンドウの大きさのプリセットのキー。 */
export const OUTPUT_SIZE_STORAGE_KEY = "ugoe-studio.output-size";
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

export function loadOutputSize(): OutputSizePreset {
  const v = read(OUTPUT_SIZE_STORAGE_KEY);
  return OUTPUT_SIZE_PRESETS.find((p) => p === v) ?? DEFAULT_OUTPUT_SIZE;
}

export function saveOutputSize(value: OutputSizePreset): void {
  write(OUTPUT_SIZE_STORAGE_KEY, value);
}

export function loadTickRate(): TickRate {
  const v = Number(read(TICK_RATE_STORAGE_KEY));
  return TICK_RATES.find((r) => r === v) ?? DEFAULT_TICK_RATE;
}

export function saveTickRate(value: TickRate): void {
  write(TICK_RATE_STORAGE_KEY, String(value));
}
