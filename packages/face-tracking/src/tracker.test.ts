import { setLocale } from "@ugoe/i18n";
import { afterEach, describe, expect, it } from "vitest";
import type { TrackingFrame } from "./frame";
import { serializeRecording } from "./recording";
import { FaceTracker, type TrackerSchedule, type TrackingTarget } from "./tracker.svelte";

const RECORDING = serializeRecording([
  { t: 0, head: { yaw: 1, pitch: 2, roll: 3 }, blendShapes: {} },
  { t: 1, head: { yaw: 4, pitch: 5, roll: 6 }, blendShapes: {} },
]);

/** 回している処理の数を数える仕組み (MediaPipe もカメラも使わない記録の再生で確かめる)。 */
function harness() {
  // 同じ `step` が 2 度回されることもあるので、回すたびに別の印で数える。
  const loops = new Set<object>();
  const schedule: TrackerSchedule = () => {
    const loop = {};
    loops.add(loop);
    return () => loops.delete(loop);
  };
  const frames: (TrackingFrame | null)[] = [];
  const target: TrackingTarget = {
    setTracking: (frame) => frames.push(frame),
    setAutoEyeBlink: () => {},
  };
  return { tracker: new FaceTracker(target, { schedule }), loops, frames };
}

afterEach(() => setLocale("system"));

describe("FaceTracker start and stop", () => {
  it("runs a replay", async () => {
    const { tracker, loops } = harness();
    await tracker.startReplay(RECORDING);
    expect(tracker.status).toBe("running");
    expect(loops.size).toBe(1);
    tracker.stop();
    expect(tracker.status).toBe("idle");
    expect(loops.size).toBe(0);
  });

  it("stays stopped when stopped while starting", async () => {
    const { tracker, loops } = harness();
    const started = tracker.startReplay(RECORDING);
    tracker.stop();
    await started;
    expect(tracker.status).toBe("idle");
    expect(tracker.source).toBeNull();
    expect(loops.size).toBe(0);
  });

  it("keeps only the last of overlapping starts", async () => {
    const { tracker, loops } = harness();
    const first = tracker.startReplay(RECORDING, "first");
    const second = tracker.startReplay(RECORDING, "second");
    await Promise.all([first, second]);
    expect(tracker.status).toBe("running");
    expect(tracker.source?.label).toBe("second");
    expect(loops.size).toBe(1);
  });

  it("reports a broken recording in the current language", async () => {
    const { tracker, loops } = harness();
    setLocale("en");
    await tracker.startReplay("{}");
    expect(tracker.status).toBe("error");
    expect(tracker.error).toMatch(/^Cannot read the recording: /);
    expect(loops.size).toBe(0);
    setLocale("ja");
    await tracker.startReplay("{}");
    expect(tracker.error).toMatch(/^記録を読めません: /);
  });
});
