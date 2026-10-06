// Player のポーズの要素の選択 (T0735) と、モーションによる切り替えへの追従 (T0736)。wasm と DOM は偽物 (src/testing/)。
import { describe, expect, it, vi } from "vitest";
import { bundle, moduleFile } from "./testing/fakePuppet";
import { ready, setupPlayer, step } from "./testing/player";

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

  it("shows an entry without a fade when asked", async () => {
    const { p, state } = await ready(options);
    await p.load(model);
    expect(p.showPose("no_earwear")).toBe(true);
    expect(state.immediate).toEqual([]);

    expect(p.showPose("earwear", { immediately: true })).toBe(true);
    expect(state.immediate).toEqual(["earwear"]);
    expect(p.poseShown).toEqual(["earwear"]);
    expect(p.showPose("Face", { immediately: true })).toBe(false);
    expect(state.immediate).toEqual(["earwear"]);
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

  it("follows the entries a motion switches to", async () => {
    const { p, state } = await ready(options);
    await p.load(model, [{ file: hat, attach: true }]);
    step(0);
    expect(p.poseShown).toEqual(["earwear", "Hat/On"]);

    // モーションがパーツの不透明度のカーブで要素を要求した (偽物では表示中の要素を書き換える)。
    state.visible.set("earwear,no_earwear", "no_earwear");
    state.visible.set("Hat/On,Hat/Off", "Hat/Off");
    step(16);
    expect(p.poseShown).toEqual(["no_earwear", "Hat/Off"]);

    // 変わらないフレームでは欄を作り直さない。
    const shown = p.poseShown;
    step(32);
    expect(p.poseShown).toBe(shown);
  });

  it("shows the latest of the user's choice and the motion's request", async () => {
    const { p, state } = await ready(options);
    await p.load(model);
    step(0);

    // 利用者が選ぶと、表示中の要素もすぐにそれになるので、次のフレームでも欄はそのまま。
    p.showPose("no_earwear");
    step(16);
    expect(p.poseShown).toEqual(["no_earwear"]);

    // その後でモーションが同じグループの要素を要求すると、モーションが勝つ。
    state.visible.set("earwear,no_earwear", "earwear");
    step(32);
    expect(p.poseShown).toEqual(["earwear"]);
  });

  it("follows the visible entries after the groups are read again", async () => {
    const { p, state } = await ready(options);
    await p.load(model);
    step(0);
    state.visible.set("earwear,no_earwear", "no_earwear");
    step(16);
    expect(p.poseShown).toEqual(["no_earwear"]);

    // 付けると読み直す。モジュールのグループの要素もモーションに追従する。
    p.loadModule(hat);
    step(32);
    expect(p.poseShown).toEqual(["no_earwear", "Hat/On"]);
    state.visible.set("Hat/On,Hat/Off", "Hat/Off");
    step(48);
    expect(p.poseShown).toEqual(["no_earwear", "Hat/Off"]);
  });
});
