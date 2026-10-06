// @vitest-environment happy-dom
// プレビューのポーズの欄 (PreviewPanel.svelte) が、PreviewMode の表示する要素 (モーションによる切り替えに追従して
// 書き換わる。T0736) を出すこと (docs/testing.md#エディタの部品のテスト)。WebPuppet と PreviewMode は偽物。
import { flushSync } from "svelte";
import { describe, expect, it, vi } from "vitest";
import { LightingView } from "../panels/lighting/lightingView.svelte";
import {
  FakeController,
  FakeDocument,
  fakePuppet,
  render,
  settle,
  testModel,
} from "../viewport/tools/test/harness.svelte";
import PreviewPanel from "./PreviewPanel.svelte";
import { FakePreview } from "./test/fakePreview.svelte";

// 部品が呼ぶバックエンドはどれも答えない。
vi.mock("../ipc", () => ({
  ipc: new Proxy({}, { get: () => vi.fn(() => new Promise(() => {})) }),
}));

const entry = (part: string) => ({ part, links: [] as string[] });

/** 手を振る差分 (`HandsDown` / `Wave`) のポーズを持つモデル。 */
function poseModel(arms = false): string {
  const model = JSON.parse(testModel());
  const part = (id: string, name = id) => ({
    id,
    name,
    parent: null,
    keyforms: { bindings: [], keyforms: [{ opacity: 1 }] },
  });
  model.meta = { canvas: { width: 200, height: 100 } };
  const effect = { parameters: ["AngleX"] };
  model.effects = { eye_blink: effect, breath: effect, lip_sync: effect };
  model.parts = [part("HandsDown"), part("Wave"), part("ArmA", "腕 A"), part("ArmB", "腕 B")];
  model.pose = { fade_duration: 0.5, groups: [{ entries: [entry("HandsDown"), entry("Wave")] }] };
  if (arms) model.pose.groups.push({ entries: [entry("ArmA"), entry("ArmB")] });
  return JSON.stringify(model);
}

async function draw({ arms = false } = {}) {
  const doc = new FakeDocument();
  doc.model = { json: poseModel(arms), bundle: null };
  const controller = new FakeController();
  controller.puppet = fakePuppet();
  const preview = new FakePreview(controller.asController);
  const { target } = render(PreviewPanel, {
    preview: preview.asPreview,
    doc: doc.asDocument,
    viewport: controller.asController,
    lighting: new LightingView(),
  });
  await settle();
  return { preview, target };
}

/** ポーズのグループの欄 (@ugoe/player の PosePicker。要素が 2〜4 個なので SegmentedControl) の見出しと選んでいる要素。 */
const rows = (target: HTMLElement) =>
  [...target.querySelectorAll(".ugoe-pose-picker__group")].map((g) => [
    g.querySelector(".ugoe-segmented__label")?.textContent?.trim(),
    g.querySelector('[data-state="checked"]')?.textContent?.trim(),
  ]);
const shown = (target: HTMLElement) => rows(target).map(([, part]) => part);

describe("PreviewPanel pose choices", () => {
  it("follows the entries the preview shows", async () => {
    const { preview, target } = await draw();
    expect(shown(target)).toEqual(["HandsDown"]);

    // モーションが Wave に切り替えた (PreviewMode が表示中の要素に合わせる)。
    preview.poseShown = ["Wave"];
    flushSync();
    expect(shown(target)).toEqual(["Wave"]);

    // 着せ替えで付けたモジュールのグループも。
    preview.modulePoses = [
      { module: "Hat", group: { entries: [entry("Hat/On"), entry("Hat/Off")] } },
    ];
    flushSync();
    expect(shown(target)).toEqual(["Wave", "On"]);
    preview.modulePoseShown = ["Hat/Off"];
    flushSync();
    expect(shown(target)).toEqual(["Wave", "Off"]);
  });

  it("labels the groups and entries and shows the chosen entry", async () => {
    const { preview, target } = await draw({ arms: true });
    preview.modulePoses = [
      { module: "Hat", group: { entries: [entry("Hat/On"), entry("Hat/Off")] } },
    ];
    flushSync();
    // 本体の要素はパーツの名前 (と ID)、モジュールの要素は接頭辞を除いたもの。
    expect(rows(target)).toEqual([
      ["Group 1", "HandsDown"],
      ["Group 2", "腕 A (ArmA)"],
      ["Hat group 1", "On"],
    ]);
    const item = (text: string) => {
      const found = [...target.querySelectorAll<HTMLButtonElement>(".ugoe-segmented__item")].find(
        (b) => b.textContent?.trim() === text,
      );
      if (found === undefined) throw new Error(`no item ${text}`);
      return found;
    };
    item("腕 B (ArmB)").click();
    item("Off").click();
    flushSync();
    expect(preview.calls).toEqual(["showPose:ArmB", "showModulePose:Hat/Off"]);

    // ポーズの段を切ると選べない。
    preview.stages = { ...preview.stages, pose: false };
    flushSync();
    expect(item("Wave").disabled).toBe(true);
  });
});
