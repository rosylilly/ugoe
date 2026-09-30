// キャリブレーション (利用者の中立の顔を差し引く)。docs/spec/avatar-profile.md#実装の範囲 でホストの責務とされたもの。
import { BODY_KEYS, type BodyKey, type BodyPose, type TrackingFrame } from "./frame";

/**
 * 基準 (中立) を差し引いたときだけ使う体の値。カメラの高さや体格で基準が大きく変わるので、キャリブレーションして
 * いなければ (または中立の姿勢でその値が無ければ) 渡さない (pose.ts)。
 */
const RELATIVE_BODY_KEYS: readonly BodyKey[] = ["pitch", "shoulderLeft", "shoulderRight"];

/** 体の値に中立を差し引く。キャリブレーションしていなければ、相対でだけ使う値を外す。 */
function calibrateBody(body: BodyPose, neutral: BodyPose | undefined | null): BodyPose {
  const out = { ...body };
  for (const key of BODY_KEYS) {
    const v = body[key];
    const n = neutral?.[key] ?? null;
    if (v === null) continue;
    if (n !== null) out[key] = v - n;
    else if (RELATIVE_BODY_KEYS.includes(key)) out[key] = null;
  }
  return out;
}

/**
 * 中立の顔 `neutral` を基準に直す。頭の回転は差し引き、ブレンドシェイプは中立の値を 0 とし、
 * 1 は 1 のまま保つように伸ばす (`(v − n) / (1 − n)`、0〜1 にクランプ)。体の値は差し引く
 * (中立に無い値のうち、ピッチと肩の上下は外す。キャリブレーションしていなくても外す)。
 */
export function applyCalibration(
  frame: TrackingFrame,
  neutral: TrackingFrame | null,
): TrackingFrame {
  if (neutral === null) {
    return frame.body ? { ...frame, body: calibrateBody(frame.body, null) } : frame;
  }
  const blendShapes: Record<string, number> = {};
  for (const [name, value] of Object.entries(frame.blendShapes)) {
    const n = neutral.blendShapes[name] ?? 0;
    blendShapes[name] = n < 0.99 ? Math.min(1, Math.max(0, (value - n) / (1 - n))) : value;
  }
  const out: TrackingFrame = {
    head: {
      yaw: frame.head.yaw - neutral.head.yaw,
      pitch: frame.head.pitch - neutral.head.pitch,
      roll: frame.head.roll - neutral.head.roll,
    },
    blendShapes,
  };
  if (frame.body) out.body = calibrateBody(frame.body, neutral.body);
  return out;
}

/** フレームの平均 (中立の顔を 1 フレームの揺れに左右されずに取るため)。空なら `null`。体の値は有る分だけで平均する。 */
export function averageFrames(frames: readonly TrackingFrame[]): TrackingFrame | null {
  if (frames.length === 0) return null;
  const head = { yaw: 0, pitch: 0, roll: 0 };
  const sums: Record<string, number> = {};
  const bodySums: Record<BodyKey, [number, number]> = {
    yaw: [0, 0],
    pitch: [0, 0],
    roll: [0, 0],
    shoulderLeft: [0, 0],
    shoulderRight: [0, 0],
  };
  let hasBody = false;
  for (const f of frames) {
    head.yaw += f.head.yaw;
    head.pitch += f.head.pitch;
    head.roll += f.head.roll;
    for (const [name, value] of Object.entries(f.blendShapes))
      sums[name] = (sums[name] ?? 0) + value;
    if (f.body) {
      hasBody = true;
      for (const key of BODY_KEYS) {
        const v = f.body[key];
        if (v !== null) {
          bodySums[key][0] += v;
          bodySums[key][1] += 1;
        }
      }
    }
  }
  const n = frames.length;
  const blendShapes: Record<string, number> = {};
  for (const [name, sum] of Object.entries(sums)) blendShapes[name] = sum / n;
  const out: TrackingFrame = {
    head: { yaw: head.yaw / n, pitch: head.pitch / n, roll: head.roll / n },
    blendShapes,
  };
  if (hasBody) {
    const avg = (key: BodyKey) => {
      const [sum, count] = bodySums[key];
      return count > 0 ? sum / count : null;
    };
    out.body = {
      yaw: avg("yaw"),
      pitch: avg("pitch"),
      roll: avg("roll"),
      shoulderLeft: avg("shoulderLeft"),
      shoulderRight: avg("shoulderRight"),
    };
  }
  return out;
}
