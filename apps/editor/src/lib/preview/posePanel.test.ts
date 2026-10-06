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
function poseModel(): string {
  const model = JSON.parse(testModel());
  const part = (id: string) => ({
    id,
    name: id,
    parent: null,
    keyforms: { bindings: [], keyforms: [{ opacity: 1 }] },
  });
  model.meta = { canvas: { width: 200, height: 100 } };
  const effect = { parameters: ["AngleX"] };
  model.effects = { eye_blink: effect, breath: effect, lip_sync: effect };
  model.parts = [part("HandsDown"), part("Wave")];
  model.pose = { fade_duration: 0.5, groups: [{ entries: [entry("HandsDown"), entry("Wave")] }] };
  return JSON.stringify(model);
}

async function draw() {
  const doc = new FakeDocument();
  doc.model = { json: poseModel(), bundle: null };
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

/** ポーズのグループの欄の選んでいる要素。 */
const shown = (target: HTMLElement) =>
  [...target.querySelectorAll(".pose .ugoe-select__value")].map((v) => v.textContent?.trim());

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
});
