// Player のポーズの要素の選択 (T0735)。wasm と DOM は偽物 (src/testing/)。
import { describe, expect, it, vi } from "vitest";
import { bundle, moduleFile } from "./testing/fakePuppet";
import { ready, setupPlayer } from "./testing/player";

// 画面の部品と MediaPipe まで読み込むと遅いので、Player が使う関数だけにする。
vi.mock("@ugoe/face-tracking", () => import("./testing/faceTracking"));

setupPlayer();

describe("Player poses", () => {
  const model = bundle("avatar.ugoe", 1);
  const hat = moduleFile("Hat", "head");
  const options = {
    pose: [["earwear", "no_earwear"]],
    modulePoses: { Hat: [["On", "Off"]] },
  };

  /** 偽物が表示している要素 (グループの順)。 */
  function visible(state: { visible: Map<string, string> }, groups: string[][]): string[] {
    return groups.map((g) => state.visible.get(g.join(",")) ?? g[0] ?? "");
  }

  it("has no groups for a model without a pose", async () => {
    const { p } = await ready();
    await p.load(model);
    expect(p.poseGroups).toEqual([]);
    expect(p.poseShown).toEqual([]);
    expect(p.showPose("earwear")).toBe(false);
  });

  it("shows the chosen entry of each group", async () => {
    const { p, state } = await ready(options);
    await p.load(model);
    expect(p.poseGroups).toEqual([{ parts: ["earwear", "no_earwear"], module: null }]);
    expect(p.poseShown).toEqual(["earwear"]);

    expect(p.showPose("no_earwear")).toBe(true);
    expect(p.poseShown).toEqual(["no_earwear"]);
    expect(state.visible.get("earwear,no_earwear")).toBe("no_earwear");

    // 要素でないパーツは何もしない。
    expect(p.showPose("Face")).toBe(false);
    expect(p.poseShown).toEqual(["no_earwear"]);
  });

  it("adds and removes module groups and keeps the choices of the other groups", async () => {
    const { p, state } = await ready(options);
    await p.load(model);
    p.showPose("no_earwear");

    p.loadModule(hat);
    expect(p.poseGroups.map((g) => g.parts)).toEqual([
      ["earwear", "no_earwear"],
      ["Hat/On", "Hat/Off"],
    ]);
    expect(p.poseShown).toEqual(["no_earwear", "Hat/On"]);
    p.showPose("Hat/Off");
    expect(p.poseShown).toEqual(["no_earwear", "Hat/Off"]);

    p.detachModule("Hat");
    expect(p.poseShown).toEqual(["no_earwear"]);
    expect(visible(state, [["earwear", "no_earwear"]])).toEqual(["no_earwear"]);

    // 付け直すと、モジュールのグループは先頭の要素から (Puppet と同じ)。
    p.attachModule("Hat");
    expect(p.poseShown).toEqual(["no_earwear", "Hat/On"]);
    expect(
      visible(state, [
        ["earwear", "no_earwear"],
        ["Hat/On", "Hat/Off"],
      ]),
    ).toEqual(p.poseShown);
  });

  it("starts from the first entries when the model is loaded again", async () => {
    const { p, state } = await ready(options);
    await p.load(model, [{ file: hat, attach: true }]);
    p.showPose("no_earwear");
    p.showPose("Hat/Off");

    // 同じモデルを読み直す (付けていたモジュールは wasm が付け直す)。
    await p.load(model);
    expect(p.poseGroups.map((g) => g.parts)).toEqual([
      ["earwear", "no_earwear"],
      ["Hat/On", "Hat/Off"],
    ]);
    expect(p.poseShown).toEqual(["earwear", "Hat/On"]);
    expect(
      visible(state, [
        ["earwear", "no_earwear"],
        ["Hat/On", "Hat/Off"],
      ]),
    ).toEqual(p.poseShown);
  });
});
