import { BrokenStorage, MemoryStorage } from "@ugoe/i18n/testing";
import { describe, expect, it } from "vitest";
import { DEFAULT_TUNING, type TrackingTuning } from "./tuning";
import { loadStoredTuning, saveStoredTuning } from "./tuningStorage";

// ビューアのキー (アプリの決めたキーをそのまま使い、ビューアが覚えた値はそのまま読める)。
const KEY = "ugoe.viewer.tracking.tuning";

const loadTrackingTuning = (storage: Storage) => loadStoredTuning(KEY, storage);
const saveTrackingTuning = (tuning: TrackingTuning, storage: Storage) =>
  saveStoredTuning(KEY, tuning, storage);

const TUNED: TrackingTuning = {
  ...DEFAULT_TUNING,
  yaw: { gain: 1.5, deadZone: 2, separate: true, gainNegative: 2.5 },
  pitch: { ...DEFAULT_TUNING.pitch, gain: 0.8 },
  roll: { ...DEFAULT_TUNING.roll, deadZone: 1 },
  eyeBlink: { low: 0.2, high: 0.7 },
  eyeBlinkSeparate: true,
  eyeBlinkRight: { low: 0.1, high: 0.6 },
  jawOpen: { low: 0.1, high: 0.9 },
  brow: { low: 0, high: 0.4 },
  eyeWideSeparate: true,
  eyeWideRight: { low: 0.1, high: 0.5 },
  mouthSide: { low: 0, high: 0.6 },
  cheekPuff: { low: 0.05, high: 0.3 },
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
        // T0877 の形の軸 (感度を向きで分けない)。
        yaw: { gain: 99, deadZone: -5 },
        pitch: { gain: "fast", deadZone: 3 },
        eyeBlink: { low: 0.9, high: 0.1 },
        jawOpen: { low: 0.3, high: 0.6 },
        unknown: { gain: 2 },
      }),
    );
    expect(loadTrackingTuning(storage)).toEqual({
      ...DEFAULT_TUNING,
      yaw: { ...DEFAULT_TUNING.yaw, gain: 4 },
      pitch: { ...DEFAULT_TUNING.pitch, deadZone: 3 },
      jawOpen: { low: 0.3, high: 0.6 },
    });
  });

  it("reads the T0881 shape without the cheeks, the mouth sides and the wide eyes", () => {
    const storage = new MemoryStorage();
    const old = Object.fromEntries(
      Object.entries({
        ...DEFAULT_TUNING,
        eyeLookSeparate: true,
        eyeLookDown: { low: 0, high: 0.7 },
      }).filter(([key]) => !/^(eyeWide|mouthSide|cheekPuff)/.test(key)),
    );
    storage.setItem(KEY, JSON.stringify(old));
    expect(loadTrackingTuning(storage)).toEqual({
      ...DEFAULT_TUNING,
      eyeLookSeparate: true,
      eyeLookDown: { low: 0, high: 0.7 },
    });
  });

  it("keeps each key apart", () => {
    const storage = new MemoryStorage();
    saveStoredTuning("ugoe.editor.trackingTuning", TUNED, storage);
    expect(storage.keys()).toEqual(["ugoe.editor.trackingTuning"]);
    expect(loadStoredTuning(KEY, storage)).toEqual(DEFAULT_TUNING);
    expect(loadStoredTuning("ugoe.editor.trackingTuning", storage)).toEqual(TUNED);
  });

  it("does not throw when localStorage is unusable", () => {
    const storage = new BrokenStorage();
    expect(() => saveTrackingTuning(TUNED, storage)).not.toThrow();
    expect(loadTrackingTuning(storage)).toEqual(DEFAULT_TUNING);
  });
});
