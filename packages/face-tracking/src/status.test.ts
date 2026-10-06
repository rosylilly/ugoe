import { describe, expect, it } from "vitest";
import {
  CONDITION_SETTLE_MS,
  SettledValue,
  type TrackingCondition,
  type TrackingConditionInput,
  trackingCondition,
} from "./status";

const RUNNING: TrackingConditionInput = {
  status: "running",
  source: { kind: "camera" },
  faceFound: true,
  trackBody: true,
  bodyFound: true,
};

describe("trackingCondition", () => {
  it("follows the tracker status", () => {
    expect(trackingCondition({ ...RUNNING, status: "idle" })).toBe("idle");
    expect(trackingCondition({ ...RUNNING, status: "loading" })).toBe("loading");
    expect(trackingCondition({ ...RUNNING, status: "error" })).toBe("stopped");
    expect(trackingCondition(RUNNING)).toBe("tracking");
  });

  it("reports a missing face before a missing body", () => {
    expect(trackingCondition({ ...RUNNING, faceFound: false, bodyFound: false })).toBe("noFace");
    expect(trackingCondition({ ...RUNNING, bodyFound: false })).toBe("noBody");
    expect(trackingCondition({ ...RUNNING, trackBody: false, bodyFound: false })).toBe("tracking");
  });

  it("does not ask for a body in a replay", () => {
    const replay = { ...RUNNING, source: { kind: "replay" as const }, bodyFound: false };
    expect(trackingCondition(replay)).toBe("replaying");
    expect(trackingCondition({ ...replay, faceFound: false })).toBe("noFace");
  });
});

/** 状態の文と同じ設定 (止めた後は待たない)。 */
function settled() {
  return new SettledValue<TrackingCondition>("idle", CONDITION_SETTLE_MS, (c) => c === "idle");
}

describe("SettledValue", () => {
  it("changes after the value lasts for the delay", () => {
    const s = settled();
    expect(s.update("tracking", 0)).toBe("idle");
    expect(s.settlesAt).toBe(1000);
    expect(s.update("tracking", 999)).toBe("idle");
    expect(s.update("tracking", 1000)).toBe("tracking");
    expect(s.settlesAt).toBeNull();
  });

  it("ignores a face that leaves and comes back within the delay", () => {
    const s = settled();
    s.update("tracking", 0);
    s.update("tracking", 1000);
    // 顔が 0.1 秒ずつ出入りする (揺れ)
    const seen = new Set<TrackingCondition>();
    for (let now = 1000; now < 5000; now += 50) {
      const found = Math.floor(now / 100) % 2 === 0;
      seen.add(s.update(found ? "tracking" : "noFace", now));
    }
    expect([...seen]).toEqual(["tracking"]);
    // 0.9 秒の外れも知らせない
    s.update("tracking", 5000);
    s.update("noFace", 5100);
    expect(s.update("noFace", 6000)).toBe("tracking");
    expect(s.update("tracking", 6050)).toBe("tracking");
    expect(s.settlesAt).toBeNull();
    expect(s.update("tracking", 8000)).toBe("tracking");
  });

  it("restarts the wait when the value changes again", () => {
    const s = settled();
    s.update("tracking", 0);
    s.update("tracking", 1000);
    s.update("noFace", 2000);
    s.update("noBody", 2800);
    expect(s.update("noBody", 3500)).toBe("tracking");
    expect(s.settlesAt).toBe(3800);
    expect(s.update("noBody", 3800)).toBe("noBody");
  });

  it("shows an immediate value at once and drops the pending one", () => {
    const s = settled();
    s.update("loading", 0);
    s.update("loading", 1000);
    s.update("tracking", 1200);
    expect(s.update("idle", 1300)).toBe("idle");
    expect(s.settlesAt).toBeNull();
    expect(s.update("idle", 5000)).toBe("idle");
  });
});
