import { beforeEach, describe, expect, it } from "vitest";
import {
  CAMERA_STORAGE_KEY,
  cameraNotices,
  DEFAULT_CAMERA_SETTINGS,
  guideFor,
  isCameraActive,
  isValidCameraSettings,
  loadCameraSettings,
  saveCameraSettings,
  type VirtualCameraSettings,
} from "./virtualCamera";

class MemoryStorage {
  #items = new Map<string, string>();
  getItem(key: string) {
    return this.#items.get(key) ?? null;
  }
  setItem(key: string, value: string) {
    this.#items.set(key, value);
  }
}

beforeEach(() => {
  Object.defineProperty(globalThis, "localStorage", {
    value: new MemoryStorage(),
    configurable: true,
  });
});

const hd: VirtualCameraSettings = { width: 1920, height: 1080, fps: 60, background: [0, 128, 255] };

describe("settings", () => {
  it("accepts the presets only", () => {
    expect(isValidCameraSettings(hd)).toBe(true);
    expect(isValidCameraSettings(DEFAULT_CAMERA_SETTINGS)).toBe(true);
    expect(isValidCameraSettings({ ...hd, width: 640, height: 480 })).toBe(false);
    expect(isValidCameraSettings({ ...hd, fps: 120 })).toBe(false);
    expect(isValidCameraSettings({ ...hd, background: [0, 0, 256] })).toBe(false);
    expect(isValidCameraSettings(null)).toBe(false);
  });

  it("remembers the settings and falls back to the default", () => {
    expect(loadCameraSettings()).toEqual(DEFAULT_CAMERA_SETTINGS);
    saveCameraSettings(hd);
    expect(loadCameraSettings()).toEqual(hd);
    localStorage.setItem(CAMERA_STORAGE_KEY, "{broken");
    expect(loadCameraSettings()).toEqual(DEFAULT_CAMERA_SETTINGS);
  });
});

describe("notices", () => {
  it("explains why the camera cannot start", () => {
    expect(cameraNotices({ state: "stopped", unavailable: null })).toEqual([]);
    expect(cameraNotices({ state: "stopped", unavailable: "noDevice" })).toEqual([
      { kind: "unavailable", tone: "warning", code: "noDevice", guide: "linux" },
    ]);
    expect(guideFor("windowsTooOld")).toBe("windows");
    expect(guideFor("notRegistered")).toBe("reinstall");
    expect(guideFor("os")).toBeNull();
  });

  it("reports a stopped camera as an error", () => {
    expect(
      cameraNotices({ state: "failed", settings: hd, code: "busy", detail: "in use" }),
    ).toEqual([{ kind: "failed", tone: "danger", code: "busy", detail: "in use", guide: "linux" }]);
  });

  it("reports a missing consumer and a late camera while running", () => {
    const running = { state: "running" as const, settings: hd, device: "/dev/video10" };
    expect(cameraNotices({ ...running, consumer: true, late: false })).toEqual([]);
    expect(cameraNotices({ ...running, consumer: false, late: true }).map((n) => n.kind)).toEqual([
      "noConsumer",
      "late",
    ]);
    expect(isCameraActive({ ...running, consumer: true, late: false })).toBe(true);
    expect(isCameraActive({ state: "starting", settings: hd })).toBe(true);
    expect(isCameraActive({ state: "stopped", unavailable: null })).toBe(false);
  });
});
