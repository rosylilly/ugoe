import { describe, expect, it } from "vitest";
import { applyCalibration, averageFrames } from "./calibration";

describe("applyCalibration", () => {
  const neutral = {
    head: { yaw: 5, pitch: -10, roll: 2 },
    blendShapes: { eyeBlinkLeft: 0.2, jawOpen: 0 },
  };

  it("subtracts the neutral head pose", () => {
    const out = applyCalibration({ head: { yaw: 5, pitch: 0, roll: 2 }, blendShapes: {} }, neutral);
    expect(out.head).toEqual({ yaw: 0, pitch: 10, roll: 0 });
  });

  it("rescales blend shapes so that neutral is 0 and 1 stays 1", () => {
    const out = applyCalibration(
      { head: neutral.head, blendShapes: { eyeBlinkLeft: 0.6, jawOpen: 0.5, cheekPuff: 0.3 } },
      neutral,
    );
    expect(out.blendShapes.eyeBlinkLeft).toBeCloseTo(0.5);
    expect(out.blendShapes.jawOpen).toBeCloseTo(0.5);
    expect(out.blendShapes.cheekPuff).toBeCloseTo(0.3);
    const closed = applyCalibration(
      { head: neutral.head, blendShapes: { eyeBlinkLeft: 1 } },
      neutral,
    );
    expect(closed.blendShapes.eyeBlinkLeft).toBeCloseTo(1);
    const below = applyCalibration(
      { head: neutral.head, blendShapes: { eyeBlinkLeft: 0.1 } },
      neutral,
    );
    expect(below.blendShapes.eyeBlinkLeft).toBe(0);
  });

  it("passes frames through without a neutral", () => {
    const frame = { head: { yaw: 1, pitch: 2, roll: 3 }, blendShapes: { jawOpen: 0.4 } };
    expect(applyCalibration(frame, null)).toBe(frame);
  });
});

describe("averageFrames", () => {
  it("averages every value", () => {
    const avg = averageFrames([
      { head: { yaw: 0, pitch: 2, roll: 4 }, blendShapes: { jawOpen: 0.2 } },
      { head: { yaw: 10, pitch: 4, roll: 0 }, blendShapes: { jawOpen: 0.4 } },
    ]);
    expect(avg?.head).toEqual({ yaw: 5, pitch: 3, roll: 2 });
    expect(avg?.blendShapes.jawOpen).toBeCloseTo(0.3);
    expect(averageFrames([])).toBeNull();
  });
});

describe("body calibration", () => {
  const body = { yaw: 10, pitch: -15, roll: 3, shoulderLeft: 0.4, shoulderRight: 0.2 };
  const frame = { head: { yaw: 0, pitch: 0, roll: 0 }, blendShapes: {}, body };

  it("keeps only yaw and roll without calibration", () => {
    const out = applyCalibration(frame, null);
    expect(out.body).toEqual({
      yaw: 10,
      pitch: null,
      roll: 3,
      shoulderLeft: null,
      shoulderRight: null,
    });
  });

  it("subtracts the neutral body and drops relative values the neutral lacks", () => {
    const neutral = {
      head: { yaw: 0, pitch: 0, roll: 0 },
      blendShapes: {},
      body: { yaw: 4, pitch: null, roll: 1, shoulderLeft: 0.1, shoulderRight: 0.1 },
    };
    const out = applyCalibration(frame, neutral);
    expect(out.body?.yaw).toBeCloseTo(6);
    expect(out.body?.roll).toBeCloseTo(2);
    // 中立で腰が見えていなかったので、ピッチは使わない。
    expect(out.body?.pitch).toBeNull();
    expect(out.body?.shoulderLeft).toBeCloseTo(0.3);
    expect(out.body?.shoulderRight).toBeCloseTo(0.1);
  });

  it("keeps yaw and roll when the neutral pose had no body", () => {
    const neutral = { head: { yaw: 0, pitch: 0, roll: 0 }, blendShapes: {} };
    const out = applyCalibration(frame, neutral);
    expect(out.body?.yaw).toBe(10);
    expect(out.body?.shoulderLeft).toBeNull();
  });

  it("averages body values over the frames that have them", () => {
    const avg = averageFrames([
      frame,
      { ...frame, body: { ...body, yaw: 20, pitch: null } },
      { head: frame.head, blendShapes: {} },
    ]);
    expect(avg?.body?.yaw).toBeCloseTo(15);
    expect(avg?.body?.pitch).toBeCloseTo(-15);
    expect(averageFrames([{ head: frame.head, blendShapes: {} }])?.body).toBeUndefined();
  });
});
