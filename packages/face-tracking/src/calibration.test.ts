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
