import { describe, expect, it } from "vitest";
import {
  applyFrame,
  decodeFrame,
  type FollowerPuppet,
  type FollowFrame,
  FrameQueue,
  MAX_PENDING_FRAMES,
} from "./follower";

/** apps/studio/src-tauri/src/stream.rs の `GOLDEN` と同じバイト列。 */
const GOLDEN = new Uint8Array([
  1, 3, 0, 0, 7, 0, 0, 0, 0x2a, 0x01, 0, 0, 0, 0, 0, 0, 0x89, 0x88, 0x88, 0x3c, 0, 0, 0, 0x3f, 0, 0,
  0x40, 0x3f, 2, 0, 0, 0, 0, 0, 0xc0, 0x3f, 0, 0, 0x20, 0xc1,
]);

function frame(seq: number, revision: number, reset = false): FollowFrame {
  return { seq, revision, dt: 1 / 60, reset, values: new Float32Array([seq]), lipSync: null };
}

describe("decodeFrame", () => {
  it("reads the bytes the studio backend writes", () => {
    const f = decodeFrame(GOLDEN.slice().buffer);
    expect(f.seq).toBe(298);
    expect(f.revision).toBe(7);
    expect(f.dt).toBeCloseTo(1 / 60, 7);
    expect(f.reset).toBe(true);
    expect(f.lipSync).toEqual({ level: 0.5, strength: 0.75 });
    expect([...f.values]).toEqual([1.5, -10]);
  });

  it("rejects broken bytes", () => {
    expect(() => decodeFrame(GOLDEN.slice(0, 10).buffer)).toThrow(/too short/);
    const kind = GOLDEN.slice();
    kind[0] = 2;
    expect(() => decodeFrame(kind.buffer)).toThrow(/kind 2/);
    expect(() => decodeFrame(GOLDEN.slice(0, 38).buffer)).toThrow(/2 values/);
    const silent = GOLDEN.slice();
    silent[1] = 0;
    expect(decodeFrame(silent.buffer).lipSync).toBeNull();
  });
});

describe("FrameQueue", () => {
  const seqs = (q: FrameQueue) => q.take().map(({ frame, gap }) => [frame.seq, gap]);

  it("waits for the scene of the frames", () => {
    const q = new FrameQueue();
    q.push(frame(0, 1));
    expect(seqs(q)).toEqual([]);
    q.setScene(1);
    q.push(frame(1, 1));
    expect(seqs(q)).toEqual([
      [0, false],
      [1, false],
    ]);
  });

  it("drops frames of an older scene and holds frames of a newer one", () => {
    const q = new FrameQueue();
    q.setScene(1);
    q.push(frame(0, 1));
    q.push(frame(1, 2));
    q.push(frame(2, 2));
    expect(seqs(q)).toEqual([[0, false]]);
    expect(q.pending).toBe(2);
    q.push(frame(3, 1));
    q.setScene(2);
    expect(seqs(q)).toEqual([
      [1, false],
      [2, false],
    ]);
    expect(q.pending).toBe(0);
  });

  it("marks a gap in seq within a scene", () => {
    const q = new FrameQueue();
    q.setScene(1);
    q.push(frame(4, 1));
    q.push(frame(5, 1));
    q.push(frame(8, 1));
    expect(seqs(q)).toEqual([
      [4, false],
      [5, false],
      [8, true],
    ]);
  });

  it("keeps the frames of the previous scene when continuing with the same model", () => {
    const q = new FrameQueue();
    q.continueScene(1);
    q.push(frame(0, 1));
    expect(seqs(q)).toEqual([[0, false]]);
    // 見せ方が変わったシーン 2 の通知より前に届いたシーン 1 のフレームも、続けて適用する。
    q.push(frame(1, 1));
    q.push(frame(2, 2));
    q.push(frame(3, 3));
    q.continueScene(2);
    expect(seqs(q)).toEqual([
      [1, false],
      [2, false],
    ]);
    // まだ受けていないシーン 3 は待たせ、受けたら seq の続きとして適用する。
    q.continueScene(3);
    expect(seqs(q)).toEqual([[3, false]]);
    // 古い通知が後から来ても戻らない。
    q.continueScene(2);
    q.push(frame(4, 3));
    expect(seqs(q)).toEqual([[4, false]]);
    // モデルを読み直したら、前のシーンのフレームは捨てる。
    q.push(frame(5, 3));
    q.push(frame(6, 4));
    q.setScene(4);
    expect(seqs(q)).toEqual([[6, false]]);
  });

  it("treats overflow as a gap", () => {
    const q = new FrameQueue();
    q.setScene(1);
    for (let i = 0; i < MAX_PENDING_FRAMES + 5; i++) q.push(frame(i, 1));
    const taken = q.take();
    expect(taken).toHaveLength(MAX_PENDING_FRAMES);
    expect(taken[0]).toMatchObject({ gap: true, frame: { seq: 5 } });
    expect(taken[1]?.gap).toBe(false);
  });
});

describe("applyFrame", () => {
  it("resets physics before the frame when asked", () => {
    const calls: string[] = [];
    const puppet: FollowerPuppet = {
      follow: (values, dt, level, strength) =>
        calls.push(`follow ${values[0]} ${dt.toFixed(3)} ${level ?? "-"} ${strength ?? "-"}`),
      reset_physics: () => calls.push("reset"),
    };
    applyFrame(puppet, frame(1, 1));
    applyFrame(puppet, frame(2, 1, true));
    applyFrame(puppet, { ...frame(3, 1), lipSync: { level: 0.25, strength: 1 } }, true);
    expect(calls).toEqual([
      "follow 1 0.017 - -",
      "reset",
      "follow 2 0.017 - -",
      "reset",
      "follow 3 0.017 0.25 1",
    ]);
  });
});
