// プレビューモードのポーズの欄が、モーションによる切り替えに追従すること (PreviewMode、T0736)。WebPuppet とビューポートは偽物。
import { beforeAll, describe, expect, it, vi } from "vitest";
import type { EditorDocument } from "../document.svelte";
import type { FrameDriver, ViewportController } from "../viewport/controller.svelte";
import type { WebPuppet } from "../viewport/wasm";
import type { PreviewMode as PreviewModeType } from "./preview.svelte";

let PreviewMode: typeof PreviewModeType;

// @ugoe/face-tracking の部品が読む部品の集まり。ここでは描かないので空にする (tracking.test.ts と同じ)。
vi.mock("@ugoe/ui", () => ({}));

beforeAll(async () => {
  (globalThis as { window?: unknown }).window ??= {};
  globalThis.cancelAnimationFrame ??= () => {};
  ({ PreviewMode } = await import("./preview.svelte"));
});

const entry = (part: string) => ({ part, links: [] as string[] });
/** 編集中のモデルのグループ。 */
const OWN = [{ entries: [entry("HandsDown"), entry("Wave")] }];
/** 着せ替えで付けたモジュールのグループ (WebPuppet の ID)。 */
const HAT = [entry("Hat/On"), entry("Hat/Off")];

/**
 * ポーズを持つ WebPuppet の偽物。`visible` (グループの順の表示中の要素) をテストが書き換えると、モーションが
 * 切り替えたことになる。`reads` は `pose_visible_json` を読んだ回数。
 */
function fakePuppet() {
  const groups = [OWN[0]?.entries ?? [], HAT];
  const visible = groups.map((g) => g[0]?.part ?? null);
  const counts = { reads: 0 };
  const puppet = {
    has_model: () => true,
    has_physics: () => true,
    set_parameter: () => true,
    parameter: () => undefined,
    set_expression_json() {},
    stop_expressions() {},
    set_lip_sync_level() {},
    set_enabled() {},
    tracking_parameter_ids: () => [],
    modules_json: () => "[]",
    detached_modules_json: () => "[]",
    load_warnings_json: () => "[]",
    attached_modules: () => [],
    is_motion_playing: () => false,
    motion_events: () => [],
    pose_json: () =>
      JSON.stringify({ fade_duration: 0.5, groups: groups.map((entries) => ({ entries })) }),
    show_pose(part: string) {
      const i = groups.findIndex((g) => g.some((e) => e.part === part));
      if (i < 0) return false;
      visible[i] = part;
      return true;
    },
    pose_visible_json() {
      counts.reads++;
      return JSON.stringify(visible);
    },
  };
  return { puppet: puppet as unknown as WebPuppet, visible, counts };
}

function setup() {
  const { puppet, visible, counts } = fakePuppet();
  const viewport = {
    puppet,
    parameterValues: {},
    setDriver(d: FrameDriver | null) {
      d?.loaded(puppet);
    },
    applyStages() {},
  } as unknown as ViewportController;
  const doc = {
    parameters: [],
    expressions: [],
    physics: null,
    parsedModel: { pose: { fade_duration: 0.5, groups: OWN } },
  } as unknown as EditorDocument;
  const preview = new PreviewMode(doc, viewport);
  preview.enter();
  /** 1 フレーム進めた後の FrameDriver の呼び出し。 */
  const frame = () => preview.afterUpdate(puppet);
  return { preview, visible, counts, frame };
}

describe("PreviewMode pose choices", () => {
  it("follows the entries a motion switches to", () => {
    const { preview, visible, frame } = setup();
    frame();
    expect(preview.modulePoses.map((m) => m.module)).toEqual(["Hat"]);
    expect(preview.poseShown).toEqual([]);
    expect(preview.modulePoseShown).toEqual([]);

    // モーションがパーツの不透明度のカーブで要素を要求した。
    visible[0] = "Wave";
    visible[1] = "Hat/Off";
    frame();
    expect(preview.poseShown).toEqual(["Wave"]);
    expect(preview.modulePoseShown).toEqual(["Hat/Off"]);

    // 変わらないフレームでは欄を作り直さない。
    const shown = preview.poseShown;
    frame();
    expect(preview.poseShown).toBe(shown);
  });

  it("shows the latest of the user's choice and the motion's request", () => {
    const { preview, visible, frame } = setup();
    preview.showPose(OWN[0] ?? { entries: [] }, "Wave");
    frame();
    expect(preview.poseShown).toEqual(["Wave"]);

    // 選んだ後でモーションが同じグループの要素を要求すると、モーションが勝つ。
    visible[0] = "HandsDown";
    frame();
    expect(preview.poseShown).toEqual(["HandsDown"]);
  });
});
