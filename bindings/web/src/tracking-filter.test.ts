import { describe, expect, it } from "vitest";
import type { TrackingInput } from "./tracking.js";
import { calibrateTracking, captureNeutral, TrackingFilter } from "./tracking-filter.js";

const input = (yaw: number, jaw: number, body?: TrackingInput["body"]): TrackingInput => ({
  blendshapes: { jawOpen: jaw },
  head: { yaw, pitch: 0, roll: 0 },
  ...(body ? { body } : {}),
});

/** 決定的な揺れ (-1〜1)。 */
const noise = (i: number) => Math.sin(i * 12.9898) * 0.5 + Math.sin(i * 78.233) * 0.5;

describe("TrackingFilter", () => {
  it("passes the first input through and keeps the input untouched", () => {
    const filter = new TrackingFilter();
    const first = input(10, 0.5, { yaw: 3 });
    const out = filter.apply(first, 0);
    expect(out).toEqual(first);
    expect(out).not.toBe(first);
  });

  it("reduces jitter around a still pose", () => {
    const filter = new TrackingFilter();
    let rawSpread = 0;
    let outSpread = 0;
    for (let i = 0; i < 300; i++) {
      const raw = 5 + noise(i);
      const out = filter.apply(input(raw, 0.3 + noise(i + 7) * 0.05), (i * 1000) / 60);
      if (i >= 60) {
        rawSpread = Math.max(rawSpread, Math.abs(raw - 5));
        outSpread = Math.max(outSpread, Math.abs(out.head.yaw - 5));
      }
    }
    expect(outSpread).toBeLessThan(rawSpread * 0.3);
  });

  it("follows fast movements with little lag", () => {
    const slow = new TrackingFilter({ rotation: { beta: 0 } });
    const fast = new TrackingFilter();
    let lagSlow = 0;
    let lagFast = 0;
    for (let i = 0; i <= 30; i++) {
      const yaw = i * 2; // 120 度/秒
      const t = (i * 1000) / 60;
      lagSlow = yaw - slow.apply(input(yaw, 0), t).head.yaw;
      lagFast = yaw - fast.apply(input(yaw, 0), t).head.yaw;
    }
    expect(lagFast).toBeGreaterThan(0);
    expect(lagFast).toBeLessThan(lagSlow * 0.5);
  });

  it("converges to a constant input", () => {
    const filter = new TrackingFilter();
    filter.apply(input(0, 0), 0);
    let out = input(0, 0);
    for (let i = 1; i <= 120; i++) out = filter.apply(input(20, 1), (i * 1000) / 60);
    expect(out.head.yaw).toBeCloseTo(20, 3);
    expect(out.blendshapes.jawOpen).toBeCloseTo(1, 3);
  });

  it("forgets values that are missing and restarts after reset", () => {
    const filter = new TrackingFilter();
    filter.apply(input(0, 0, { yaw: 0, shoulderLeft: 0 }), 0);
    // body.yaw が途切れた後に戻ると、その値から始める
    const gap = filter.apply(input(0, 0, { shoulderLeft: 0 }), 16);
    expect(gap.body).toEqual({ shoulderLeft: 0 });
    expect(filter.apply(input(0, 0, { yaw: 30 }), 33).body?.yaw).toBe(30);
    filter.reset();
    expect(filter.apply(input(40, 0.9), 50).head.yaw).toBe(40);
  });

  it("does not break when time does not advance", () => {
    const filter = new TrackingFilter();
    filter.apply(input(0, 0), 100);
    const out = filter.apply(input(10, 0), 100);
    expect(Number.isFinite(out.head.yaw)).toBe(true);
    expect(out.head.yaw).toBeGreaterThan(0);
    expect(out.head.yaw).toBeLessThan(10);
  });
});

describe("calibration", () => {
  it("averages the neutral inputs", () => {
    const neutral = captureNeutral([input(4, 0.1, { yaw: 2 }), input(6, 0.3)]);
    expect(neutral).toEqual({
      head: { yaw: 5, pitch: 0, roll: 0 },
      blendshapes: { jawOpen: 0.2 },
      body: { yaw: 2 },
    });
    expect(captureNeutral([])).toBeNull();
    expect(captureNeutral(input(1, 0))?.head.yaw).toBe(1);
  });

  it("subtracts the neutral head and rescales blendshapes", () => {
    const neutral = captureNeutral(input(10, 0.2));
    if (!neutral) throw new Error("no neutral");
    const out = calibrateTracking(
      { blendshapes: { jawOpen: 0.6, eyeBlinkLeft: 0.4 }, head: { yaw: 25, pitch: 3, roll: -170 } },
      { ...neutral, head: { yaw: 10, pitch: 0, roll: 175 } },
    );
    expect(out.head.yaw).toBeCloseTo(15);
    expect(out.head.pitch).toBeCloseTo(3);
    expect(out.head.roll).toBeCloseTo(15); // -170 − 175 を −180〜180 に
    expect(out.blendshapes.jawOpen).toBeCloseTo(0.5); // (0.6 − 0.2) / 0.8
    expect(out.blendshapes.eyeBlinkLeft).toBe(0.4); // 中立に無い名前はそのまま
    expect(calibrateTracking(input(0, 0.1), neutral).blendshapes.jawOpen).toBe(0);
  });

  it("drops body pitch and shoulders without a neutral value", () => {
    const body = { yaw: 10, pitch: -15, roll: 4, shoulderLeft: 0.3, shoulderRight: 0.2 };
    const without = calibrateTracking(input(0, 0, body), {
      head: { yaw: 0, pitch: 0, roll: 0 },
      blendshapes: {},
    });
    expect(without.body).toEqual({ yaw: 10, roll: 4 });
    const withNeutral = calibrateTracking(input(0, 0, body), {
      head: { yaw: 0, pitch: 0, roll: 0 },
      blendshapes: {},
      body: { yaw: 5, pitch: -20, shoulderLeft: 0.1, shoulderRight: 0.1 },
    });
    expect(withNeutral.body?.yaw).toBeCloseTo(5);
    expect(withNeutral.body?.pitch).toBeCloseTo(5);
    expect(withNeutral.body?.roll).toBeCloseTo(4);
    expect(withNeutral.body?.shoulderLeft).toBeCloseTo(0.2);
    expect(withNeutral.body?.shoulderRight).toBeCloseTo(0.1);
  });
});

describe("TrackingFilter options", () => {
  it("uses the defaults for options given as undefined", () => {
    const filter = new TrackingFilter({ rotation: { minCutoff: undefined, beta: undefined } });
    filter.apply(input(0, 0), 0);
    const out = filter.apply(input(10, 0), 16);
    expect(Number.isFinite(out.head.yaw)).toBe(true);
    expect(out.head.yaw).toBeGreaterThan(0);
  });
});
