import { describe, expect, it } from "vitest";
import { applyCalibration } from "./calibration";
import { type TrackingFrame, unmirrorFrame } from "./frame";
import {
  type AxisTuning,
  applyTuning,
  DEFAULT_TUNING,
  isDefaultTuning,
  moveRangeBound,
  RANGE_MIN_SPAN,
  sanitizeTuning,
  setAxisSeparate,
  setEyeBlinkSeparate,
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

function axis(patch: Partial<AxisTuning>): AxisTuning {
  return { ...DEFAULT_TUNING.yaw, ...patch };
}

describe("applyTuning", () => {
  it("returns the same frame with the defaults", () => {
    expect(isDefaultTuning(DEFAULT_TUNING)).toBe(true);
    expect(applyTuning(FRAME, DEFAULT_TUNING)).toBe(FRAME);
    // 既定値と同じ値を新しく作っても同じ。
    expect(applyTuning(FRAME, sanitizeTuning({}))).toBe(FRAME);
  });

  it("applies the gain and the dead zone around neutral", () => {
    const a = axis({ gain: 2, deadZone: 5 });
    expect(tuneAngle(3, a)).toBe(0);
    expect(tuneAngle(-5, a)).toBe(0);
    expect(tuneAngle(12.5, a)).toBe(15);
    expect(tuneAngle(-12.5, a)).toBe(-15);
    // 遊びの境で続けてつながる。
    expect(tuneAngle(5.001, a)).toBeCloseTo(0.002);
    const out = applyTuning(FRAME, tuning({ yaw: a, roll: axis({ gain: 0.5, deadZone: 0 }) }));
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
      tuning({ yaw: axis({ gain: 1, deadZone: 10 }), eyeBlink: { low: 0, high: 0.5 } }),
    );
    expect(out.head.yaw).toBe(-10);
    expect(out.blendShapes.eyeBlinkRight).toBe(1);
    expect(out.blendShapes.eyeBlinkLeft).toBe(0);
  });
});

describe("applyTuning with the gain separated by direction", () => {
  it("returns the same frame when the switch is on but both gains are 1", () => {
    const separate = setAxisSeparate(DEFAULT_TUNING, "yaw", true);
    expect(separate.yaw).toEqual(axis({ separate: true, gainNegative: 1 }));
    expect(isDefaultTuning(separate)).toBe(false);
    expect(applyTuning(FRAME, separate)).toBe(FRAME);
    // 切のときの負の向きの感度は使わない。
    expect(applyTuning(FRAME, tuning({ yaw: axis({ gainNegative: 3 }) }))).toBe(FRAME);
    expect(tuneAngle(-12.5, axis({ gainNegative: 3 }))).toBe(-12.5);
  });

  it("applies a separate gain to each direction", () => {
    const a = axis({ gain: 2, gainNegative: 0.5, separate: true });
    expect(tuneAngle(10, a)).toBe(20);
    expect(tuneAngle(-10, a)).toBe(-5);
    expect(tuneAngle(0, a)).toBe(0);
    const out = applyTuning(
      FRAME,
      tuning({ pitch: axis({ gain: 3, gainNegative: 2, separate: true }) }),
    );
    // ピッチは負 (下) なので負の向きの感度。
    expect(out.head).toEqual({ yaw: 12.5, pitch: -14.5, roll: 3 });
  });

  it("is continuous across the dead zone and through zero", () => {
    const a = axis({ gain: 4, deadZone: 5, gainNegative: 0.25, separate: true });
    expect(tuneAngle(5, a)).toBe(0);
    expect(tuneAngle(-5, a)).toBe(0);
    expect(tuneAngle(5.001, a)).toBeCloseTo(0.004);
    expect(tuneAngle(-5.001, a)).toBeCloseTo(-0.00025);
    // 遊びが 0 でも 0 をまたいで続く (片側の倍率だけが変わる)。
    const b = axis({ gain: 2, gainNegative: 0.5, separate: true });
    expect(tuneAngle(1e-6, b)).toBeCloseTo(0);
    expect(tuneAngle(-1e-6, b)).toBeCloseTo(0);
  });

  it("splits yaw and roll by the sides the user sees after swapping sides", () => {
    // 入れ替えの前の正のヨー (反転した映像の右) は、入れ替えた後の負 (利用者から見て右) になる。
    const raw = { head: { yaw: 20, pitch: 0, roll: 10 }, blendShapes: {} };
    const swapped = applyCalibration(unmirrorFrame(raw), null);
    const t = tuning({
      yaw: axis({ gain: 1, gainNegative: 2, separate: true }),
      roll: axis({ gain: 1, gainNegative: 3, separate: true }),
    });
    const out = applyTuning(swapped, t);
    expect(out.head.yaw).toBe(-40);
    expect(out.head.roll).toBe(-30);
    // 入れ替えないときは正の向きの感度 (1) のまま。
    const unswapped = applyTuning(applyCalibration(raw, null), t);
    expect(unswapped.head.yaw).toBe(20);
    expect(unswapped.head.roll).toBe(10);
  });

  it("copies the gain when the switch is turned on and keeps the negative gain when off", () => {
    const on = setAxisSeparate(tuning({ roll: axis({ gain: 1.5 }) }), "roll", true);
    expect(on.roll).toEqual(axis({ gain: 1.5, gainNegative: 1.5, separate: true }));
    // 入れただけでは出力が変わらない。
    expect(applyTuning(FRAME, on).head.roll).toBe(
      applyTuning(FRAME, tuning({ roll: axis({ gain: 1.5 }) })).head.roll,
    );
    const off = setAxisSeparate({ ...on, roll: { ...on.roll, gainNegative: 0.5 } }, "roll", false);
    expect(off.roll).toEqual(axis({ gain: 1.5, gainNegative: 0.5 }));
    expect(setAxisSeparate(off, "roll", false)).toBe(off);
    // ほかの軸は変えない。
    expect(on.yaw).toBe(DEFAULT_TUNING.yaw);
  });
});

describe("applyTuning on the added ranges", () => {
  const face: TrackingFrame = {
    head: { yaw: 0, pitch: 0, roll: 0 },
    blendShapes: {
      eyeBlinkLeft: 0.3,
      eyeBlinkRight: 0.3,
      eyeLookInLeft: 0.25,
      eyeLookOutRight: 0.5,
      eyeLookUpLeft: 0.1,
      eyeLookDownRight: 0.6,
      jawOpen: 0.2,
      mouthClose: 0.04,
      mouthSmileLeft: 0.3,
      mouthFrownRight: 0.05,
      browInnerUp: 0.2,
      browOuterUpLeft: 0.4,
      browDownRight: 0.01,
      cheekPuff: 0.5,
    },
  };

  it("returns the same frame with the defaults, also when the eyelids are separate", () => {
    expect(applyTuning(face, DEFAULT_TUNING)).toBe(face);
    const separate = setEyeBlinkSeparate(DEFAULT_TUNING, true);
    expect(isDefaultTuning(separate)).toBe(false);
    expect(applyTuning(face, separate)).toBe(face);
    // 切のときの右目の境目は使わない。
    expect(applyTuning(face, tuning({ eyeBlinkRight: { low: 0.1, high: 0.2 } }))).toBe(face);
  });

  it("stretches each group with one pair of bounds and clamps outside", () => {
    const out = applyTuning(
      face,
      tuning({
        eyeLook: { low: 0, high: 0.5 },
        mouthClose: { low: 0, high: 0.08 },
        mouthForm: { low: 0.1, high: 0.5 },
        brow: { low: 0.05, high: 0.25 },
      }),
    );
    const b = out.blendShapes;
    // 視線: 0.5 で見きった。
    expect(b.eyeLookInLeft).toBeCloseTo(0.5);
    expect(b.eyeLookOutRight).toBe(1);
    expect(b.eyeLookUpLeft).toBeCloseTo(0.2);
    expect(b.eyeLookDownRight).toBe(1);
    // 口の閉じ: 0.08 で閉じた。
    expect(b.mouthClose).toBeCloseTo(0.5);
    // 口の形: 笑みと口角の下がりを一緒に。下の境目より下は 0。
    expect(b.mouthSmileLeft).toBeCloseTo(0.5);
    expect(b.mouthFrownRight).toBe(0);
    // 眉: 内・外の上げと下げを一緒に。
    expect(b.browInnerUp).toBeCloseTo(0.75);
    expect(b.browOuterUpLeft).toBe(1);
    expect(b.browDownRight).toBe(0);
    // ほかは変えない。
    expect(b.eyeBlinkLeft).toBe(0.3);
    expect(b.jawOpen).toBe(0.2);
    expect(b.cheekPuff).toBe(0.5);
  });

  it("tunes the eyelids together or separately", () => {
    const together = applyTuning(face, tuning({ eyeBlink: { low: 0, high: 0.6 } }));
    expect(together.blendShapes.eyeBlinkLeft).toBeCloseTo(0.5);
    expect(together.blendShapes.eyeBlinkRight).toBeCloseTo(0.5);
    // 入れた直後は右目に両目の境目を写すので、出力は変わらない。
    const on = setEyeBlinkSeparate(tuning({ eyeBlink: { low: 0, high: 0.6 } }), true);
    expect(on.eyeBlinkRight).toEqual({ low: 0, high: 0.6 });
    expect(applyTuning(face, on).blendShapes).toEqual(together.blendShapes);
    // 右目だけ変える。
    const separate = { ...on, eyeBlinkRight: { low: 0.2, high: 0.4 } };
    const out = applyTuning(face, separate);
    expect(out.blendShapes.eyeBlinkLeft).toBeCloseTo(0.5);
    expect(out.blendShapes.eyeBlinkRight).toBeCloseTo(0.5);
    const out2 = applyTuning(face, { ...separate, eyeBlink: { low: 0.3, high: 1 } });
    expect(out2.blendShapes.eyeBlinkLeft).toBe(0);
    expect(out2.blendShapes.eyeBlinkRight).toBeCloseTo(0.5);
    // 切ると右目の境目は残るが使わない。
    const off = setEyeBlinkSeparate(separate, false);
    expect(off.eyeBlinkRight).toEqual({ low: 0.2, high: 0.4 });
    expect(applyTuning(face, off).blendShapes).toEqual(together.blendShapes);
    expect(setEyeBlinkSeparate(off, false)).toBe(off);
  });

  it("applies the separate eyelids to the sides the user sees after swapping sides", () => {
    // 検出の左目が閉じ、右目が開いている。入れ替えると利用者から見た右目が閉じている。
    const raw = {
      head: { yaw: 0, pitch: 0, roll: 0 },
      blendShapes: { eyeBlinkLeft: 0.5, eyeBlinkRight: 0.1 },
    };
    const swapped = applyCalibration(unmirrorFrame(raw), null);
    expect(swapped.blendShapes.eyeBlinkRight).toBe(0.5);
    const out = applyTuning(swapped, {
      ...setEyeBlinkSeparate(DEFAULT_TUNING, true),
      // 左目 (利用者から見た左) だけ 0.4 で閉じる。
      eyeBlink: { low: 0, high: 0.4 },
      eyeBlinkRight: { low: 0, high: 1 },
    });
    expect(out.blendShapes.eyeBlinkLeft).toBeCloseTo(0.25);
    expect(out.blendShapes.eyeBlinkRight).toBe(0.5);
    // 入れ替えずに同じ調整を当てると、検出の左目 (0.5) に左の境目が当たる。
    const unswapped = applyTuning(raw, {
      ...setEyeBlinkSeparate(DEFAULT_TUNING, true),
      eyeBlink: { low: 0.4, high: 0.6 },
      eyeBlinkRight: { low: 0, high: 1 },
    });
    expect(unswapped.blendShapes.eyeBlinkLeft).toBeCloseTo(0.5);
    expect(unswapped.blendShapes.eyeBlinkRight).toBe(0.1);
  });
});

describe("sanitizeTuning", () => {
  it("fills missing items and drops unknown ones", () => {
    expect(sanitizeTuning(null)).toEqual(DEFAULT_TUNING);
    const t = sanitizeTuning({ yaw: { gain: 2 }, cheekPuff: { low: 0.1, high: 0.2 } });
    expect(t).toEqual({ ...DEFAULT_TUNING, yaw: axis({ gain: 2, deadZone: 0 }) });
    expect(Object.keys(t)).toEqual(Object.keys(DEFAULT_TUNING));
    // T0877 の形 (眉・視線・口の形・口の閉じ・瞼の左右が無い) も読める。
    const old = sanitizeTuning({
      yaw: { gain: 1.5, deadZone: 2 },
      pitch: { gain: 1, deadZone: 0 },
      roll: { gain: 1, deadZone: 0 },
      eyeBlink: { low: 0.1, high: 0.7 },
      jawOpen: { low: 0.05, high: 0.8 },
    });
    expect(old).toEqual({
      ...DEFAULT_TUNING,
      yaw: axis({ gain: 1.5, deadZone: 2 }),
      eyeBlink: { low: 0.1, high: 0.7 },
      jawOpen: { low: 0.05, high: 0.8 },
    });
  });

  it("keeps values in range", () => {
    const t = sanitizeTuning({
      yaw: { gain: 100, deadZone: 100 },
      eyeBlink: { low: -1, high: 2 },
      jawOpen: { low: 0.3, high: 0.3 + RANGE_MIN_SPAN },
    });
    expect(t.yaw).toEqual(axis({ gain: 4, deadZone: 20 }));
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

  it("keeps the separate gains in range", () => {
    const t = sanitizeTuning({
      yaw: { gain: 2, separate: true, gainNegative: 9 },
      pitch: { separate: "on", gainNegative: Number.NaN },
      roll: { separate: true, gainNegative: -1 },
    });
    expect(t.yaw).toEqual(axis({ gain: 2, separate: true, gainNegative: 4 }));
    expect(t.pitch).toEqual(axis({}));
    expect(t.roll).toEqual(axis({ separate: true, gainNegative: 0 }));
    // 壊れた負の向きの感度は、分けているときも既定 (1) として扱う。
    const frame = { head: { yaw: -10, pitch: 0, roll: 0 }, blendShapes: {} };
    const broken = {
      ...DEFAULT_TUNING,
      yaw: { gain: 1, deadZone: 0, separate: true, gainNegative: Number.POSITIVE_INFINITY },
    };
    expect(applyTuning(frame, broken)).toBe(frame);
  });

  it("keeps the added ranges and the eyelid switch in range", () => {
    const t = sanitizeTuning({
      eyeBlinkSeparate: "yes",
      eyeBlinkRight: { low: 0.2, high: Number.POSITIVE_INFINITY },
      eyeLook: { low: Number.NaN, high: 0.5 },
      mouthClose: { low: 0, high: 3 },
      mouthForm: { low: 0.9, high: 0.1 },
      brow: "wide",
    });
    expect(t.eyeBlinkSeparate).toBe(false);
    expect(t.eyeBlinkRight).toEqual({ low: 0.2, high: 1 });
    expect(t.eyeLook).toEqual({ low: 0, high: 0.5 });
    expect(t.mouthClose).toEqual({ low: 0, high: 1 });
    expect(t.mouthForm).toEqual({ low: 0, high: 1 });
    expect(t.brow).toEqual({ low: 0, high: 1 });
    expect(sanitizeTuning({ eyeBlinkSeparate: true }).eyeBlinkSeparate).toBe(true);
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
