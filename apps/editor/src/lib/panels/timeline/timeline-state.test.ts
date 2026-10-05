// TimelineState の再生・シークが wasm の MotionSampler をどう使うか (索引の作り直しと再生中の呼び出し)。wasm は偽物。

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { EditorDocument } from "../../document.svelte";
import type { Curve, Motion } from "../../ipc";
import type { ViewportController } from "../../viewport/controller.svelte";
import { TimelineState } from "./timeline.svelte";

/** 偽の MotionSampler が受けた呼び出し。 */
let calls: string[] = [];

/**
 * 偽の MotionSampler。本物と同じく、渡されたモーションを読んで持ち (索引の代わり)、`sample` はそれだけで値を出す。
 * 値は各カーブの最初の点の値に時刻を足したもの。
 */
class FakeMotionSampler {
  #motion: Motion;
  constructor(json: string) {
    calls.push("new");
    this.#motion = JSON.parse(json);
  }
  set_motion_json(json: string): void {
    calls.push("set_motion_json");
    const motion = JSON.parse(json) as Motion;
    // 本物は読めないモーションでエラーにし、今のモーションを保つ
    if (motion.name === "broken") throw new Error("broken motion");
    this.#motion = motion;
  }
  sample(t: number): string {
    calls.push("sample");
    return JSON.stringify(
      this.#motion.curves.map((c) => {
        const start = c.segments[0];
        return { target: c.target, value: (start?.type === "start" ? start.p.v : 0) + t };
      }),
    );
  }
  free(): void {
    calls.push("free");
  }
}

vi.mock("../../viewport/wasm", () => ({
  loadUgoeWasm: async () => ({ MotionSampler: FakeMotionSampler }),
}));

function curve(id: string, v: number): Curve {
  return {
    target: { kind: "parameter", id },
    segments: [
      { type: "start", p: { t: 0, v } },
      { type: "linear", to: { t: 1, v } },
    ],
    fade_in: null,
    fade_out: null,
  };
}

function motion(name: string, curves: Curve[]): Motion {
  return { name, duration: 2, looping: false, fade_in: 0, fade_out: 0, curves, events: [] };
}

/** requestAnimationFrame に渡された関数 (テストが好きな時に呼ぶ)。 */
let frames: FrameRequestCallback[] = [];

function setup(motions: Motion[]) {
  const doc = { motions } as unknown as EditorDocument;
  const values: Record<string, number> = {};
  const viewport = {
    setParameter: (id: string, value: number) => {
      values[id] = value;
    },
    parameterValues: values,
  } as unknown as ViewportController;
  const timeline = new TimelineState(doc, viewport);
  return { doc, values, timeline };
}

/** wasm の読み込み (偽物の約束) が終わるのを待つ。 */
const loaded = () => new Promise((resolve) => setTimeout(resolve));

beforeEach(() => {
  calls = [];
  frames = [];
  vi.stubGlobal("requestAnimationFrame", (f: FrameRequestCallback) => frames.push(f));
  vi.stubGlobal("cancelAnimationFrame", () => {});
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("TimelineState と MotionSampler", () => {
  it("同じモーションでは索引を作り直さず sample だけを呼ぶ", async () => {
    const { timeline, values } = setup([motion("idle", [curve("A", 1)])]);
    await loaded();
    timeline.select("idle");
    timeline.seek(0.5);
    timeline.seek(1);
    expect(calls).toEqual(["new", "sample", "sample"]);
    expect(values.A).toBeCloseTo(2);
  });

  it("再生中のフレームは sample だけを呼ぶ", async () => {
    const { timeline, values } = setup([motion("idle", [curve("A", 1)])]);
    await loaded();
    timeline.select("idle");
    timeline.seek(0);
    calls = [];
    timeline.play();
    for (let i = 0; i < 3; i++) frames.shift()?.(performance.now() + 100 * (i + 1));
    expect(calls).toEqual(["sample", "sample", "sample"]);
    expect(values.A).toBeGreaterThan(1);
    timeline.stop();
  });

  it("編集の後は新しいモーションを渡し直し、古い索引の値を返さない", async () => {
    const { doc, timeline, values } = setup([motion("idle", [curve("A", 1)])]);
    await loaded();
    timeline.select("idle");
    timeline.seek(0.5);
    expect(values.A).toBeCloseTo(1.5);

    // 編集 (Command の適用) の後、ドキュメントはモーションの配列を読み直す (同じ名前の新しいオブジェクト)
    doc.motions = [motion("idle", [curve("A", 10), curve("B", 3)])];
    calls = [];
    timeline.applyPose();
    expect(calls).toEqual(["set_motion_json", "sample"]);
    expect(values.A).toBeCloseTo(10.5);
    expect(values.B).toBeCloseTo(3.5);
  });

  it("選んだモーションを変えると渡し直す", async () => {
    const { timeline } = setup([motion("idle", [curve("A", 1)]), motion("walk", [curve("B", 2)])]);
    await loaded();
    timeline.select("idle");
    timeline.seek(0);
    timeline.select("walk");
    expect(timeline.sample(0)).toEqual({ B: 2 });
    expect(calls).toEqual(["new", "sample", "set_motion_json", "sample"]);
  });

  it("渡し直しに失敗したら前のモーションの値を返さず、次の評価で渡し直す", async () => {
    const { doc, timeline } = setup([motion("idle", [curve("A", 1)])]);
    await loaded();
    timeline.select("idle");
    expect(timeline.sample(0)).toEqual({ A: 1 });

    const broken = motion("broken", [curve("A", 5)]);
    doc.motions = [broken];
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    expect(timeline.sample(0, broken)).toEqual({});
    expect(timeline.sample(0, broken)).toEqual({});
    warn.mockRestore();
    expect(calls.filter((c) => c === "set_motion_json")).toHaveLength(2);
  });

  it("dispose で MotionSampler を解放する", async () => {
    const { timeline } = setup([motion("idle", [curve("A", 1)])]);
    await loaded();
    timeline.select("idle");
    timeline.sample(0);
    timeline.dispose();
    expect(calls.at(-1)).toBe("free");
  });
});
