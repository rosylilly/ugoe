import { setLocale } from "@ugoe/i18n";
import { afterEach, describe, expect, it } from "vitest";
import type { TrackingFrame } from "./frame";
import { parseRecording, serializeRecording } from "./recording";
import { FaceTracker, type TrackerSchedule, type TrackingTarget } from "./tracker.svelte";
import { DEFAULT_TUNING } from "./tuning";

const RECORDING = serializeRecording([
  { t: 0, head: { yaw: 1, pitch: 2, roll: 3 }, blendShapes: {} },
  { t: 1, head: { yaw: 4, pitch: 5, roll: 6 }, blendShapes: {} },
]);

/** 回している処理の数を数える仕組み (MediaPipe もカメラも使わない記録の再生で確かめる)。 */
function harness() {
  // 同じ `step` が 2 度回されることもあるので、回すたびに別の印で数える。
  const loops = new Set<object>();
  /** 最後に回し始めた処理 (手で刻むため)。 */
  let step: ((now: number) => void) | null = null;
  const schedule: TrackerSchedule = (s) => {
    const loop = {};
    loops.add(loop);
    step = s;
    return () => loops.delete(loop);
  };
  const frames: (TrackingFrame | null)[] = [];
  const target: TrackingTarget = {
    setTracking: (frame) => frames.push(frame),
    setAutoEyeBlink: () => {},
  };
  const tick = (now: number) => step?.(now);
  return { tracker: new FaceTracker(target, { schedule }), loops, frames, tick };
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

describe("FaceTracker tuning", () => {
  // 1 秒目から頭が 6° 回り、口が開く。
  const MOVING = serializeRecording([
    { t: 0, head: { yaw: 10, pitch: 0, roll: 0 }, blendShapes: { jawOpen: 0.2 } },
    { t: 1, head: { yaw: 16, pitch: 0, roll: 0 }, blendShapes: { jawOpen: 0.5 } },
    { t: 2, head: { yaw: 16, pitch: 0, roll: 0 }, blendShapes: { jawOpen: 0.5 } },
  ]);

  /** 100 ms ずつ刻んで 1.1 秒目まで進める。 */
  function advance(tick: (now: number) => void) {
    for (let i = 1; i <= 11; i++) tick(i * 100);
  }

  it("applies the tuning after calibration and before smoothing, and records the values before it", async () => {
    const { tracker, frames, tick } = harness();
    tracker.smoothing = 0;
    await tracker.startReplay(MOVING);
    tick(0);
    tracker.calibrate();
    tracker.setTuning({
      ...DEFAULT_TUNING,
      yaw: { gain: 2, deadZone: 1 },
      jawOpen: { low: 0, high: 0.5 },
    });
    tracker.startRecording();
    advance(tick);
    const out = frames.at(-1);
    // 中立 (10°) を引いた 6° から、遊び 1° を引いて 2 倍。
    expect(out?.head.yaw).toBeCloseTo(10);
    // 中立 (0.2) を差し引いて伸ばした 0.375 を、0〜0.5 の境目で伸ばす。
    expect(out?.blendShapes.jawOpen).toBeCloseTo(0.75);
    expect(tracker.tuningPreview?.before.head.yaw).toBeCloseTo(6);
    expect(tracker.tuningPreview?.after.head.yaw).toBeCloseTo(10);
    // 記録は調整 (とキャリブレーション) の前の値。
    const json = tracker.recordingJson();
    expect(json).not.toBeNull();
    expect(parseRecording(json as string).frames.at(-1)?.head.yaw).toBe(16);
    tracker.stop();
    expect(tracker.tuningPreview).toBeNull();
  });

  it("passes the calibrated values through with the default tuning", async () => {
    const { tracker, frames, tick } = harness();
    tracker.smoothing = 0;
    await tracker.startReplay(MOVING);
    tick(0);
    advance(tick);
    const out = frames.at(-1);
    expect(out?.head.yaw).toBe(16);
    expect(out?.blendShapes.jawOpen).toBe(0.5);
    expect(tracker.tuningPreview?.after).toBe(tracker.tuningPreview?.before);
  });

  it("sanitizes the tuning it is given", () => {
    const { tracker } = harness();
    tracker.setTuning({
      ...DEFAULT_TUNING,
      pitch: { gain: Number.NaN, deadZone: 50 },
      eyeBlink: { low: 0.9, high: 0.1 },
    });
    expect(tracker.tuning).toEqual({
      ...DEFAULT_TUNING,
      pitch: { gain: 1, deadZone: 20 },
    });
  });
});
