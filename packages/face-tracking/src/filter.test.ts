import { describe, expect, it } from "vitest";
import { FrameSmoother, minCutoffForStrength, OneEuroFilter } from "./filter";

describe("OneEuroFilter", () => {
  it("passes the first value and converges to a constant", () => {
    const f = new OneEuroFilter(1, 0);
    expect(f.filter(1, 1 / 60)).toBe(1);
    let y = 0;
    for (let i = 0; i < 600; i++) y = f.filter(5, 1 / 60);
    expect(y).toBeCloseTo(5, 3);
  });

  it("reduces jitter around a still value", () => {
    const f = new OneEuroFilter(0.5, 0);
    let maxDev = 0;
    for (let i = 0; i < 300; i++) {
      const y = f.filter(i % 2 === 0 ? 1 : -1, 1 / 60);
      if (i > 60) maxDev = Math.max(maxDev, Math.abs(y));
    }
    expect(maxDev).toBeLessThan(0.1);
  });

  it("lags less when moving fast with a larger beta", () => {
    const run = (beta: number) => {
      const f = new OneEuroFilter(0.5, beta);
      let y = 0;
      for (let i = 0; i <= 30; i++) y = f.filter(i * 2, 1 / 60); // 120/秒で動く
      return 60 - y;
    };
    expect(run(0.1)).toBeLessThan(run(0));
  });
});

describe("FrameSmoother", () => {
  it("does nothing at strength 0", () => {
    const s = new FrameSmoother(0);
    s.smooth({ head: { yaw: 0, pitch: 0, roll: 0 }, blendShapes: { jawOpen: 0 } }, 1 / 60);
    const out = s.smooth(
      { head: { yaw: 30, pitch: 0, roll: 0 }, blendShapes: { jawOpen: 1 } },
      1 / 60,
    );
    expect(out.head.yaw).toBe(30);
    expect(out.blendShapes.jawOpen).toBe(1);
  });

  it("smooths more with a larger strength", () => {
    const step = (strength: number) => {
      const s = new FrameSmoother(strength);
      s.smooth({ head: { yaw: 0, pitch: 0, roll: 0 }, blendShapes: {} }, 1 / 60);
      return s.smooth({ head: { yaw: 30, pitch: 0, roll: 0 }, blendShapes: {} }, 1 / 60).head.yaw;
    };
    expect(step(1)).toBeLessThan(step(0.3));
    expect(minCutoffForStrength(0)).toBeGreaterThan(minCutoffForStrength(1));
  });
});
