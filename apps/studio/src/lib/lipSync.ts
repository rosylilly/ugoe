// マイクの音から口パクの音量 (0〜1) を作る (T0803、docs/studio.md#マイクの口パク)。純粋な関数だけを置く。
//
// 音の大きさ (RMS) を dB にし、-60〜0 dB を 0〜1 に並べた値 (`loudness`) を使う。ノイズの閾値より小さければ 0、
// 閾値から上を 0〜1 に広げて感度の倍率を掛ける。口がぱたぱたしないように、開くときは速く、閉じるときはゆっくり追う。

/** 0 とみなす音の大きさ (dB)。 */
export const FLOOR_DB = -60;
/** 開くときに追う速さ (秒の時定数)。 */
export const ATTACK_SECONDS = 0.03;
/** 閉じるときに追う速さ (秒の時定数)。 */
export const RELEASE_SECONDS = 0.12;

/** 感度と閾値 (プロファイルの `input.sensitivity`・`input.noiseGate`)。 */
export interface LipSyncTuning {
  /** 0〜1。0.5 で等倍、0 で 1/4、1 で 4 倍。 */
  sensitivity: number;
  /** 0〜1 (`loudness` の値)。これより小さな音では口を開かない。 */
  noiseGate: number;
}

function clamp01(v: number): number {
  return Number.isFinite(v) ? Math.min(1, Math.max(0, v)) : 0;
}

/** 波形 (-1〜1) の RMS。 */
export function rms(samples: ArrayLike<number>): number {
  if (samples.length === 0) return 0;
  let sum = 0;
  for (let i = 0; i < samples.length; i++) {
    const v = samples[i] ?? 0;
    sum += v * v;
  }
  return Math.sqrt(sum / samples.length);
}

/** RMS を 0〜1 の大きさにする (-60 dB 以下は 0、0 dB で 1)。メーターの表示にも使う。 */
export function loudness(value: number): number {
  if (!(value > 0)) return 0;
  const db = 20 * Math.log10(value);
  return clamp01((db - FLOOR_DB) / -FLOOR_DB);
}

/** 感度の倍率 (0.5 で 1)。 */
export function gain(sensitivity: number): number {
  return 4 ** (clamp01(sensitivity) * 2 - 1);
}

/** 大きさ (`loudness`) から、口を開く量の目標 (0〜1)。 */
export function targetLevel(level: number, tuning: LipSyncTuning): number {
  const gate = clamp01(tuning.noiseGate);
  const value = clamp01(level);
  if (value <= gate || gate >= 1) return 0;
  return clamp01(((value - gate) / (1 - gate)) * gain(tuning.sensitivity));
}

/** 前の値から目標へ `dt` 秒だけ追う (開くときは速く、閉じるときはゆっくり)。 */
export function follow(previous: number, target: number, dt: number): number {
  if (!(dt > 0)) return previous;
  const tau = target > previous ? ATTACK_SECONDS : RELEASE_SECONDS;
  const k = 1 - Math.exp(-dt / tau);
  return clamp01(previous + (target - previous) * k);
}
