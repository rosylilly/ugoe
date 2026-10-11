// 利用者の調整 (首振りの感度と遊び・瞼・視線・口・眉の境目。T0877・T0879)。キャリブレーションと同じく
// docs/spec/avatar-profile.md#実装の範囲 でアプリの責務とされたもの。キャリブレーションの後・平滑化の前に当てる。
//
// 調整は配信者の顔・カメラ・部屋に合わせるもので、モデルの `avatar.tracking` の係数 (モデルを作る人が決める) とは別。
// 既定値 (`DEFAULT_TUNING`) のままなら、フレームをそのまま返す (出力はビット単位で変わらない)。
// 型・既定値・収め直し (`sanitizeTuning`) はここに置き、調整を覚えるアプリ (スタジオのプロファイル・ビューアの設定) はこれを使う。
// このファイルは型のほかに何も import しない (ui-audit の偽のホストが Node で `@ugoe/face-tracking/tuning` を読む)。
import type { HeadPose, TrackingFrame } from "./frame";

/** 頭の回転の 1 つの軸の調整。 */
export interface AxisTuning {
  /** 感度 (倍率)。遊びを引いた後の角度に掛ける。 */
  gain: number;
  /** 遊び (度)。中立のまわりのこの角度の中は 0 にし、外は遊びの分を引いて続けてつなぐ。 */
  deadZone: number;
}

/** ブレンドシェイプの値の境目。`low` 以下を 0、`high` 以上を 1 とし、その間を引き伸ばす。 */
export interface RangeTuning {
  low: number;
  high: number;
}

/** 頭の回転の軸。 */
export const TUNING_AXES = ["yaw", "pitch", "roll"] as const satisfies readonly (keyof HeadPose)[];
export type TuningAxis = (typeof TUNING_AXES)[number];

/**
 * 境目を調整できるブレンドシェイプのまとまりと、当てる名前。まとまりの中は 1 つの組で一緒に動かす
 * (瞼だけは「左右を別々にする」(`eyeBlinkSeparate`) で、`eyeBlink` を左目、`eyeBlinkRight` を右目に当てられる)。
 * ほかの入力へ広げるときは、ここに名前を足し、`TrackingTuning` と既定値に項目を足す。
 * 標準のパラメータの式は crates/ugoe-motion/src/tracking.rs。
 */
export const TUNING_RANGES = {
  /** 瞼 (目の閉じ。0 が開いた、1 が閉じた。`EyeLOpen`・`EyeROpen` の元)。 */
  eyeBlink: ["eyeBlinkLeft", "eyeBlinkRight"],
  /** 視線 (上下左右へ見る量。`EyeBallX`・`EyeBallY` の元)。上の境目は「見きった」と見なす値。 */
  eyeLook: [
    "eyeLookInLeft",
    "eyeLookOutLeft",
    "eyeLookUpLeft",
    "eyeLookDownLeft",
    "eyeLookInRight",
    "eyeLookOutRight",
    "eyeLookUpRight",
    "eyeLookDownRight",
  ],
  /** 口の開き (`MouthOpenY` = `max(0, jawOpen − mouthClose)` の元)。 */
  jawOpen: ["jawOpen"],
  /** 口の閉じ (`MouthOpenY` で `jawOpen` から引く)。上の境目は「閉じた」と見なす値。 */
  mouthClose: ["mouthClose"],
  /** 口の形 (笑みと口角の下がり。`MouthForm` の元)。 */
  mouthForm: ["mouthSmileLeft", "mouthSmileRight", "mouthFrownLeft", "mouthFrownRight"],
  /** 眉 (上げと下げ。`BrowLY`・`BrowRY` などの元)。 */
  brow: ["browInnerUp", "browOuterUpLeft", "browOuterUpRight", "browDownLeft", "browDownRight"],
} as const satisfies Record<string, readonly string[]>;
export type TuningRange = keyof typeof TUNING_RANGES;
/** 境目の組を持つ項目 (`TUNING_RANGES` のまとまりと、左右を別々にしたときの右目の瞼)。 */
export type TuningRangeKey = TuningRange | "eyeBlinkRight";

/** 利用者の調整 (スタジオのプロファイルの `input.tuning`。apps/studio/src-tauri/src/input.rs の `TrackingTuning`)。 */
export interface TrackingTuning {
  yaw: AxisTuning;
  pitch: AxisTuning;
  roll: AxisTuning;
  /** 瞼。`eyeBlinkSeparate` が切なら両目、入なら左目 (利用者から見た左。`swapSides` の後の左右)。 */
  eyeBlink: RangeTuning;
  /** 瞼の左右を別々にするか。 */
  eyeBlinkSeparate: boolean;
  /** 右目の瞼。`eyeBlinkSeparate` が入のときだけ使う。 */
  eyeBlinkRight: RangeTuning;
  eyeLook: RangeTuning;
  jawOpen: RangeTuning;
  mouthClose: RangeTuning;
  mouthForm: RangeTuning;
  brow: RangeTuning;
}

/** 感度の範囲。 */
export const GAIN_MIN = 0;
export const GAIN_MAX = 4;
/** 遊びの範囲 (度)。 */
export const DEAD_ZONE_MAX = 20;
/** 境目の間の最小の幅 (これより狭いと、わずかな揺れで 0 と 1 を行き来する)。 */
export const RANGE_MIN_SPAN = 0.05;

const DEFAULT_AXIS: AxisTuning = { gain: 1, deadZone: 0 };
const DEFAULT_RANGE: RangeTuning = { low: 0, high: 1 };

/** 既定値 (今までの振る舞いと同じ。倍率 1・遊び 0・境目 0 と 1)。 */
export const DEFAULT_TUNING: TrackingTuning = Object.freeze({
  yaw: DEFAULT_AXIS,
  pitch: DEFAULT_AXIS,
  roll: DEFAULT_AXIS,
  eyeBlink: DEFAULT_RANGE,
  eyeBlinkSeparate: false,
  eyeBlinkRight: DEFAULT_RANGE,
  eyeLook: DEFAULT_RANGE,
  jawOpen: DEFAULT_RANGE,
  mouthClose: DEFAULT_RANGE,
  mouthForm: DEFAULT_RANGE,
  brow: DEFAULT_RANGE,
});

function isObject(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null;
}

/** 数で範囲の中ならそのまま、範囲の外なら端に寄せる。数でなければ (NaN・無限大を含む) `fallback`。 */
function inRange(v: unknown, min: number, max: number, fallback: number): number {
  return typeof v === "number" && Number.isFinite(v) ? Math.min(max, Math.max(min, v)) : fallback;
}

/** 軸の調整を範囲に収める。 */
export function sanitizeAxis(value: unknown): AxisTuning {
  if (!isObject(value)) return { ...DEFAULT_AXIS };
  return {
    gain: inRange(value.gain, GAIN_MIN, GAIN_MAX, DEFAULT_AXIS.gain),
    deadZone: inRange(value.deadZone, 0, DEAD_ZONE_MAX, DEFAULT_AXIS.deadZone),
  };
}

/** 境目を 0〜1 に収める。間が `RANGE_MIN_SPAN` より狭い (下が上以上を含む) なら既定に戻す。 */
export function sanitizeRange(value: unknown): RangeTuning {
  if (!isObject(value)) return { ...DEFAULT_RANGE };
  const low = inRange(value.low, 0, 1, DEFAULT_RANGE.low);
  const high = inRange(value.high, 0, 1, DEFAULT_RANGE.high);
  // 浮動小数点の誤差 (0.35 − 0.3 など) で、画面で決めた最小の幅を弾かないように少し緩める。
  if (high - low < RANGE_MIN_SPAN - 1e-9) return { ...DEFAULT_RANGE };
  return { low, high };
}

/** 調整を範囲に収める (知らない項目は捨て、無い・壊れた項目は既定値にする)。 */
export function sanitizeTuning(value: unknown): TrackingTuning {
  const v = isObject(value) ? value : {};
  return {
    yaw: sanitizeAxis(v.yaw),
    pitch: sanitizeAxis(v.pitch),
    roll: sanitizeAxis(v.roll),
    eyeBlink: sanitizeRange(v.eyeBlink),
    eyeBlinkSeparate: typeof v.eyeBlinkSeparate === "boolean" ? v.eyeBlinkSeparate : false,
    eyeBlinkRight: sanitizeRange(v.eyeBlinkRight),
    eyeLook: sanitizeRange(v.eyeLook),
    jawOpen: sanitizeRange(v.jawOpen),
    mouthClose: sanitizeRange(v.mouthClose),
    mouthForm: sanitizeRange(v.mouthForm),
    brow: sanitizeRange(v.brow),
  };
}

/**
 * 瞼の左右を別々にするかを切り替えた調整。入れるときは右目に今の (両目の) 境目を写す (切り替えただけでは出力が変わらない)。
 * 切るときは右目の境目を残す (使わない)。
 */
export function setEyeBlinkSeparate(tuning: TrackingTuning, separate: boolean): TrackingTuning {
  if (separate === tuning.eyeBlinkSeparate) return tuning;
  return separate
    ? { ...tuning, eyeBlinkSeparate: true, eyeBlinkRight: { ...tuning.eyeBlink } }
    : { ...tuning, eyeBlinkSeparate: false };
}

/**
 * 境目の片方を動かしたときの組 (画面のスライダーから)。もう片方との間が `RANGE_MIN_SPAN` より狭くなるなら、もう片方を押して
 * 間を保つ (動かした方は利用者の決めた値のまま。下の境目は `1 − RANGE_MIN_SPAN` まで、上の境目は `RANGE_MIN_SPAN` から)。
 */
export function moveRangeBound(
  range: RangeTuning,
  bound: "low" | "high",
  value: number,
): RangeTuning {
  if (bound === "low") {
    const low = inRange(value, 0, 1 - RANGE_MIN_SPAN, range.low);
    return { low, high: Math.max(range.high, low + RANGE_MIN_SPAN) };
  }
  const high = inRange(value, RANGE_MIN_SPAN, 1, range.high);
  return { low: Math.min(range.low, high - RANGE_MIN_SPAN), high };
}

function isDefaultAxis(a: AxisTuning): boolean {
  return a.gain === 1 && a.deadZone === 0;
}

function isDefaultRange(r: RangeTuning): boolean {
  return r.low === 0 && r.high === 1;
}

/**
 * 当てる境目の組と、当てるブレンドシェイプの名前。瞼は、左右を別々にしているときだけ左目と右目に分ける
 * (切のときの `eyeBlinkRight` は使わない)。
 */
export function tuningRanges(tuning: TrackingTuning): [RangeTuning, readonly string[]][] {
  return (Object.keys(TUNING_RANGES) as TuningRange[]).flatMap(
    (key): [RangeTuning, readonly string[]][] =>
      key === "eyeBlink" && tuning.eyeBlinkSeparate
        ? [
            [tuning.eyeBlink, ["eyeBlinkLeft"]],
            [tuning.eyeBlinkRight, ["eyeBlinkRight"]],
          ]
        : [[tuning[key], TUNING_RANGES[key]]],
  );
}

/** 当てても何も変わらないか (左右を別々にしていても、両方の境目が既定なら変わらない)。 */
function isIdentityTuning(tuning: TrackingTuning): boolean {
  return (
    TUNING_AXES.every((axis) => isDefaultAxis(tuning[axis])) &&
    tuningRanges(tuning).every(([range]) => isDefaultRange(range))
  );
}

/** 既定値と同じか (「既定に戻す」で変わるものが無いか。瞼の左右を別々にしているなら既定ではない)。 */
export function isDefaultTuning(tuning: TrackingTuning): boolean {
  return !tuning.eyeBlinkSeparate && isIdentityTuning(tuning);
}

/** 角度に遊びと感度を当てる (遊びの中は 0、外は遊びの分を引いて倍率を掛ける。符号は保つ)。 */
export function tuneAngle(value: number, axis: AxisTuning): number {
  if (isDefaultAxis(axis)) return value;
  const magnitude = Math.max(0, Math.abs(value) - axis.deadZone) * axis.gain;
  return value < 0 && magnitude > 0 ? -magnitude : magnitude;
}

/** 値に境目を当てる (`low` 以下を 0、`high` 以上を 1、間を引き伸ばす)。 */
export function tuneScore(value: number, range: RangeTuning): number {
  if (isDefaultRange(range)) return value;
  return Math.min(1, Math.max(0, (value - range.low) / (range.high - range.low)));
}

/**
 * フレームに調整を当てる。キャリブレーションの後・平滑化の前に使う (tracker.svelte.ts)。
 * 左右の入れ替え (`swapSides`) は検出の直後に済んでいるので、調整は利用者から見た左右 (出力の左右) に当たる。
 * 体 (`body`) と、`TUNING_RANGES` に無いブレンドシェイプは変えない。調整は `sanitizeTuning` を通してから使う
 * (範囲の外や NaN を含む値をそのまま渡さない。渡しても既定値として扱う)。
 */
export function applyTuning(frame: TrackingFrame, tuning: TrackingTuning): TrackingFrame {
  const t = sanitizeTuning(tuning);
  if (isIdentityTuning(t)) return frame;
  const blendShapes = { ...frame.blendShapes };
  for (const [range, names] of tuningRanges(t)) {
    if (isDefaultRange(range)) continue;
    for (const name of names) {
      const v = blendShapes[name];
      if (v !== undefined) blendShapes[name] = tuneScore(v, range);
    }
  }
  const out: TrackingFrame = {
    head: {
      yaw: tuneAngle(frame.head.yaw, t.yaw),
      pitch: tuneAngle(frame.head.pitch, t.pitch),
      roll: tuneAngle(frame.head.roll, t.roll),
    },
    blendShapes,
  };
  if (frame.body) out.body = frame.body;
  return out;
}
