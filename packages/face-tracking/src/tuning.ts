// 利用者の調整 (首振りの感度と遊び・瞼・視線・目の見開き・口・頬・眉の境目。T0877・T0879・T0880・T0881・T0882)。キャリブレーションと同じく
// docs/spec/avatar-profile.md#実装の範囲 でアプリの責務とされたもの。キャリブレーションの後・平滑化の前に当てる。
//
// 調整は配信者の顔・カメラ・部屋に合わせるもので、モデルの `avatar.tracking` の係数 (モデルを作る人が決める) とは別。
// 既定値 (`DEFAULT_TUNING`) のままなら、フレームをそのまま返す (出力はビット単位で変わらない)。
// 型・既定値・収め直し (`sanitizeTuning`) はここに置き、調整を覚えるアプリ (スタジオのプロファイル・ビューアの設定) はこれを使う。
// このファイルは型のほかに何も import しない (ui-audit の偽のホストが Node で `@ugoe/face-tracking/tuning` を読む)。
import type { HeadPose, TrackingFrame } from "./frame";

/**
 * 頭の回転の 1 つの軸の調整。正の向きは「向きの約束」(frame.ts) の符号で、`swapSides` の後の、利用者から見た向き
 * (ヨーは自分の左を向く、ピッチは上を向く、ロールは自分の左へ傾ける)。
 */
export interface AxisTuning {
  /** 感度 (倍率)。遊びを引いた後の角度に掛ける。`separate` が入なら正の向きだけ。 */
  gain: number;
  /** 遊び (度)。中立のまわりのこの角度の中は 0 にし、外は遊びの分を引いて続けてつなぐ。向きに依らず共通。 */
  deadZone: number;
  /** 感度を向きで分けるか (カメラを横に置いて片側だけ振れ幅が小さいとき)。 */
  separate: boolean;
  /** 負の向きの感度。`separate` が入のときだけ使う。 */
  gainNegative: number;
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
 * (`TUNING_SPLITS` のまとまりは、分けるスイッチを入れると左右・向きごとの組を当てられる)。
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
  /** 目の見開き (`EyeLWide`・`EyeRWide` の元)。上の境目は「見開ききった」と見なす値。 */
  eyeWide: ["eyeWideLeft", "eyeWideRight"],
  /** 口の開き (`MouthOpenY` = `max(0, jawOpen − mouthClose)` の元)。 */
  jawOpen: ["jawOpen"],
  /** 口の閉じ (`MouthOpenY` で `jawOpen` から引く)。上の境目は「閉じた」と見なす値。 */
  mouthClose: ["mouthClose"],
  /** 口の形 (笑みと口角の下がり。`MouthForm` の元)。 */
  mouthForm: ["mouthSmileLeft", "mouthSmileRight", "mouthFrownLeft", "mouthFrownRight"],
  /** 口の左右 (口を横へ寄せる量。`MouthX` = `mouthLeft − mouthRight` の元)。 */
  mouthSide: ["mouthLeft", "mouthRight"],
  /** 頬の膨らみ (`CheekPuff` の元)。 */
  cheekPuff: ["cheekPuff"],
  /** 眉 (上げと下げ。`BrowLY`・`BrowRY` などの元)。 */
  brow: ["browInnerUp", "browOuterUpLeft", "browOuterUpRight", "browDownLeft", "browDownRight"],
} as const satisfies Record<string, readonly string[]>;
export type TuningRange = keyof typeof TUNING_RANGES;

/**
 * 左右・向きで分けられるまとまりと、分けたときの組 (向きの名前・境目の項目・当てる名前)。最初の組はまとまりの項目そのものを使い、
 * ほかの組は分けたときだけ使う項目を持つ。まとまりの名前のすべてを、どれか 1 つの組が受け持つ。
 * 左右は利用者から見た左右 (`swapSides` の後)。視線の左右は見る向きで、左へ見るのは左目の外 (`eyeLookOutLeft`) と右目の内
 * (`eyeLookInRight`) (crates/ugoe-motion/src/tracking.rs の `EyeBallX` と同じ向き)。
 */
export const TUNING_SPLITS = {
  /** 瞼の左右 (T0879)。 */
  eyeBlink: [
    { direction: "left", key: "eyeBlink", names: ["eyeBlinkLeft"] },
    { direction: "right", key: "eyeBlinkRight", names: ["eyeBlinkRight"] },
  ],
  /** 視線の上下左右 (T0881)。 */
  eyeLook: [
    { direction: "up", key: "eyeLook", names: ["eyeLookUpLeft", "eyeLookUpRight"] },
    { direction: "down", key: "eyeLookDown", names: ["eyeLookDownLeft", "eyeLookDownRight"] },
    { direction: "left", key: "eyeLookLeft", names: ["eyeLookOutLeft", "eyeLookInRight"] },
    { direction: "right", key: "eyeLookRight", names: ["eyeLookInLeft", "eyeLookOutRight"] },
  ],
  /** 目の見開きの左右 (T0882)。 */
  eyeWide: [
    { direction: "left", key: "eyeWide", names: ["eyeWideLeft"] },
    { direction: "right", key: "eyeWideRight", names: ["eyeWideRight"] },
  ],
  /** 口の形の笑みと口角の下がり (T0881)。 */
  mouthForm: [
    { direction: "smile", key: "mouthForm", names: ["mouthSmileLeft", "mouthSmileRight"] },
    { direction: "frown", key: "mouthFormFrown", names: ["mouthFrownLeft", "mouthFrownRight"] },
  ],
  /** 口を左へ寄せると右へ寄せる (利用者から見た左右。T0882)。 */
  mouthSide: [
    { direction: "left", key: "mouthSide", names: ["mouthLeft"] },
    { direction: "right", key: "mouthSideRight", names: ["mouthRight"] },
  ],
  /** 眉の上げと下げ (T0881)。 */
  brow: [
    { direction: "up", key: "brow", names: ["browInnerUp", "browOuterUpLeft", "browOuterUpRight"] },
    { direction: "down", key: "browDown", names: ["browDownLeft", "browDownRight"] },
  ],
} as const satisfies Partial<
  Record<TuningRange, readonly { direction: string; key: string; names: readonly string[] }[]>
>;
/** 左右・向きで分けられるまとまり。 */
export type SplitRange = keyof typeof TUNING_SPLITS;
export const SPLIT_RANGES = Object.keys(TUNING_SPLITS) as SplitRange[];
/** 分けたときの 1 つの組。 */
export type TuningSplit = (typeof TUNING_SPLITS)[SplitRange][number];
/** まとまりを分けるかのスイッチの項目。 */
export type SplitSwitch = `${SplitRange}Separate`;
/** 分けたときだけ使う境目の項目。 */
type SplitOnlyKey = Exclude<TuningSplit["key"], TuningRange>;
/** 境目の組を持つ項目 (`TUNING_RANGES` のまとまりと、分けたときだけ使う組)。 */
export type TuningRangeKey = TuningRange | SplitOnlyKey;

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
  /** 視線。`eyeLookSeparate` が切なら上下左右、入なら上へ見る量。 */
  eyeLook: RangeTuning;
  /** 視線を向き (上・下・左・右) で分けるか。 */
  eyeLookSeparate: boolean;
  /** 視線の下・左・右 (利用者から見た左右)。`eyeLookSeparate` が入のときだけ使う。 */
  eyeLookDown: RangeTuning;
  eyeLookLeft: RangeTuning;
  eyeLookRight: RangeTuning;
  /** 目の見開き。`eyeWideSeparate` が切なら両目、入なら左目 (利用者から見た左)。 */
  eyeWide: RangeTuning;
  /** 目の見開きの左右を別々にするか。 */
  eyeWideSeparate: boolean;
  /** 右目の見開き。`eyeWideSeparate` が入のときだけ使う。 */
  eyeWideRight: RangeTuning;
  jawOpen: RangeTuning;
  mouthClose: RangeTuning;
  /** 口の形。`mouthFormSeparate` が切なら笑みと口角の下がり、入なら笑み。 */
  mouthForm: RangeTuning;
  /** 口の形を笑みと口角の下がりで分けるか。 */
  mouthFormSeparate: boolean;
  /** 口角の下がり。`mouthFormSeparate` が入のときだけ使う。 */
  mouthFormFrown: RangeTuning;
  /** 口の左右。`mouthSideSeparate` が切なら左右へ寄せる量、入なら左へ寄せる量 (利用者から見た左)。 */
  mouthSide: RangeTuning;
  /** 口の左右を向き (左へ・右へ) で分けるか。 */
  mouthSideSeparate: boolean;
  /** 口を右へ寄せる量。`mouthSideSeparate` が入のときだけ使う。 */
  mouthSideRight: RangeTuning;
  /** 頬の膨らみ。 */
  cheekPuff: RangeTuning;
  /** 眉。`browSeparate` が切なら上げと下げ、入なら上げ。 */
  brow: RangeTuning;
  /** 眉を上げと下げで分けるか。 */
  browSeparate: boolean;
  /** 眉の下げ。`browSeparate` が入のときだけ使う。 */
  browDown: RangeTuning;
}

/** 感度の範囲。 */
export const GAIN_MIN = 0;
export const GAIN_MAX = 4;
/** 遊びの範囲 (度)。 */
export const DEAD_ZONE_MAX = 20;
/** 境目の間の最小の幅 (これより狭いと、わずかな揺れで 0 と 1 を行き来する)。 */
export const RANGE_MIN_SPAN = 0.05;

const DEFAULT_AXIS: AxisTuning = { gain: 1, deadZone: 0, separate: false, gainNegative: 1 };
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
  eyeLookSeparate: false,
  eyeLookDown: DEFAULT_RANGE,
  eyeLookLeft: DEFAULT_RANGE,
  eyeLookRight: DEFAULT_RANGE,
  eyeWide: DEFAULT_RANGE,
  eyeWideSeparate: false,
  eyeWideRight: DEFAULT_RANGE,
  jawOpen: DEFAULT_RANGE,
  mouthClose: DEFAULT_RANGE,
  mouthForm: DEFAULT_RANGE,
  mouthFormSeparate: false,
  mouthFormFrown: DEFAULT_RANGE,
  mouthSide: DEFAULT_RANGE,
  mouthSideSeparate: false,
  mouthSideRight: DEFAULT_RANGE,
  cheekPuff: DEFAULT_RANGE,
  brow: DEFAULT_RANGE,
  browSeparate: false,
  browDown: DEFAULT_RANGE,
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
    separate: typeof value.separate === "boolean" ? value.separate : DEFAULT_AXIS.separate,
    gainNegative: inRange(value.gainNegative, GAIN_MIN, GAIN_MAX, DEFAULT_AXIS.gainNegative),
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

function sanitizeSwitch(v: unknown): boolean {
  return typeof v === "boolean" ? v : false;
}

/**
 * 調整を範囲に収める (知らない項目は捨て、無い・壊れた項目は既定値にする)。分けるスイッチと分けたときの組が無い形
 * (T0879 より前の形を含む) は、分けないまとまりとして読む。
 */
export function sanitizeTuning(value: unknown): TrackingTuning {
  const v = isObject(value) ? value : {};
  return {
    yaw: sanitizeAxis(v.yaw),
    pitch: sanitizeAxis(v.pitch),
    roll: sanitizeAxis(v.roll),
    eyeBlink: sanitizeRange(v.eyeBlink),
    eyeBlinkSeparate: sanitizeSwitch(v.eyeBlinkSeparate),
    eyeBlinkRight: sanitizeRange(v.eyeBlinkRight),
    eyeLook: sanitizeRange(v.eyeLook),
    eyeLookSeparate: sanitizeSwitch(v.eyeLookSeparate),
    eyeLookDown: sanitizeRange(v.eyeLookDown),
    eyeLookLeft: sanitizeRange(v.eyeLookLeft),
    eyeLookRight: sanitizeRange(v.eyeLookRight),
    eyeWide: sanitizeRange(v.eyeWide),
    eyeWideSeparate: sanitizeSwitch(v.eyeWideSeparate),
    eyeWideRight: sanitizeRange(v.eyeWideRight),
    jawOpen: sanitizeRange(v.jawOpen),
    mouthClose: sanitizeRange(v.mouthClose),
    mouthForm: sanitizeRange(v.mouthForm),
    mouthFormSeparate: sanitizeSwitch(v.mouthFormSeparate),
    mouthFormFrown: sanitizeRange(v.mouthFormFrown),
    mouthSide: sanitizeRange(v.mouthSide),
    mouthSideSeparate: sanitizeSwitch(v.mouthSideSeparate),
    mouthSideRight: sanitizeRange(v.mouthSideRight),
    cheekPuff: sanitizeRange(v.cheekPuff),
    brow: sanitizeRange(v.brow),
    browSeparate: sanitizeSwitch(v.browSeparate),
    browDown: sanitizeRange(v.browDown),
  };
}

/** まとまりを分けるかのスイッチの項目の名前。 */
export function splitSwitch(range: SplitRange): SplitSwitch {
  return `${range}Separate`;
}

/**
 * まとまりを左右・向きで分けるかを切り替えた調整。入れるときは、分けたときだけ使う組に今の (まとまりの) 境目を写す
 * (切り替えただけでは出力が変わらない)。切るときは分けたときの組を残す (使わない)。
 */
export function setRangeSeparate(
  tuning: TrackingTuning,
  range: SplitRange,
  separate: boolean,
): TrackingTuning {
  const flag = splitSwitch(range);
  if (separate === tuning[flag]) return tuning;
  if (!separate) return { ...tuning, [flag]: false };
  const next: TrackingTuning = { ...tuning, [flag]: true };
  for (const split of TUNING_SPLITS[range]) {
    if (split.key !== range) next[split.key] = { ...tuning[range] };
  }
  return next;
}

/**
 * 軸の感度を向きで分けるかを切り替えた調整。入れるときは負の向きに今の感度を写す (切り替えただけでは出力が変わらない)。
 * 切るときは負の向きの感度を残す (使わない)。
 */
export function setAxisSeparate(
  tuning: TrackingTuning,
  axis: TuningAxis,
  separate: boolean,
): TrackingTuning {
  const a = tuning[axis];
  if (separate === a.separate) return tuning;
  return {
    ...tuning,
    [axis]: separate ? { ...a, separate: true, gainNegative: a.gain } : { ...a, separate: false },
  };
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

/** 当てても角度が変わらないか (向きで分けていても、両方の感度が 1 なら変わらない。分けていないときの `gainNegative` は見ない)。 */
function isIdentityAxis(a: AxisTuning): boolean {
  return a.gain === 1 && a.deadZone === 0 && (!a.separate || a.gainNegative === 1);
}

function isDefaultRange(r: RangeTuning): boolean {
  return r.low === 0 && r.high === 1;
}

function isSplitRange(key: TuningRange): key is SplitRange {
  return key in TUNING_SPLITS;
}

/**
 * 当てる境目の組と、当てるブレンドシェイプの名前。`TUNING_SPLITS` のまとまりは、分けるスイッチが入のときだけ左右・向きの組に分ける
 * (切のときの分けた組は使わない)。
 */
export function tuningRanges(tuning: TrackingTuning): [RangeTuning, readonly string[]][] {
  return (Object.keys(TUNING_RANGES) as TuningRange[]).flatMap(
    (key): [RangeTuning, readonly string[]][] =>
      isSplitRange(key) && tuning[splitSwitch(key)]
        ? TUNING_SPLITS[key].map((split) => [tuning[split.key], split.names])
        : [[tuning[key], TUNING_RANGES[key]]],
  );
}

/** 当てても何も変わらないか (左右・向きで分けていても、どの組の境目も既定なら変わらない)。 */
function isIdentityTuning(tuning: TrackingTuning): boolean {
  return (
    TUNING_AXES.every((axis) => isIdentityAxis(tuning[axis])) &&
    tuningRanges(tuning).every(([range]) => isDefaultRange(range))
  );
}

/**
 * 既定値と同じか (「既定に戻す」で変わるものが無いか。左右・向きで分けているまとまりや、感度を向きで分けている軸があるなら既定ではない)。
 */
export function isDefaultTuning(tuning: TrackingTuning): boolean {
  return (
    SPLIT_RANGES.every((range) => !tuning[splitSwitch(range)]) &&
    TUNING_AXES.every((axis) => !tuning[axis].separate) &&
    isIdentityTuning(tuning)
  );
}

/**
 * 角度に遊びと感度を当てる (遊びの中は 0、外は遊びの分を引いて倍率を掛ける。符号は保つ)。向きで分けているなら、
 * 負の角度には `gainNegative` を掛ける (遊びの中はどちらも 0 なので、0 をまたいで続く)。
 */
export function tuneAngle(value: number, axis: AxisTuning): number {
  if (isIdentityAxis(axis)) return value;
  const gain = axis.separate && value < 0 ? axis.gainNegative : axis.gain;
  const magnitude = Math.max(0, Math.abs(value) - axis.deadZone) * gain;
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
