import { describe, expect, it } from "vitest";
import { frameAt, parseRecording, recordingDuration, serializeRecording } from "./recording";

const frames = [
  { t: 0, head: { yaw: 1, pitch: 2, roll: 3 }, blendShapes: { jawOpen: 0.1 } },
  { t: 0.5, head: { yaw: 4, pitch: 5, roll: 6 }, blendShapes: { jawOpen: 0.123456 } },
];

describe("recording", () => {
  it("round-trips through JSON", () => {
    const rec = parseRecording(serializeRecording(frames));
    expect(rec.frames).toHaveLength(2);
    expect(rec.frames[1]?.blendShapes.jawOpen).toBeCloseTo(0.1235, 6);
    expect(recordingDuration(rec)).toBe(0.5);
  });

  it("rejects other formats and broken frames", () => {
    expect(() => parseRecording("{}")).toThrow();
    expect(() =>
      parseRecording('{"format":"ugoe-face-tracking","version":99,"frames":[]}'),
    ).toThrow();
    expect(() =>
      parseRecording('{"format":"ugoe-face-tracking","version":1,"frames":[{"t":0}]}'),
    ).toThrow();
    const reversed = serializeRecording([...frames].reverse());
    expect(() => parseRecording(reversed)).toThrow();
  });

  it("holds the last frame at or before t", () => {
    const rec = parseRecording(serializeRecording(frames));
    expect(frameAt(rec, -1)?.head.yaw).toBe(1);
    expect(frameAt(rec, 0.49)?.head.yaw).toBe(1);
    expect(frameAt(rec, 0.5)?.head.yaw).toBe(4);
    expect(frameAt(rec, 10)?.head.yaw).toBe(4);
  });
});
