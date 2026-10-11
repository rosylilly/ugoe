import { DEFAULT_TUNING, type TrackingTuning } from "@ugoe/face-tracking";
import { BrokenStorage, MemoryStorage } from "@ugoe/i18n/testing";
import { describe, expect, it } from "vitest";
import { STORAGE_KEYS } from "./storageKeys";
import { loadTrackingTuning, saveTrackingTuning } from "./trackingTuning";

const KEY = STORAGE_KEYS["tracking.tuning"];

const TUNED: TrackingTuning = {
  ...DEFAULT_TUNING,
  yaw: { gain: 1.5, deadZone: 2 },
  pitch: { gain: 0.8, deadZone: 0 },
  roll: { gain: 1, deadZone: 1 },
  eyeBlink: { low: 0.2, high: 0.7 },
  eyeBlinkSeparate: true,
  eyeBlinkRight: { low: 0.1, high: 0.6 },
  jawOpen: { low: 0.1, high: 0.9 },
  brow: { low: 0, high: 0.4 },
};

describe("remembered tracking tuning", () => {
  it("is the default when nothing is remembered", () => {
    expect(loadTrackingTuning(new MemoryStorage())).toEqual(DEFAULT_TUNING);
  });

  it("comes back as it was saved", () => {
    const storage = new MemoryStorage();
    saveTrackingTuning(TUNED, storage);
    expect(storage.keys()).toEqual([KEY]);
    expect(loadTrackingTuning(storage)).toEqual(TUNED);
  });

  it("forgets the key when set back to the default", () => {
    const storage = new MemoryStorage();
    saveTrackingTuning(TUNED, storage);
    saveTrackingTuning(DEFAULT_TUNING, storage);
    expect(storage.keys()).toEqual([]);
    expect(loadTrackingTuning(storage)).toEqual(DEFAULT_TUNING);
  });

  it("reads broken values as the default", () => {
    const storage = new MemoryStorage();
    for (const text of ["{", "null", "42", '"tuning"', "[]"]) {
      storage.setItem(KEY, text);
      expect(loadTrackingTuning(storage)).toEqual(DEFAULT_TUNING);
    }
  });

  it("clamps out-of-range values and defaults broken or missing items", () => {
    const storage = new MemoryStorage();
    storage.setItem(
      KEY,
      JSON.stringify({
        yaw: { gain: 99, deadZone: -5 },
        pitch: { gain: "fast", deadZone: 3 },
        eyeBlink: { low: 0.9, high: 0.1 },
        jawOpen: { low: 0.3, high: 0.6 },
        unknown: { gain: 2 },
      }),
    );
    expect(loadTrackingTuning(storage)).toEqual({
      ...DEFAULT_TUNING,
      yaw: { gain: 4, deadZone: 0 },
      pitch: { gain: 1, deadZone: 3 },
      jawOpen: { low: 0.3, high: 0.6 },
    });
  });

  it("does not throw when localStorage is unusable", () => {
    const storage = new BrokenStorage();
    expect(() => saveTrackingTuning(TUNED, storage)).not.toThrow();
    expect(loadTrackingTuning(storage)).toEqual(DEFAULT_TUNING);
  });
});
