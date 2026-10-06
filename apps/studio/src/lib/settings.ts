// 出力ウィンドウの大きさのプリセットと刻みの速さ (docs/studio.md#出力ウィンドウ)。localStorage のキーは storageKeys.ts。
// 出力の大きさと刻みの速さはプロファイル (lib/profile.ts の `Profile.output`) に持つ (T0802・T0827)。

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

/** プロファイルに書いたプリセットを読む。知らない値なら既定。 */
export function parseOutputSize(value: string | null | undefined): OutputSizePreset {
  return OUTPUT_SIZE_PRESETS.find((p) => p === value) ?? DEFAULT_OUTPUT_SIZE;
}

/** プロファイルに書いた刻みの速さを読む。選べない値なら既定。 */
export function parseTickRate(value: number | null | undefined): TickRate {
  return TICK_RATES.find((r) => r === value) ?? DEFAULT_TICK_RATE;
}
