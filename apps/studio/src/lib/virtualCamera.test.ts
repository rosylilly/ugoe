import { describe, expect, it } from "vitest";
import {
  cameraNotices,
  guideFor,
  isCameraActive,
  type VirtualCameraSettings,
} from "./virtualCamera";

const hd: VirtualCameraSettings = { width: 1920, height: 1080, fps: 60, background: [0, 128, 255] };

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
