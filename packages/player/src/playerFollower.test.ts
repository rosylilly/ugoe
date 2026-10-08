// Player の受け手のモード (スタジオの出力ウィンドウ。フレームの順と適用・覚えるものを書かない。T0931)。
// wasm と DOM は偽物 (src/testing/)。

import { MemoryStorage } from "@ugoe/i18n/testing";
import { describe, expect, it, vi } from "vitest";
import { idleMemory } from "./animation";
import type { FollowFrame, FollowRenderStats } from "./follower";
import { t } from "./i18n";
import { modelIdentity } from "./modelMemory";
import { PlayerStorage } from "./storage";
import { bundle, fakePuppet, moduleFile } from "./testing/fakePuppet";
import {
  container,
  moduleRows,
  pending,
  player,
  ready,
  settle,
  setupPlayer,
  step,
} from "./testing/player";

// 画面の部品と MediaPipe まで読み込むと遅いので、Player が使う関数だけにする。
vi.mock("@ugoe/face-tracking", () => import("./testing/faceTracking"));

setupPlayer();

/** 受け手のフレーム (値は 1 つ)。 */
function followFrame(seq: number, revision: number, value: number): FollowFrame {
  return {
    seq,
    revision,
    dt: 0.5,
    reset: false,
    values: new Float32Array([value]),
    lipSync: null,
    partOpacityParts: new Uint32Array(),
    partOpacityValues: new Float32Array(),
  };
}

/** 受け手の適用 (と描画) の呼び出しだけ。 */
function followLog(log: string[], render = false): string[] {
  return log.filter(
    (l) => l.startsWith("follow") || l === "reset_physics" || (render && l === "render"),
  );
}

describe("Player follower mode", () => {
  const first = bundle("a.ugoe", 1);
  const second = bundle("b.ugoe", 2);

  it("applies frames that arrived before the model was read in order once the scene is followed", async () => {
    const p = player();
    p.startFollowing();
    expect(p.following).toBe(true);
    // 前のシーンのフレームと、WebPuppet を作っている間に届いたフレーム。
    p.pushFrame(followFrame(1, 0, 9));
    const attached = p.attach(container);
    await settle();
    p.pushFrame(followFrame(5, 1, 1));
    const fake = fakePuppet();
    pending[0]?.resolve(fake.puppet);
    await attached;
    // モデルを読む前に届いたフレーム。
    p.pushFrame(followFrame(6, 1, 2));
    await p.load(first);
    expect(followLog(fake.log)).toEqual([]);

    p.followScene(1);
    expect(followLog(fake.log)).toEqual(["follow 1 0.5", "follow 2 0.5"]);
    // 同じシーンのフレームは届いたその場で適用する。seq が飛んだら物理をリセットしてから。
    p.pushFrame(followFrame(7, 1, 3));
    p.pushFrame(followFrame(9, 1, 4));
    expect(followLog(fake.log).slice(2)).toEqual(["follow 3 0.5", "reset_physics", "follow 4 0.5"]);

    // まだ読んでいない次のシーンのフレームは、そのモデルを読み終えるまで待たせる。
    p.pushFrame(followFrame(1, 2, 5));
    p.pushFrame(followFrame(2, 2, 6));
    expect(followLog(fake.log)).toHaveLength(5);
    await p.load(second);
    expect(followLog(fake.log)).toHaveLength(5);
    p.followScene(2);
    expect(followLog(fake.log).slice(5)).toEqual(["follow 5 0.5", "follow 6 0.5"]);
  });

  it("applies the waiting frames before drawing when the model was read after the scene was followed", async () => {
    const p = player();
    p.startFollowing();
    // WebPuppet ができる前に読んだモデルは、できたときに読む。
    await p.load(first);
    p.followScene(1);
    p.pushFrame(followFrame(1, 1, 1));
    const attached = p.attach(container);
    await settle();
    const fake = fakePuppet();
    pending[0]?.resolve(fake.puppet);
    await attached;
    expect(fake.loaded).toEqual([first.bytes]);
    expect(followLog(fake.log)).toEqual([]);
    step(0);
    expect(followLog(fake.log, true)).toEqual(["follow 1 0.5", "render"]);
  });

  it("continues the frames of the previous scene when the model is kept", async () => {
    const { p, log } = await ready();
    p.startFollowing();
    await p.load(first);
    p.followScene(1);
    p.pushFrame(followFrame(1, 1, 1));
    p.pushFrame(followFrame(2, 2, 2));
    p.pushFrame(followFrame(1, 1, 9));
    p.continueScene(2);
    // 前のシーンのフレームが後から届いても、モデルを読んだシーン以降なので適用する (seq が戻ったので物理はリセット)。
    expect(followLog(log)).toEqual([
      "follow 1 0.5",
      "follow 2 0.5",
      "reset_physics",
      "follow 9 0.5",
    ]);
  });

  it("remaps the part numbers of the previous scene's frames after dressing up (T0853)", async () => {
    const { p, log } = await ready({
      parts: ["Root"],
      moduleParts: { LongHair: ["Hair"], Dress: ["Dress"] },
    });
    p.startFollowing();
    await p.load(first);
    p.loadModule(moduleFile("Dress", "Outfit"), true);
    p.followScene(1, ["Root", "Dress/Dress"]);
    // 着せ替えの前のシーンのフレーム (Dress/Dress の番号 1) が、着せ替えたシーンを受けた後に届く。
    const late = {
      ...followFrame(1, 1, 1),
      partOpacityParts: new Uint32Array([1]),
      partOpacityValues: new Float32Array([0.5, 1]),
    };
    p.unloadModule("Dress");
    p.loadModule(moduleFile("LongHair", "Hair"), true);
    p.loadModule(moduleFile("Dress", "Outfit"), true);
    p.continueScene(2, ["Root", "LongHair/Hair", "Dress/Dress"]);
    p.pushFrame(late);
    p.pushFrame({
      ...followFrame(2, 2, 2),
      partOpacityParts: new Uint32Array([2]),
      partOpacityValues: new Float32Array([0.5, 1]),
    });
    expect(followLog(log)).toEqual(["follow 1 0.5 [2]", "follow 2 0.5 [2]"]);
  });

  it("remaps the part numbers to its own model when a module could not be read (T1400)", async () => {
    const { p, log } = await ready({
      parts: ["Root"],
      moduleParts: { LongHair: ["Hair"], Dress: ["Dress"], Hat: ["Hat"] },
    });
    p.startFollowing();
    await p.load(first);
    // 指揮役は LongHair・Dress・Hat を付けたが、受け手は LongHair を読めずに飛ばした (Dress/Dress は番号 1、Hat/Hat は 2)。
    p.loadModule(moduleFile("Dress", "Outfit"), true);
    p.loadModule(moduleFile("Hat", "Head"), true);
    p.followScene(1, ["Root", "LongHair/Hair", "Dress/Dress", "Hat/Hat"]);
    p.pushFrame({
      ...followFrame(1, 1, 1),
      partOpacityParts: new Uint32Array([2, 1]),
      partOpacityValues: new Float32Array([0.5, 1, 0.25, 1]),
    });
    // 指揮役の Dress/Dress (2) は受け手の 1 に当たり、受け手に無い LongHair/Hair (1) は捨てる。
    expect(followLog(log)).toEqual(["follow 1 0.5 [1]"]);

    // 次のシーンで LongHair を付け直した (付け直した後にシーンに移る)。前のシーンのフレームも今の並びに引き直す。
    p.unloadModule("Dress");
    p.unloadModule("Hat");
    for (const name of ["LongHair", "Dress", "Hat"]) p.loadModule(moduleFile(name, name), true);
    p.pushFrame({
      ...followFrame(2, 1, 2),
      partOpacityParts: new Uint32Array([3]),
      partOpacityValues: new Float32Array([0.5, 1]),
    });
    p.continueScene(2, ["Root", "LongHair/Hair", "Dress/Dress", "Hat/Hat"]);
    expect(followLog(log)).toEqual(["follow 1 0.5 [1]", "follow 2 0.5 [3]"]);
  });

  it("reports the drawing load and the frames dropped while waiting", async () => {
    const stats: FollowRenderStats[] = [];
    const p = player();
    p.startFollowing((s) => stats.push(s));
    await p.load(first);
    p.followScene(1);
    // WebPuppet ができるまで待たせすぎて溢れたら古いものから捨て、続きは seq が飛んだものとして物理をリセットしてから適用する。
    for (let seq = 1; seq <= 601; seq++) p.pushFrame(followFrame(seq, 1, seq));
    const attached = p.attach(container);
    await settle();
    const fake = fakePuppet();
    pending[0]?.resolve(fake.puppet);
    await attached;
    step(100);
    const applied = followLog(fake.log, true);
    expect(applied.slice(0, 2)).toEqual(["reset_physics", "follow 2 0.5"]);
    expect(applied.slice(-2)).toEqual(["follow 601 0.5", "render"]);
    expect(applied).toHaveLength(602);
    expect(stats).toHaveLength(1);
    expect(stats[0]?.at).toBe(100);
    expect(stats[0]?.drawn).toBe(true);
    expect(stats[0]?.dropped).toBe(1);
    expect(stats[0]?.renderMs).toBeGreaterThanOrEqual(0);
  });

  it("draws only after a frame or a change of the look, and reports every animation frame (T0857)", async () => {
    const stats: FollowRenderStats[] = [];
    const { p, log } = await ready({ pose: [["HandsDown", "Wave"]] });
    p.startFollowing((s) => stats.push(s));
    await p.load(first);
    p.followScene(1);
    let now = 0;
    /** `requestAnimationFrame` を 1 回進め、描いたか (`render` を呼んだか) を返す。 */
    const drew = (): boolean => {
      const before = log.filter((l) => l === "render").length;
      step(now);
      now += 7;
      return log.filter((l) => l === "render").length > before;
    };
    // モデルを読んだ後の最初の回は描き、何も変わらなければ描かない。
    expect(drew()).toBe(true);
    expect(drew()).toBe(false);
    expect(drew()).toBe(false);
    // フレームを適用した (2 つ届いても 1 回だけ描く)。
    p.pushFrame(followFrame(1, 1, 1));
    p.pushFrame(followFrame(2, 1, 2));
    expect(drew()).toBe(true);
    expect(drew()).toBe(false);
    // 見せ方の変化。
    const changes: [string, () => void][] = [
      ["background", () => p.setBackground("#000000", true)],
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
      ["pose", () => p.showPose("Wave", { immediately: true })],
    ];
    for (const [what, change] of changes) {
      change();
      expect(drew(), what).toBe(true);
      expect(drew(), `${what} (again)`).toBe(false);
    }
    // 報告は描かなかった回も届き、描いた回だけ時間を持つ。
    expect(stats.length).toBe(Math.round(now / 7));
    expect(stats.filter((s) => s.drawn)).toHaveLength(2 + changes.length);
    expect(stats.every((s) => s.drawn || s.renderMs === 0)).toBe(true);
    expect(stats.map((s) => s.at)).toEqual(stats.map((_, i) => i * 7));
  });

  it("draws after the WebPuppet is recreated even without a new frame (T0857)", async () => {
    const p = player();
    p.startFollowing();
    const attached = p.attach(container);
    await settle();
    const firstFake = fakePuppet();
    pending[0]?.resolve(firstFake.puppet);
    await attached;
    await p.load(first);
    p.followScene(1);
    step(0);
    step(16);
    expect(firstFake.log.filter((l) => l === "render")).toHaveLength(1);
    const lost = pending[0]?.canvas;
    lost?.dispatchEvent(new Event("webglcontextlost", { cancelable: true }));
    lost?.dispatchEvent(new Event("webglcontextrestored"));
    await settle();
    const second = fakePuppet();
    pending[1]?.resolve(second.puppet);
    await settle();
    expect(second.loaded).toEqual([first.bytes]);
    step(32);
    step(48);
    expect(second.log.filter((l) => l === "render")).toHaveLength(1);
  });

  it("draws every animation frame when not following", async () => {
    const { log } = await ready();
    for (let i = 0; i < 4; i++) step(i * 16);
    expect(log.filter((l) => l === "render")).toHaveLength(4);
  });

  it("neither uses nor remembers the dress-up set and the lights", async () => {
    const hat = moduleFile("hat", "head", "path:/hat.ugoemod");
    const { p: viewer } = await ready();
    await viewer.load(first, [{ file: hat, attach: true }]);
    viewer.detach();
    const saved = localStorage.getItem("ugoe.viewer.dressUp");
    const lighting = localStorage.getItem("ugoe.viewer.lighting");

    const { p } = await ready({}, { readFile: async () => hat });
    p.startFollowing();
    p.setAutoRestore(true);
    await p.load(first, [{ file: moduleFile("scarf", "neck"), attach: true }]);
    expect(p.restoreOffer).toBeNull();
    expect(moduleRows(p)).toEqual([["scarf", true]]);
    p.updateLighting((s) => {
      s.ambient.intensity = 2;
    });
    expect(p.lighting.ambient.intensity).toBe(2);
    expect(p.lightingRemembered).toBe(false);
    expect(localStorage.getItem("ugoe.viewer.dressUp")).toBe(saved);
    expect(localStorage.getItem("ugoe.viewer.lighting")).toBe(lighting);
  });

  it("writes nothing to the storage, not even the remembered idle motion (T0930)", async () => {
    const storage = new MemoryStorage();
    const store = new PlayerStorage("ugoe.studio.player", storage as unknown as Storage);
    idleMemory(store).set(modelIdentity(first), { motion: "idle" });
    const writes: string[] = [];
    const setItem = storage.setItem.bind(storage);
    storage.setItem = (key, value) => {
      writes.push(key);
      setItem(key, value);
    };
    const { p } = await ready(
      { motions: [{ name: "idle", looping: true, parameters: [] }] },
      { storagePrefix: "ugoe.studio.player", storage: storage as unknown as Storage },
    );
    p.startFollowing();
    await p.load(first);
    p.followScene(1);
    p.pushFrame(followFrame(1, 1, 1));
    p.commitLighting();
    expect(writes).toEqual([]);
    expect(localStorage.length).toBe(0);
  });

  it("reports a missing follower API instead of applying frames", async () => {
    const { p, log } = await ready({ follower: false });
    p.startFollowing();
    await p.load(first);
    p.pushFrame(followFrame(1, 1, 1));
    p.followScene(1);
    expect(p.error).toBe(t("errors.noFollowerApi"));
    expect(followLog(log)).toEqual([]);
  });
});
