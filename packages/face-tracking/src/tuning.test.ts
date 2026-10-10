import { describe, expect, it } from "vitest";
import { applyCalibration } from "./calibration";
import { type TrackingFrame, unmirrorFrame } from "./frame";
import {
  applyTuning,
  DEFAULT_TUNING,
  isDefaultTuning,
  moveRangeBound,
  RANGE_MIN_SPAN,
  sanitizeTuning,
  type TrackingTuning,
  tuneAngle,
  tuneScore,
} from "./tuning";

const FRAME: TrackingFrame = {
  head: { yaw: 12.5, pitch: -7.25, roll: 3 },
  blendShapes: {
    eyeBlinkLeft: 0.4,
    eyeBlinkRight: 0.1,
    jawOpen: 0.3,
    mouthClose: 0.05,
    browInnerUp: 0.2,
  },
  body: { yaw: 4, pitch: null, roll: -1, shoulderLeft: 0.1, shoulderRight: null },
};

function tuning(patch: Partial<TrackingTuning>): TrackingTuning {
  return { ...DEFAULT_TUNING, ...patch };
}

describe("applyTuning", () => {
  it("returns the same frame with the defaults", () => {
    expect(isDefaultTuning(DEFAULT_TUNING)).toBe(true);
    expect(applyTuning(FRAME, DEFAULT_TUNING)).toBe(FRAME);
    // 既定値と同じ値を新しく作っても同じ。
    expect(applyTuning(FRAME, sanitizeTuning({}))).toBe(FRAME);
  });

  it("applies the gain and the dead zone around neutral", () => {
    const axis = { gain: 2, deadZone: 5 };
    expect(tuneAngle(3, axis)).toBe(0);
    expect(tuneAngle(-5, axis)).toBe(0);
    expect(tuneAngle(12.5, axis)).toBe(15);
    expect(tuneAngle(-12.5, axis)).toBe(-15);
    // 遊びの境で続けてつながる。
    expect(tuneAngle(5.001, axis)).toBeCloseTo(0.002);
    const out = applyTuning(FRAME, tuning({ yaw: axis, roll: { gain: 0.5, deadZone: 0 } }));
    expect(out.head).toEqual({ yaw: 15, pitch: -7.25, roll: 1.5 });
  });

  it("stretches the range between the bounds and clamps outside", () => {
    const range = { low: 0.2, high: 0.6 };
    expect(tuneScore(0.1, range)).toBe(0);
    expect(tuneScore(0.2, range)).toBe(0);
    expect(tuneScore(0.4, range)).toBeCloseTo(0.5);
    expect(tuneScore(0.6, range)).toBe(1);
    expect(tuneScore(0.9, range)).toBe(1);
    const out = applyTuning(FRAME, tuning({ eyeBlink: range, jawOpen: { low: 0, high: 0.5 } }));
    // 左右の瞼を一緒に動かす。
    expect(out.blendShapes.eyeBlinkLeft).toBeCloseTo(0.5);
    expect(out.blendShapes.eyeBlinkRight).toBe(0);
    expect(out.blendShapes.jawOpen).toBeCloseTo(0.6);
    // ほかのブレンドシェイプ・頭・体は変えない。
    expect(out.blendShapes.mouthClose).toBe(0.05);
    expect(out.blendShapes.browInnerUp).toBe(0.2);
    expect(out.head).toEqual(FRAME.head);
    expect(out.body).toEqual(FRAME.body);
    // 入力のフレームは書き換えない。
    expect(FRAME.blendShapes.eyeBlinkLeft).toBe(0.4);
  });

  it("does not add blend shapes that are missing", () => {
    const frame = { head: FRAME.head, blendShapes: { eyeBlinkLeft: 0.5 } };
    const out = applyTuning(
      frame,
      tuning({ eyeBlink: { low: 0.1, high: 0.9 }, jawOpen: { low: 0.1, high: 0.9 } }),
    );
    expect(Object.keys(out.blendShapes)).toEqual(["eyeBlinkLeft"]);
  });

  it("treats broken values as the defaults", () => {
    const broken = {
      yaw: { gain: Number.NaN, deadZone: Number.POSITIVE_INFINITY },
      pitch: { gain: -1, deadZone: -3 },
      roll: null,
      eyeBlink: { low: 0.8, high: 0.2 },
      jawOpen: { low: Number.NaN, high: 0.5 },
    } as unknown as TrackingTuning;
    const out = applyTuning(FRAME, broken);
    expect(out.head.yaw).toBe(FRAME.head.yaw);
    // 負の感度は 0 に寄せる (範囲の外は端へ)。
    expect(out.head.pitch).toBe(0);
    expect(out.head.roll).toBe(FRAME.head.roll);
    expect(out.blendShapes.eyeBlinkLeft).toBe(0.4);
    expect(out.blendShapes.jawOpen).toBeCloseTo(0.6);
  });

  it("applies to the sides the user sees after swapping sides", () => {
    // 検出 → (左右の入れ替え) → キャリブレーション → 調整 の順。入れ替えた後の右目に当たる。
    const raw = {
      head: { yaw: 20, pitch: 0, roll: 0 },
      blendShapes: { eyeBlinkLeft: 0.9, eyeBlinkRight: 0 },
    };
    const swapped = applyCalibration(unmirrorFrame(raw), null);
    const out = applyTuning(
      swapped,
      tuning({ yaw: { gain: 1, deadZone: 10 }, eyeBlink: { low: 0, high: 0.5 } }),
    );
    expect(out.head.yaw).toBe(-10);
    expect(out.blendShapes.eyeBlinkRight).toBe(1);
    expect(out.blendShapes.eyeBlinkLeft).toBe(0);
  });
});

describe("sanitizeTuning", () => {
  it("fills missing items and drops unknown ones", () => {
    expect(sanitizeTuning(null)).toEqual(DEFAULT_TUNING);
    const t = sanitizeTuning({ yaw: { gain: 2 }, brow: { low: 0.1, high: 0.2 } });
    expect(t).toEqual({ ...DEFAULT_TUNING, yaw: { gain: 2, deadZone: 0 } });
  });

  it("keeps values in range", () => {
    const t = sanitizeTuning({
      yaw: { gain: 100, deadZone: 100 },
      eyeBlink: { low: -1, high: 2 },
      jawOpen: { low: 0.3, high: 0.3 + RANGE_MIN_SPAN },
    });
    expect(t.yaw).toEqual({ gain: 4, deadZone: 20 });
    expect(t.eyeBlink).toEqual({ low: 0, high: 1 });
    expect(t.jawOpen.low).toBe(0.3);
    // 間が狭すぎる・逆さまの組は既定に戻す。
    expect(sanitizeTuning({ eyeBlink: { low: 0.5, high: 0.5 } }).eyeBlink).toEqual({
      low: 0,
      high: 1,
    });
    expect(sanitizeTuning({ eyeBlink: { low: 0.6, high: 0.4 } }).eyeBlink).toEqual({
      low: 0,
      high: 1,
    });
  });
});

describe("moveRangeBound", () => {
  it("keeps the minimum span by pushing the other bound", () => {
    expect(moveRangeBound({ low: 0.2, high: 0.6 }, "low", 0.4)).toEqual({ low: 0.4, high: 0.6 });
    expect(moveRangeBound({ low: 0.2, high: 0.6 }, "low", 0.7)).toEqual({
      low: 0.7,
      high: 0.7 + RANGE_MIN_SPAN,
    });
    expect(moveRangeBound({ low: 0.2, high: 0.6 }, "high", 0.1)).toEqual({
      low: 0.1 - RANGE_MIN_SPAN,
      high: 0.1,
    });
    // 端では動かした方を止める。
    expect(moveRangeBound({ low: 0.2, high: 0.6 }, "low", 1)).toEqual({
      low: 1 - RANGE_MIN_SPAN,
      high: 1,
    });
    expect(moveRangeBound({ low: 0.2, high: 0.6 }, "high", 0)).toEqual({
      low: 0,
      high: RANGE_MIN_SPAN,
    });
    expect(moveRangeBound({ low: 0.2, high: 0.6 }, "low", Number.NaN)).toEqual({
      low: 0.2,
      high: 0.6,
    });
    // 動かした結果は sanitizeTuning を通っても変わらない。
    for (const v of [0, 0.33, 0.62, 0.95, 1]) {
      const moved = moveRangeBound({ low: 0.3, high: 0.35 }, "low", v);
      expect(sanitizeTuning({ jawOpen: moved }).jawOpen).toEqual(moved);
      const movedHigh = moveRangeBound({ low: 0.3, high: 0.35 }, "high", v);
      expect(sanitizeTuning({ jawOpen: movedHigh }).jawOpen).toEqual(movedHigh);
    }
  });
});
