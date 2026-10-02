// 平滑化 (One Euro フィルタ)。docs/spec/avatar-profile.md#実装の範囲 でホストの責務とされたもの。
//
// One Euro フィルタは、動きが遅いときは強く、速いときは弱く平滑化する (揺れを抑えつつ遅れを小さくする)。
// 出典: Casiez, Roussel, Vogel, "1€ Filter: A Simple Speed-based Low-pass Filter for Noisy Input in
// Interactive Systems" (CHI 2012), https://gery.casiez.net/1euro/
import { BODY_KEYS, type BodyPose, type TrackingFrame } from "./frame";

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
/** 速さの係数。頭の回転 (度/秒) とブレンドシェイプ (1/秒) で値の大きさが違うので分ける。肩の上下は後者と同じ。 */
const BETA_ANGLE = 0.02;
const BETA_SCORE = 0.5;
const BODY_BETA: Record<string, number> = {
  yaw: BETA_ANGLE,
  pitch: BETA_ANGLE,
  roll: BETA_ANGLE,
  shoulderLeft: BETA_SCORE,
  shoulderRight: BETA_SCORE,
};

/** 強さ (0〜1) から止まっているときの遮断周波数を決める (対数で補間する)。 */
export function minCutoffForStrength(strength: number): number {
  const s = Math.min(1, Math.max(0, strength));
  return Math.exp(Math.log(MIN_CUTOFF_WEAK) * (1 - s) + Math.log(MIN_CUTOFF_STRONG) * s);
}

/**
 * フレームのすべての値 (頭の回転・ブレンドシェイプ・体) を、値ごとの One Euro フィルタで平滑化する。
 * 体の値が途切れたら (`null`)、その値のフィルタは忘れる (戻ったときに古い値から追いかけないように)。
 */
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
    const out: TrackingFrame = {
      head: {
        yaw: this.#filter("#yaw", frame.head.yaw, dt, BETA_ANGLE),
        pitch: this.#filter("#pitch", frame.head.pitch, dt, BETA_ANGLE),
        roll: this.#filter("#roll", frame.head.roll, dt, BETA_ANGLE),
      },
      blendShapes,
    };
    if (frame.body) {
      const body = { ...frame.body };
      for (const key of BODY_KEYS) {
        const v = frame.body[key];
        const id = `#body.${key}`;
        if (v === null) this.#filters.delete(id);
        else body[key] = this.#filter(id, v, dt, BODY_BETA[key] ?? BETA_ANGLE);
      }
      out.body = body satisfies BodyPose;
    } else {
      for (const key of BODY_KEYS) this.#filters.delete(`#body.${key}`);
    }
    return out;
  }

  /** 途切れた後や入力を切り替えたときに、過去の値を忘れる。 */
  reset(): void {
    this.#filters.clear();
  }
}

/**
 * 平滑化に渡す `dt` を、前の値 (検出・再生) から測る (T0812)。処理を回す刻みは値の出る間隔より細かいことがあり
 * (60/s の刻みと 30fps のカメラ、検出が刻みより重いときなど)、刻みの間隔で測ると平滑化が実際より遅れる。
 */
export class FrameInterval {
  #last: number | null = null;

  /** @param maxDt 1 つの値で進める時間の上限 (秒)。止まっていた後に大きく飛ばないように。 */
  constructor(readonly maxDt: number) {}

  /** 時刻 `now` (ミリ秒) に値が出た。前の値からの秒数を返す (最初の値は 0)。 */
  next(now: number): number {
    const dt = this.#last === null ? 0 : Math.min((now - this.#last) / 1000, this.maxDt);
    this.#last = now;
    return dt;
  }

  /** 入力を切り替えたときに、前の値の時刻を忘れる。 */
  reset(): void {
    this.#last = null;
  }
}
