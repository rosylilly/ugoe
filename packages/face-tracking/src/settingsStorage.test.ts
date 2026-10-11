import { BrokenStorage, MemoryStorage } from "@ugoe/i18n/testing";
import { describe, expect, it } from "vitest";
import {
  applyTrackerSettings,
  DEFAULT_TRACKING_SETTINGS,
  loadStoredSettings,
  saveStoredSettings,
  type TrackingSettings,
  trackerSettings,
} from "./settingsStorage";
import { FaceTracker, type TrackingTarget } from "./tracker.svelte";

// ビューアのキー。
const KEY = "ugoe.viewer.tracking.settings";

const CHANGED: TrackingSettings = {
  smoothing: 0.8,
  swapSides: true,
  stopEyeBlink: false,
  trackBody: false,
};

function newTracker(blinks: boolean[] = []) {
  const target: TrackingTarget = {
    setTracking: () => {},
    setAutoEyeBlink: (enabled) => blinks.push(enabled),
  };
  return new FaceTracker(target, { schedule: () => () => {} });
}

describe("remembered tracking settings", () => {
  it("default to what a new tracker starts with", () => {
    expect(trackerSettings(newTracker())).toEqual(DEFAULT_TRACKING_SETTINGS);
  });

  it("is the default when nothing is remembered", () => {
    expect(loadStoredSettings(KEY, new MemoryStorage())).toEqual(DEFAULT_TRACKING_SETTINGS);
  });

  it("comes back as it was saved", () => {
    const storage = new MemoryStorage();
    saveStoredSettings(KEY, CHANGED, storage);
    expect(storage.keys()).toEqual([KEY]);
    expect(loadStoredSettings(KEY, storage)).toEqual(CHANGED);
  });

  it("forgets the key when set back to the default", () => {
    const storage = new MemoryStorage();
    saveStoredSettings(KEY, CHANGED, storage);
    saveStoredSettings(KEY, DEFAULT_TRACKING_SETTINGS, storage);
    expect(storage.keys()).toEqual([]);
  });

  it("reads broken values as the default", () => {
    const storage = new MemoryStorage();
    for (const text of ["{", "null", "42", '"settings"', "[]", "true"]) {
      storage.setItem(KEY, text);
      expect(loadStoredSettings(KEY, storage)).toEqual(DEFAULT_TRACKING_SETTINGS);
    }
  });

  it("defaults broken items one by one and clamps the smoothing", () => {
    const storage = new MemoryStorage();
    storage.setItem(
      KEY,
      JSON.stringify({ smoothing: 7, swapSides: "yes", trackBody: false, unknown: 1 }),
    );
    expect(loadStoredSettings(KEY, storage)).toEqual({
      ...DEFAULT_TRACKING_SETTINGS,
      smoothing: 1,
      trackBody: false,
    });
    storage.setItem(KEY, JSON.stringify({ smoothing: -1, stopEyeBlink: 0 }));
    expect(loadStoredSettings(KEY, storage)).toEqual({
      ...DEFAULT_TRACKING_SETTINGS,
      smoothing: 0,
    });
    // JSON に NaN は無いが、文字や null は既定値。
    storage.setItem(KEY, JSON.stringify({ smoothing: null, swapSides: true }));
    expect(loadStoredSettings(KEY, storage)).toEqual({
      ...DEFAULT_TRACKING_SETTINGS,
      swapSides: true,
    });
    storage.setItem(KEY, '{"smoothing": 1e999}');
    expect(loadStoredSettings(KEY, storage)).toEqual(DEFAULT_TRACKING_SETTINGS);
  });

  it("does not throw when localStorage is unusable", () => {
    const storage = new BrokenStorage();
    expect(() => saveStoredSettings(KEY, CHANGED, storage)).not.toThrow();
    expect(loadStoredSettings(KEY, storage)).toEqual(DEFAULT_TRACKING_SETTINGS);
  });

  it("are applied to a tracker and read back from it", () => {
    const tracker = newTracker();
    applyTrackerSettings(tracker, CHANGED);
    expect(trackerSettings(tracker)).toEqual(CHANGED);
    applyTrackerSettings(tracker, { ...DEFAULT_TRACKING_SETTINGS, smoothing: Number.NaN });
    expect(trackerSettings(tracker)).toEqual(DEFAULT_TRACKING_SETTINGS);
  });
});
