// キャリブレーション (利用者の中立の顔を差し引く)。docs/spec/avatar-profile.md#実装の範囲 でホストの責務とされたもの。
import type { TrackingFrame } from "./frame";

/**
 * 中立の顔 `neutral` を基準に直す。頭の回転は差し引き、ブレンドシェイプは中立の値を 0 とし、
 * 1 は 1 のまま保つように伸ばす (`(v − n) / (1 − n)`、0〜1 にクランプ)。
 */
export function applyCalibration(
  frame: TrackingFrame,
  neutral: TrackingFrame | null,
): TrackingFrame {
  if (neutral === null) return frame;
  const blendShapes: Record<string, number> = {};
  for (const [name, value] of Object.entries(frame.blendShapes)) {
    const n = neutral.blendShapes[name] ?? 0;
    blendShapes[name] = n < 0.99 ? Math.min(1, Math.max(0, (value - n) / (1 - n))) : value;
  }
  return {
    head: {
      yaw: frame.head.yaw - neutral.head.yaw,
      pitch: frame.head.pitch - neutral.head.pitch,
      roll: frame.head.roll - neutral.head.roll,
    },
    blendShapes,
  };
}

/** フレームの平均 (中立の顔を 1 フレームの揺れに左右されずに取るため)。空なら `null`。 */
export function averageFrames(frames: readonly TrackingFrame[]): TrackingFrame | null {
  if (frames.length === 0) return null;
  const head = { yaw: 0, pitch: 0, roll: 0 };
  const sums: Record<string, number> = {};
  for (const f of frames) {
    head.yaw += f.head.yaw;
    head.pitch += f.head.pitch;
    head.roll += f.head.roll;
    for (const [name, value] of Object.entries(f.blendShapes))
      sums[name] = (sums[name] ?? 0) + value;
  }
  const n = frames.length;
  const blendShapes: Record<string, number> = {};
  for (const [name, sum] of Object.entries(sums)) blendShapes[name] = sum / n;
  return { head: { yaw: head.yaw / n, pitch: head.pitch / n, roll: head.roll / n }, blendShapes };
}
