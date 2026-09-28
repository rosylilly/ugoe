// 平滑化 (One Euro フィルタ)。docs/spec/avatar-profile.md#実装の範囲 でホストの責務とされたもの。
//
// One Euro フィルタは、動きが遅いときは強く、速いときは弱く平滑化する (揺れを抑えつつ遅れを小さくする)。
// 出典: Casiez, Roussel, Vogel, "1€ Filter: A Simple Speed-based Low-pass Filter for Noisy Input in
// Interactive Systems" (CHI 2012), https://gery.casiez.net/1euro/
import type { TrackingFrame } from "./frame";

function alpha(cutoff: number, dt: number): number {
  const tau = 1 / (2 * Math.PI * cutoff);
  return 1 / (1 + tau / dt);
}

export class OneEuroFilter {
  #x: number | null = null;
  #dx = 0;

  /**
   * @param minCutoff 止まっているときの遮断周波数 (Hz)。小さいほど強く平滑化する。
   * @param beta 速さに応じて遮断周波数を上げる係数 (値の単位/秒 あたりの Hz)。
   * @param dCutoff 速さの推定に使う遮断周波数 (Hz)。
   */
  constructor(
    public minCutoff: number,
    public beta: number,
    public dCutoff = 1,
  ) {}

  /** `dt` 秒後の値 `x` を入れ、平滑化した値を返す。 */
  filter(x: number, dt: number): number {
    if (this.#x === null || !(dt > 0)) {
      this.#x = x;
      this.#dx = 0;
      return x;
    }
    const dx = (x - this.#x) / dt;
    this.#dx += alpha(this.dCutoff, dt) * (dx - this.#dx);
    const cutoff = this.minCutoff + this.beta * Math.abs(this.#dx);
    this.#x += alpha(cutoff, dt) * (x - this.#x);
    return this.#x;
  }

  reset(): void {
    this.#x = null;
    this.#dx = 0;
  }
}

/** 強さ 0 の遮断周波数 (Hz)。60fps ではほとんど平滑化しない。 */
const MIN_CUTOFF_WEAK = 30;
/** 強さ 1 の遮断周波数 (Hz)。 */
const MIN_CUTOFF_STRONG = 0.3;
/** 速さの係数。頭の回転 (度/秒) とブレンドシェイプ (1/秒) で値の大きさが違うので分ける。 */
const BETA_ANGLE = 0.02;
const BETA_SCORE = 0.5;

/** 強さ (0〜1) から止まっているときの遮断周波数を決める (対数で補間する)。 */
export function minCutoffForStrength(strength: number): number {
  const s = Math.min(1, Math.max(0, strength));
  return Math.exp(Math.log(MIN_CUTOFF_WEAK) * (1 - s) + Math.log(MIN_CUTOFF_STRONG) * s);
}

/** フレームのすべての値 (頭の回転とブレンドシェイプ) を、値ごとの One Euro フィルタで平滑化する。 */
export class FrameSmoother {
  /** 強さ (0〜1)。0 なら平滑化しない。 */
  strength: number;
  #filters = new Map<string, OneEuroFilter>();

  constructor(strength = 0.5) {
    this.strength = strength;
  }

  #filter(key: string, value: number, dt: number, beta: number): number {
    let f = this.#filters.get(key);
    if (f === undefined) {
      f = new OneEuroFilter(0, beta);
      this.#filters.set(key, f);
    }
    f.minCutoff = minCutoffForStrength(this.strength);
    const y = f.filter(value, dt);
    return this.strength > 0 ? y : value;
  }

  smooth(frame: TrackingFrame, dt: number): TrackingFrame {
    const blendShapes: Record<string, number> = {};
    for (const [name, value] of Object.entries(frame.blendShapes)) {
      blendShapes[name] = this.#filter(name, value, dt, BETA_SCORE);
    }
    return {
      head: {
        yaw: this.#filter("#yaw", frame.head.yaw, dt, BETA_ANGLE),
        pitch: this.#filter("#pitch", frame.head.pitch, dt, BETA_ANGLE),
        roll: this.#filter("#roll", frame.head.roll, dt, BETA_ANGLE),
      },
      blendShapes,
    };
  }

  /** 途切れた後や入力を切り替えたときに、過去の値を忘れる。 */
  reset(): void {
    this.#filters.clear();
  }
}
