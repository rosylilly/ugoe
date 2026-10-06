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
    expect(stats[0]?.dropped).toBe(1);
    expect(stats[0]?.renderMs).toBeGreaterThanOrEqual(0);
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
