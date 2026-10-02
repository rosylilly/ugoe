import { describe, expect, it } from "vitest";
import { follow, gain, loudness, rms, targetLevel } from "./lipSync";

describe("lip sync from the microphone", () => {
  it("measures the loudness in dB", () => {
    expect(rms([])).toBe(0);
    expect(rms([1, -1, 1, -1])).toBeCloseTo(1);
    expect(rms([0.5, -0.5])).toBeCloseTo(0.5);
    expect(loudness(0)).toBe(0);
    expect(loudness(1)).toBeCloseTo(1);
    // -30 dB は真ん中。
    expect(loudness(10 ** (-30 / 20))).toBeCloseTo(0.5);
    expect(loudness(1e-6)).toBe(0);
    expect(loudness(Number.NaN)).toBe(0);
  });

  it("gates the noise and applies the sensitivity", () => {
    const tuning = { sensitivity: 0.5, noiseGate: 0.3 };
    expect(gain(0.5)).toBeCloseTo(1);
    expect(gain(0)).toBeCloseTo(0.25);
    expect(gain(1)).toBeCloseTo(4);
    expect(targetLevel(0.2, tuning)).toBe(0);
    expect(targetLevel(0.3, tuning)).toBe(0);
    expect(targetLevel(0.65, tuning)).toBeCloseTo(0.5);
    expect(targetLevel(1, tuning)).toBeCloseTo(1);
    expect(targetLevel(0.65, { sensitivity: 1, noiseGate: 0.3 })).toBe(1);
    expect(targetLevel(0.9, { sensitivity: 0.5, noiseGate: 1 })).toBe(0);
  });

  it("opens fast and closes slowly", () => {
    const opened = follow(0, 1, 1 / 60);
    const closed = follow(1, 0, 1 / 60);
    expect(opened).toBeGreaterThan(0.3);
    expect(1 - closed).toBeLessThan(opened);
    expect(follow(0.4, 1, 0)).toBe(0.4);
    // 長く待てば目標に着く。
    expect(follow(1, 0, 10)).toBeCloseTo(0);
  });
});
