import { describe, expect, it } from "vitest";
import { FrameInterval, FrameSmoother, minCutoffForStrength, OneEuroFilter } from "./filter";

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

describe("FrameSmoother body", () => {
  const frame = (yaw: number | null) => ({
    head: { yaw: 0, pitch: 0, roll: 0 },
    blendShapes: {},
    body: { yaw, pitch: null, roll: 0, shoulderLeft: 0.5, shoulderRight: null },
  });

  it("smooths body values and keeps missing ones missing", () => {
    const smoother = new FrameSmoother(1);
    smoother.smooth(frame(0), 1 / 60);
    const out = smoother.smooth(frame(10), 1 / 60);
    expect(out.body?.yaw).toBeGreaterThan(0);
    expect(out.body?.yaw).toBeLessThan(10);
    expect(out.body?.pitch).toBeNull();
    expect(out.body?.shoulderRight).toBeNull();
  });

  it("forgets a body value while it is missing", () => {
    const smoother = new FrameSmoother(1);
    smoother.smooth(frame(0), 1 / 60);
    expect(smoother.smooth(frame(null), 1 / 60).body?.yaw).toBeNull();
    // 戻ったときは古い値 (0) から追いかけない。
    expect(smoother.smooth(frame(10), 1 / 60).body?.yaw).toBe(10);
  });
});

describe("FrameInterval", () => {
  it("measures from the previous value, not from the previous tick", () => {
    // 60/s の刻みで、30fps のカメラの新しいフレームは 1 刻みおきにしか出ない。値が出たときだけ測る。
    const interval = new FrameInterval(0.1);
    const dts: number[] = [];
    for (let tick = 0; tick <= 8; tick++) {
      const now = (tick * 1000) / 60;
      if (tick % 2 === 0) dts.push(interval.next(now));
    }
    expect(dts[0]).toBe(0);
    for (const dt of dts.slice(1)) expect(dt).toBeCloseTo(1 / 30, 9);
  });

  it("matches the tick interval when every tick has a value", () => {
    // requestAnimationFrame で回し、毎回値が出る (ビューア・エディタの再生や 60fps の入力) なら、これまでと同じ dt。
    const interval = new FrameInterval(0.1);
    interval.next(0);
    for (let tick = 1; tick <= 5; tick++) {
      expect(interval.next((tick * 1000) / 60)).toBeCloseTo(1 / 60, 9);
    }
  });

  it("caps the step and starts over after a reset", () => {
    const interval = new FrameInterval(0.1);
    interval.next(0);
    expect(interval.next(200)).toBe(0.1);
    interval.reset();
    expect(interval.next(300)).toBe(0);
    expect(interval.next(350)).toBeCloseTo(0.05, 9);
  });
});
