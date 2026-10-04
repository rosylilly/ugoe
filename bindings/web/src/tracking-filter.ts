// フェイストラッキングの入力の平滑化 (`TrackingFilter`) とキャリブレーション (`captureNeutral`・`calibrateTracking`)。
// どちらも使わなくてよい補助で、ホストが `setTracking` に渡す前に掛ける。DOM と wasm に触れないので、Node でテストする
// (tracking-filter.test.ts)。
//
// docs/spec/avatar-profile.md#実装の範囲 でアプリ (入力を作る側) が行うとしたことのうち、平滑化・途切れた値の扱い・
// キャリブレーションを、依存なしで MIT で書いたもの (ADR 0025 の追記「平滑化とキャリブレーション (T0413)」)。
// GPL の packages/face-tracking は読まずに、One Euro フィルタの論文の定義から書いた:
// Casiez, Roussel, Vogel. "1€ Filter: A Simple Speed-based Low-pass Filter for Noisy Input in Interactive Systems."
// CHI 2012. https://gery.casiez.net/1euro/

import type { TrackingBody, TrackingInput, TrackingRotation } from "./tracking.js";

/** 1 つの値の One Euro フィルタの設定。 */
export interface OneEuroOptions {
  /** 止まっているときのカットオフ周波数 (Hz)。小さいほど揺れが減り、遅れが増える。 */
  minCutoff?: number;
  /** 速さ (単位/秒) に掛けてカットオフに足す係数。大きいほど速い動きの遅れが減る。 */
  beta?: number;
  /** 速さを平滑化するカットオフ周波数 (Hz)。 */
  derivativeCutoff?: number;
}

/** `TrackingFilter` の設定。回転 (度) とブレンドシェイプ・肩 (0〜1) は単位が違うので、別に決める。 */
export interface TrackingFilterOptions {
  /** 頭と体の回転 (度)。既定は `{ minCutoff: 1, beta: 0.05, derivativeCutoff: 1 }`。 */
  rotation?: OneEuroOptions;
  /** ブレンドシェイプと肩の上下 (0〜1)。既定は `{ minCutoff: 1, beta: 2, derivativeCutoff: 1 }`。 */
  blendshapes?: OneEuroOptions;
}

const ROTATION_DEFAULTS: Required<OneEuroOptions> = {
  minCutoff: 1,
  beta: 0.05,
  derivativeCutoff: 1,
};
const BLENDSHAPE_DEFAULTS: Required<OneEuroOptions> = {
  minCutoff: 1,
  beta: 2,
  derivativeCutoff: 1,
};
/** 時刻が進まなかった・戻ったときに使う間隔 (秒)。 */
const FALLBACK_DT = 1 / 60;

/** カットオフ周波数 `cutoff` (Hz) の 1 次のローパスで、間隔 `dt` (秒) のときの重み。 */
function smoothingFactor(cutoff: number, dt: number): number {
  const tau = 1 / (2 * Math.PI * cutoff);
  return 1 / (1 + tau / dt);
}

/** 設定の省略した値と `undefined` の値を既定値にする (`{ ...defaults, ...options }` では `undefined` が既定値を消す)。 */
function withDefaults(
  defaults: Required<OneEuroOptions>,
  options: OneEuroOptions | undefined,
): Required<OneEuroOptions> {
  return {
    minCutoff: options?.minCutoff ?? defaults.minCutoff,
    beta: options?.beta ?? defaults.beta,
    derivativeCutoff: options?.derivativeCutoff ?? defaults.derivativeCutoff,
  };
}

/** 1 つの値の One Euro フィルタ。 */
class OneEuro {
  readonly #options: Required<OneEuroOptions>;
  #value = 0;
  #speed = 0;
  #started = false;

  constructor(options: Required<OneEuroOptions>) {
    this.#options = options;
  }

  apply(x: number, dt: number): number {
    if (!this.#started) {
      this.#started = true;
      this.#value = x;
      return x;
    }
    const { minCutoff, beta, derivativeCutoff } = this.#options;
    const rawSpeed = (x - this.#value) / dt;
    this.#speed += smoothingFactor(derivativeCutoff, dt) * (rawSpeed - this.#speed);
    const cutoff = minCutoff + beta * Math.abs(this.#speed);
    this.#value += smoothingFactor(cutoff, dt) * (x - this.#value);
    return this.#value;
  }
}

/**
 * トラッキングの入力を値ごとに One Euro フィルタで平滑化する (止まっているときの揺れを抑え、速い動きには遅れを小さくする)。
 *
 * ```js
 * const filter = new TrackingFilter();
 * const input = trackingFromFaceLandmarker(result);
 * if (input) avatar.setTracking(filter.apply(input, performance.now()));
 * else filter.reset(); // 顔を見失ったら忘れる
 * ```
 *
 * 入力に無い値 (ブレンドシェイプの名前・体の値) のフィルタは忘れ、次に来たときはその値から始める。
 */
export class TrackingFilter {
  readonly #rotation: Required<OneEuroOptions>;
  readonly #blendshapes: Required<OneEuroOptions>;
  #filters = new Map<string, OneEuro>();
  #lastTime: number | null = null;

  constructor(options: TrackingFilterOptions = {}) {
    this.#rotation = withDefaults(ROTATION_DEFAULTS, options.rotation);
    this.#blendshapes = withDefaults(BLENDSHAPE_DEFAULTS, options.blendshapes);
  }

  /** すべての値のフィルタを忘れる (顔を見失ったとき・トラッキングをやめたとき)。 */
  reset(): void {
    this.#filters.clear();
    this.#lastTime = null;
  }

  /**
   * 入力を平滑化した新しい入力を返す (`input` は変えない)。
   * `timeMs` は入力の時刻 (ミリ秒。`performance.now()` や `detectForVideo` に渡した時刻)。
   */
  apply(input: TrackingInput, timeMs: number): TrackingInput {
    const dt =
      this.#lastTime !== null && timeMs > this.#lastTime
        ? (timeMs - this.#lastTime) / 1000
        : FALLBACK_DT;
    this.#lastTime = timeMs;
    const next = new Map<string, OneEuro>();
    const filter = (key: string, value: number, options: Required<OneEuroOptions>): number => {
      const f = this.#filters.get(key) ?? new OneEuro(options);
      next.set(key, f);
      return f.apply(value, dt);
    };
    const rotation = (prefix: string, r: TrackingRotation): TrackingRotation => ({
      yaw: filter(`${prefix}.yaw`, r.yaw, this.#rotation),
      pitch: filter(`${prefix}.pitch`, r.pitch, this.#rotation),
      roll: filter(`${prefix}.roll`, r.roll, this.#rotation),
    });
    const optional = (key: string, value: number | undefined, options: Required<OneEuroOptions>) =>
      value === undefined ? undefined : filter(key, value, options);

    const blendshapes: Record<string, number> = {};
    for (const [name, value] of Object.entries(input.blendshapes)) {
      blendshapes[name] = filter(`blendshape.${name}`, value, this.#blendshapes);
    }
    const output: TrackingInput = { blendshapes, head: rotation("head", input.head) };
    const { body } = input;
    if (body) {
      output.body = withoutUndefined({
        yaw: optional("body.yaw", body.yaw, this.#rotation),
        pitch: optional("body.pitch", body.pitch, this.#rotation),
        roll: optional("body.roll", body.roll, this.#rotation),
        shoulderLeft: optional("body.shoulderLeft", body.shoulderLeft, this.#blendshapes),
        shoulderRight: optional("body.shoulderRight", body.shoulderRight, this.#blendshapes),
      });
    }
    this.#filters = next;
    return output;
  }
}

/** 値が `undefined` のキーを除く (`body` の省略を「書き込まない」のまま保つため)。 */
function withoutUndefined(body: TrackingBody): TrackingBody {
  const out: TrackingBody = {};
  for (const [key, value] of Object.entries(body) as [keyof TrackingBody, number | undefined][]) {
    if (value !== undefined) out[key] = value;
  }
  return out;
}

/** 利用者の中立の顔と姿勢 (`captureNeutral` で作り、`calibrateTracking` に渡す)。 */
export interface TrackingNeutral {
  head: TrackingRotation;
  /** 中立のブレンドシェイプの値 (0〜1)。 */
  blendshapes: Record<string, number>;
  body?: TrackingBody;
}

/**
 * 中立の顔 (正面を向いた、力を抜いた表情) の入力から、キャリブレーションの基準を作る。
 * 配列を渡すと値ごとに平均する (数フレーム分を渡すと揺れの影響が減る)。空の配列なら `null`。
 */
export function captureNeutral(
  inputs: TrackingInput | readonly TrackingInput[],
): TrackingNeutral | null {
  const list = Array.isArray(inputs)
    ? (inputs as readonly TrackingInput[])
    : [inputs as TrackingInput];
  if (list.length === 0) return null;
  const sums = new Map<string, { sum: number; count: number }>();
  const add = (key: string, value: number | undefined) => {
    if (value === undefined) return;
    const s = sums.get(key) ?? { sum: 0, count: 0 };
    s.sum += value;
    s.count += 1;
    sums.set(key, s);
  };
  for (const input of list) {
    add("head.yaw", input.head.yaw);
    add("head.pitch", input.head.pitch);
    add("head.roll", input.head.roll);
    for (const [name, value] of Object.entries(input.blendshapes)) add(`blendshape.${name}`, value);
    for (const key of BODY_KEYS) add(`body.${key}`, input.body?.[key]);
  }
  const mean = (key: string) => {
    const s = sums.get(key);
    return s ? s.sum / s.count : undefined;
  };
  const blendshapes: Record<string, number> = {};
  for (const key of sums.keys()) {
    if (key.startsWith("blendshape."))
      blendshapes[key.slice("blendshape.".length)] = mean(key) ?? 0;
  }
  const neutral: TrackingNeutral = {
    head: {
      yaw: mean("head.yaw") ?? 0,
      pitch: mean("head.pitch") ?? 0,
      roll: mean("head.roll") ?? 0,
    },
    blendshapes,
  };
  const body = withoutUndefined(Object.fromEntries(BODY_KEYS.map((k) => [k, mean(`body.${k}`)])));
  if (Object.keys(body).length > 0) neutral.body = body;
  return neutral;
}

const BODY_KEYS = ["yaw", "pitch", "roll", "shoulderLeft", "shoulderRight"] as const;

/** 角度の差を −180〜180 度にする。 */
function angleDelta(value: number, neutral: number): number {
  const d = (value - neutral) % 360;
  return d > 180 ? d - 360 : d <= -180 ? d + 360 : d;
}

/**
 * 入力から利用者の中立を差し引く (`input` は変えない)。
 *
 * - 頭と体の回転: 中立の角度を引く。
 * - ブレンドシェイプ: 中立の値 `n` を 0 に、1 を 1 にして引き伸ばす (`(v − n) / (1 − n)` を 0〜1 に収める)。
 *   力を抜いても少し開いている口や細めた目を 0 として扱うため。中立に無い名前はそのまま。
 * - 体のピッチと肩の上下: 中立の値を引く。カメラの高さや角度・体格で基準が大きく変わるので、
 *   **中立にその値が無ければ渡さない** (体のヨーとロールは中立に無ければそのまま)。
 */
export function calibrateTracking(input: TrackingInput, neutral: TrackingNeutral): TrackingInput {
  const blendshapes: Record<string, number> = {};
  for (const [name, value] of Object.entries(input.blendshapes)) {
    const n = neutral.blendshapes[name];
    blendshapes[name] =
      n === undefined || n >= 1 ? value : Math.min(1, Math.max(0, (value - n) / (1 - n)));
  }
  const { head } = input;
  const output: TrackingInput = {
    blendshapes,
    head: {
      yaw: angleDelta(head.yaw, neutral.head.yaw),
      pitch: angleDelta(head.pitch, neutral.head.pitch),
      roll: angleDelta(head.roll, neutral.head.roll),
    },
  };
  const { body } = input;
  if (body) {
    const n = neutral.body ?? {};
    /** 中立があれば差し引く。無ければ `required` のときは渡さず、そうでなければそのまま。 */
    const angle = (value: number | undefined, base: number | undefined, required: boolean) => {
      if (value === undefined) return undefined;
      if (base === undefined) return required ? undefined : value;
      return angleDelta(value, base);
    };
    const shoulder = (value: number | undefined, base: number | undefined) =>
      value === undefined || base === undefined ? undefined : value - base;
    output.body = withoutUndefined({
      yaw: angle(body.yaw, n.yaw, false),
      pitch: angle(body.pitch, n.pitch, true),
      roll: angle(body.roll, n.roll, false),
      shoulderLeft: shoulder(body.shoulderLeft, n.shoulderLeft),
      shoulderRight: shoulder(body.shoulderRight, n.shoulderRight),
    });
  }
  return output;
}
