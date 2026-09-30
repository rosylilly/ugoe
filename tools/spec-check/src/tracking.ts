// 追従 (トラッキング): 顔の追従の入力からパラメータの値を作る (spec/avatar-profile.md#face-tracking)。

import type { ParameterValues } from "./evaluate.ts";
import type * as M from "./types.ts";

/** 1 フレームの追従の入力 (spec/avatar-profile.md#tracking-input)。角度は度。 */
export interface TrackingInput {
  head: { yaw: number; pitch: number; roll: number };
  /** 上半身の回転。各角度は個別に無くてよい。 */
  body?: { yaw?: number; pitch?: number; roll?: number };
  /** 本人の左右の肩の上がり具合。それぞれ無くてよい。 */
  shoulders?: { left?: number; right?: number };
  /** ARKit のブレンドシェイプの名前 → 値。無いものは 0。 */
  blend_shapes?: Record<string, number>;
}

/** ARKit の 52 のブレンドシェイプの名前 (spec/avatar-profile.md#tracking-input)。 */
export const BLEND_SHAPES: readonly string[] = [
  "eyeBlinkLeft",
  "eyeLookDownLeft",
  "eyeLookInLeft",
  "eyeLookOutLeft",
  "eyeLookUpLeft",
  "eyeSquintLeft",
  "eyeWideLeft",
  "eyeBlinkRight",
  "eyeLookDownRight",
  "eyeLookInRight",
  "eyeLookOutRight",
  "eyeLookUpRight",
  "eyeSquintRight",
  "eyeWideRight",
  "jawForward",
  "jawLeft",
  "jawRight",
  "jawOpen",
  "mouthClose",
  "mouthFunnel",
  "mouthPucker",
  "mouthLeft",
  "mouthRight",
  "mouthSmileLeft",
  "mouthSmileRight",
  "mouthFrownLeft",
  "mouthFrownRight",
  "mouthDimpleLeft",
  "mouthDimpleRight",
  "mouthStretchLeft",
  "mouthStretchRight",
  "mouthRollLower",
  "mouthRollUpper",
  "mouthShrugLower",
  "mouthShrugUpper",
  "mouthPressLeft",
  "mouthPressRight",
  "mouthLowerDownLeft",
  "mouthLowerDownRight",
  "mouthUpperUpLeft",
  "mouthUpperUpRight",
  "browDownLeft",
  "browDownRight",
  "browInnerUp",
  "browOuterUpLeft",
  "browOuterUpRight",
  "cheekPuff",
  "cheekSquintLeft",
  "cheekSquintRight",
  "noseSneerLeft",
  "noseSneerRight",
  "tongueOut",
];

type Formula = (i: TrackingInput, b: (name: string) => number) => number | null;

/** 標準の対応の式 (spec/avatar-profile.md#standard-mapping)。値が無ければ null。 */
export const STANDARD_MAPPING: ReadonlyMap<string, Formula> = new Map<string, Formula>([
  ["AngleX", (i) => i.head.yaw],
  ["AngleY", (i) => i.head.pitch],
  ["AngleZ", (i) => i.head.roll],
  ["BodyAngleX", (i) => i.body?.yaw ?? 0.3 * i.head.yaw],
  ["BodyAngleY", (i) => i.body?.pitch ?? 0.3 * i.head.pitch],
  ["BodyAngleZ", (i) => i.body?.roll ?? 0.3 * i.head.roll],
  ["ShoulderLY", (i) => i.shoulders?.left ?? null],
  ["ShoulderRY", (i) => i.shoulders?.right ?? null],
  ["EyeLOpen", (_, b) => 1 - b("eyeBlinkLeft")],
  ["EyeROpen", (_, b) => 1 - b("eyeBlinkRight")],
  ["EyeLWide", (_, b) => b("eyeWideLeft")],
  ["EyeRWide", (_, b) => b("eyeWideRight")],
  ["EyeLSmile", (_, b) => b("eyeSquintLeft")],
  ["EyeRSmile", (_, b) => b("eyeSquintRight")],
  [
    "EyeBallX",
    (_, b) =>
      (b("eyeLookOutLeft") - b("eyeLookInLeft") - (b("eyeLookOutRight") - b("eyeLookInRight"))) / 2,
  ],
  [
    "EyeBallY",
    (_, b) =>
      (b("eyeLookUpLeft") + b("eyeLookUpRight") - b("eyeLookDownLeft") - b("eyeLookDownRight")) / 2,
  ],
  ["BrowLY", (_, b) => (b("browOuterUpLeft") + b("browInnerUp")) / 2 - b("browDownLeft")],
  ["BrowRY", (_, b) => (b("browOuterUpRight") + b("browInnerUp")) / 2 - b("browDownRight")],
  ["BrowInnerUp", (_, b) => b("browInnerUp")],
  ["BrowLAngle", (_, b) => b("browOuterUpLeft") - b("browInnerUp")],
  ["BrowRAngle", (_, b) => b("browOuterUpRight") - b("browInnerUp")],
  ["MouthOpenY", (_, b) => Math.max(0, b("jawOpen") - b("mouthClose"))],
  [
    "MouthForm",
    (_, b) =>
      (b("mouthSmileLeft") + b("mouthSmileRight") - b("mouthFrownLeft") - b("mouthFrownRight")) / 2,
  ],
  ["MouthX", (_, b) => b("mouthLeft") - b("mouthRight")],
  ["JawX", (_, b) => b("jawLeft") - b("jawRight")],
  ["MouthPucker", (_, b) => b("mouthPucker")],
  ["MouthFunnel", (_, b) => b("mouthFunnel")],
  ["MouthPress", (_, b) => (b("mouthPressLeft") + b("mouthPressRight")) / 2],
  ["MouthShrug", (_, b) => (b("mouthShrugUpper") + b("mouthShrugLower")) / 2],
  ["CheekPuff", (_, b) => b("cheekPuff")],
  ["TongueOut", (_, b) => b("tongueOut")],
  ["NoseSneer", (_, b) => (b("noseSneerLeft") + b("noseSneerRight")) / 2],
]);

const HEAD_INPUTS = new Set(["yaw", "pitch", "roll"]);

/** 要素の入力の値 (spec/avatar-profile.md#overrides)。知らない入力なら undefined、値が無ければ null。 */
function inputValue(
  o: M.TrackingOverride,
  input: TrackingInput,
  b: (name: string) => number,
): number | null | undefined {
  switch (o.source) {
    case "standard": {
      const f = STANDARD_MAPPING.get(o.input);
      return f ? f(input, b) : undefined;
    }
    case "blend_shape":
      return BLEND_SHAPES.includes(o.input) ? b(o.input) : undefined;
    case "head":
      return HEAD_INPUTS.has(o.input)
        ? input.head[o.input as keyof TrackingInput["head"]]
        : undefined;
  }
}

/**
 * 追従で書くパラメータの値 (spec/avatar-profile.md#computing-the-parameter-values)。
 * 返すのは書く値 (範囲に切り詰めた後) だけで、値の無いパラメータは含まない。
 */
export function trackingValues(model: M.Model, input: TrackingInput): Map<string, number> {
  const params = new Map(model.parameters.map((p) => [p.id, p]));
  const b = (name: string): number => input.blend_shapes?.[name] ?? 0;
  const tracking = model.avatar?.tracking ?? [];

  // 無視しない要素 (知らない source の要素は読み込みで除いてある)
  const active: { o: M.TrackingOverride; target: string; value: number | null }[] = [];
  for (const o of tracking) {
    const value = inputValue(o, input, b);
    if (value === undefined) continue;
    const target = o.parameter ?? (o.source === "standard" ? o.input : null);
    if (target === null || !params.has(target)) continue;
    if (!Number.isFinite(o.scale) || !Number.isFinite(o.offset)) continue;
    active.push({ o, target, value });
  }

  const values = new Map<string, number | null>();
  // 1. 上書き: 同じ対象の要素の和。どれかの入力に値が無ければ値なし
  for (const { o, target, value } of active) {
    const prev = values.has(target) ? (values.get(target) ?? null) : 0;
    values.set(target, prev === null || value === null ? null : prev + value * o.scale + o.offset);
  }
  // 2. 標準の対応 (置き換えられたものを除く)
  const replaced = new Set<string>();
  for (const { o, target } of active) {
    if (o.source === "standard") replaced.add(o.input);
    replaced.add(target);
  }
  for (const [id, f] of STANDARD_MAPPING) {
    if (!params.has(id) || replaced.has(id)) continue;
    values.set(id, f(input, b));
  }
  // 3. 除外と 4. 切り詰め
  const out = new Map<string, number>();
  for (const [id, v] of values) {
    const p = params.get(id);
    if (!p || p.internal || v === null || !Number.isFinite(v)) continue;
    out.set(id, Math.min(Math.max(v, p.min), p.max));
  }
  return out;
}

/** 追従の値を書く (1 フレームの順の手順 3)。 */
export function applyTracking(model: M.Model, values: ParameterValues, input: TrackingInput): void {
  for (const [id, v] of trackingValues(model, input)) values.set(id, v);
}
