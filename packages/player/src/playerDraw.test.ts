// Player のふつうのモードで、形 (`shape_changed`) も見せ方 (`#dirty`) も変わらないフレームは描かないこと (T0982)。
// wasm と DOM は偽物 (src/testing/)。受け手のモードの描く・描かないは playerFollower.test.ts (T0857)。

import { describe, expect, it, vi } from "vitest";
import { bundle, fakePuppet, moduleFile } from "./testing/fakePuppet";
import { container, pending, player, ready, settle, setupPlayer, step } from "./testing/player";

// 画面の部品と MediaPipe まで読み込むと遅いので、Player が使う関数だけにする。
vi.mock("@ugoe/face-tracking", () => import("./testing/faceTracking"));

setupPlayer();

const model = bundle("a.ugoe", 1);
const parameters = [{ id: "AngleX" }];

/** `requestAnimationFrame` を 1 回ずつ進め、描いたか (`render` を呼んだか) を返す。 */
function frames(state: { renders: number }) {
  let now = 0;
  return (): boolean => {
    const before = state.renders;
    step(now);
    now += 16;
    return state.renders > before;
  };
}

describe("Player draws only when the shape or the look changed (T0982)", () => {
  it("draws a still model once and then skips the frames", async () => {
    const { p, state } = await ready({ parameters });
    await p.load(model);
    const drew = frames(state);
    expect(drew()).toBe(true);
    for (let i = 0; i < 5; i++) expect(drew(), `frame ${i + 2}`).toBe(false);
  });

  it("draws while the shape moves and stops when it settles", async () => {
    const { p, state } = await ready({ parameters });
    await p.load(model);
    const drew = frames(state);
    expect(drew()).toBe(true);
    expect(drew()).toBe(false);
    // モーション・物理などで `update` のたびに形が動く。
    state.moving = true;
    expect(drew()).toBe(true);
    expect(drew()).toBe(true);
    state.moving = false;
    expect(drew()).toBe(false);
  });

  it("draws after a parameter changes", async () => {
    const { p, state } = await ready({ parameters });
    await p.load(model);
    const drew = frames(state);
    drew();
    expect(drew()).toBe(false);
    p.setParameter("AngleX", 0.5);
    expect(drew()).toBe(true);
    expect(drew()).toBe(false);
    p.resetAllParameters();
    expect(drew()).toBe(true);
    expect(drew()).toBe(false);
  });

  it("draws after a change of the look", async () => {
    const { p, state } = await ready({ parameters, pose: [["HandsDown", "Wave"]] });
    await p.load(model);
    const drew = frames(state);
    drew();
    expect(drew()).toBe(false);
    const changes: [string, () => void][] = [
      ["zoom", () => p.zoomBy(2)],
      ["pan", () => p.panBy(3, 4)],
      ["fit", () => p.fit()],
      ["size", () => p.resize(320, 240)],
      [
        "render scale",
        () => {
          p.renderScale = 0.5;
        },
      ],
      ["background", () => p.setBackground("#000000", true)],
      [
        "lighting",
        () =>
          p.updateLighting(
            (s) => {
              s.ambient.intensity = 0.5;
            },
            { preview: true },
          ),
      ],
      ["module", () => p.loadModule(moduleFile("hat", "head"))],
      ["detach module", () => p.detachModule("hat")],
      ["pose", () => p.showPose("Wave", { immediately: true })],
    ];
    for (const [what, change] of changes) {
      change();
      expect(drew(), what).toBe(true);
      expect(drew(), `${what} (again)`).toBe(false);
    }
  });

  it("draws after the device pixel ratio changes", async () => {
    const queries: EventTarget[] = [];
    globalThis.matchMedia = () => {
      const target = new EventTarget();
      queries.push(target);
      return target as unknown as MediaQueryList;
    };
    globalThis.devicePixelRatio = 1;
    try {
      const { p, state } = await ready({ parameters });
      await p.load(model);
      const drew = frames(state);
      drew();
      expect(drew()).toBe(false);
      globalThis.devicePixelRatio = 2;
      queries.at(-1)?.dispatchEvent(new Event("change"));
      expect(drew()).toBe(true);
      expect(drew()).toBe(false);
    } finally {
      globalThis.devicePixelRatio = 1;
      delete (globalThis as { matchMedia?: unknown }).matchMedia;
    }
  });

  it("draws on the recreated WebPuppet after the GPU is lost", async () => {
    const p = player();
    const attached = p.attach(container);
    await settle();
    const first = fakePuppet({ parameters });
    pending[0]?.resolve(first.puppet);
    await attached;
    await p.load(model);
    let now = 0;
    const tick = () => {
      step(now);
      now += 16;
    };
    tick();
    tick();
    expect(first.state.renders).toBe(1);
    const lost = pending[0]?.canvas;
    lost?.dispatchEvent(new Event("webglcontextlost", { cancelable: true }));
    lost?.dispatchEvent(new Event("webglcontextrestored"));
    await settle();
    const second = fakePuppet({ parameters });
    pending[1]?.resolve(second.puppet);
    await settle();
    expect(second.loaded).toEqual([model.bytes]);
    // 作り直した WebPuppet の形が変わっていなくても (`shape_changed` が偽でも)、新しい canvas は空なので描く。
    second.state.shapeChanged = false;
    tick();
    tick();
    expect(second.state.renders).toBe(1);
  });

  it("draws once without a model and then skips the frames", async () => {
    const { state } = await ready();
    const drew = frames(state);
    expect(drew()).toBe(true);
    expect(drew()).toBe(false);
  });

  it("captures the current state even after frames that were not drawn", async () => {
    const { p, state } = await ready({ parameters });
    await p.load(model);
    const drew = frames(state);
    drew();
    for (let i = 0; i < 3; i++) expect(drew()).toBe(false);
    // 描かなかったフレームの後でも、読み戻しは描き直さずに今の状態の画を返す。
    const renders = state.renders;
    const still = await p.capture();
    expect(still?.pixels[0]).toBe(100);
    // パラメータを変えて、まだ描いていないフレームでも今の値の画になる。
    p.setParameter("AngleX", 0.5);
    const changed = await p.capture();
    expect(changed?.pixels[0]).toBe(150);
    expect(state.renders).toBe(renders);
    expect(drew()).toBe(true);
  });
});
