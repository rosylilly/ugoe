// 操作ウィンドウが覚える設定 (localStorage。言語・テーマ) と、出力ウィンドウの大きさのプリセットと刻みの速さ (docs/studio.md#出力ウィンドウ)。
// 出力の大きさと刻みの速さはプロファイル (lib/profile.ts の `Profile.output`) に持つ (T0802・T0827)。
import { isLocale, type LocalePreference } from "@ugoe/i18n";
import { PlayerStorage } from "@ugoe/player";

/** 画面の言語の設定のキー。 */
export const LOCALE_STORAGE_KEY = "ugoe-studio.locale";
/** テーマの設定のキー (@ugoe/ui の initTheme に渡す。index.html の <head> のスクリプトも同じキーを読む)。出力ウィンドウには効かせない。 */
export const THEME_STORAGE_KEY = "ugoe-studio.theme";
/**
 * プロファイルへ移す前に localStorage に覚えていた設定のキー (刻みの速さと仮想カメラ)。引き継がずに消す
 * (T0827。docs/studio.md#プロファイルの形式)。
 */
export const LEGACY_STORAGE_KEYS = ["ugoe-studio.tick-rate", "ugoe-studio.virtual-camera"] as const;
/**
 * 出力ウィンドウの Player (@ugoe/player) が覚えるもののキーの接頭辞 (T0930)。受け手のモードではほぼ何も覚えないが、
 * ビューアの `ugoe-viewer` とは分ける。
 */
export const PLAYER_STORAGE_PREFIX = "ugoe-studio.player";
/**
 * T0930 より前に、出力ウィンドウの Player がビューアの接頭辞で覚えていたもの (待機モーションの選択など)。
 * 受け手は使わないので、引き継がずに消す (スタジオとビューアは別のアプリで、localStorage も別)。
 */
export const LEGACY_PLAYER_STORAGE_PREFIX = "ugoe-viewer";

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

/** プロファイルに書いた刻みの速さを読む。選べない値なら既定。 */
export function parseTickRate(value: number | null | undefined): TickRate {
  return TICK_RATES.find((r) => r === value) ?? DEFAULT_TICK_RATE;
}

/** 前に覚えていて、今は使わない設定を消す (`LEGACY_STORAGE_KEYS` と `LEGACY_PLAYER_STORAGE_PREFIX`)。 */
export function forgetLegacySettings(): void {
  for (const key of LEGACY_STORAGE_KEYS) {
    try {
      localStorage.removeItem(key);
    } catch {
      // 消せなくても読まないので害は無い。
    }
  }
  new PlayerStorage(LEGACY_PLAYER_STORAGE_PREFIX).clear();
}
