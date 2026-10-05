// @vitest-environment happy-dom
// 読めないモデル (`doc.parsedModel` が null) でも、主なパネルが例外を出さずに描けること
// (docs/testing.md#エディタの部品のテスト)。描いた後で読めなくなる (読み直しの JSON が壊れている) 場合も確かめる。
import { flushSync } from "svelte";
import { describe, expect, it, vi } from "vitest";
import type { EditorDocument } from "../document.svelte";
import { Selection } from "../selection.svelte";
import type { ViewportController } from "../viewport/controller.svelte";
import {
  FakeController,
  FakeDocument,
  render,
  settle,
  testModel,
} from "../viewport/tools/test/harness.svelte";
import GluesPanel from "./glues/GluesPanel.svelte";
import { GlueView } from "./glues/glueView.svelte";
import InspectorPanel from "./InspectorPanel.svelte";
import LightingPanel from "./lighting/LightingPanel.svelte";
import { LightingView } from "./lighting/lightingView.svelte";
import ParametersPanel from "./ParametersPanel.svelte";
import PhysicsPanel from "./physics/PhysicsPanel.svelte";
import { PhysicsView } from "./physics/physicsView.svelte";
import PosesPanel from "./poses/PosesPanel.svelte";
import SlotsPanel from "./slots/SlotsPanel.svelte";
import { SlotsView } from "./slots/slotsView.svelte";
import TreePanel from "./TreePanel.svelte";
import TimelinePanel from "./timeline/TimelinePanel.svelte";
import VerifyPanel from "./verify/VerifyPanel.svelte";
import { VerifyView } from "./verify/verifyView.svelte";

// パネルが呼ぶバックエンドはどれも答えない (読み込み中のまま。モデルが読めないことだけを確かめる)。
vi.mock("../ipc", () => ({
  ipc: new Proxy({}, { get: () => vi.fn(() => new Promise(() => {})) }),
}));

const UNREADABLE = "{ not json";

interface Env {
  doc: EditorDocument;
  selection: Selection;
  viewport: ViewportController;
}

/** パネルの名前と、偽の document・選択・ビューポートでそれを描く関数 (描いた先を返す)。 */
const PANELS: [string, (env: Env) => HTMLElement][] = [
  ["TreePanel", ({ doc, selection }) => render(TreePanel, { doc, selection }).target],
  [
    "InspectorPanel",
    ({ doc, selection, viewport }) => render(InspectorPanel, { doc, selection, viewport }).target,
  ],
  [
    "ParametersPanel",
    ({ doc, selection, viewport }) =>
      render(ParametersPanel, { doc, selection, viewport, parameters: [] }).target,
  ],
  [
    "PhysicsPanel",
    ({ doc, selection, viewport }) =>
      render(PhysicsPanel, { doc, selection, viewport, view: new PhysicsView() }).target,
  ],
  [
    "LightingPanel",
    ({ doc, selection }) =>
      render(LightingPanel, { doc, selection, view: new LightingView() }).target,
  ],
  [
    "GluesPanel",
    ({ doc, selection }) => render(GluesPanel, { doc, selection, view: new GlueView() }).target,
  ],
  [
    "SlotsPanel",
    ({ doc, selection }) => render(SlotsPanel, { doc, selection, view: new SlotsView() }).target,
  ],
  [
    "PosesPanel",
    ({ doc, selection, viewport }) => render(PosesPanel, { doc, selection, viewport }).target,
  ],
  [
    "VerifyPanel",
    ({ doc, selection, viewport }) =>
      render(VerifyPanel, {
        doc,
        selection,
        viewport,
        view: new VerifyView(),
        physics: new PhysicsView(),
      }).target,
  ],
  ["TimelinePanel", ({ doc, viewport }) => render(TimelinePanel, { doc, viewport }).target],
];

/** `json` を最初のモデルにし、メッシュ Quad を選んで `draw` で描く。 */
function setup(json: string, draw: (env: Env) => HTMLElement) {
  const doc = new FakeDocument();
  doc.model = { json, bundle: null };
  const selection = new Selection();
  selection.set([{ kind: "mesh", id: "Quad" }]);
  const controller = new FakeController();
  const target = draw({ doc: doc.asDocument, selection, viewport: controller.asController });
  return { doc, target };
}

describe("panels with an unreadable model", () => {
  it.each(PANELS)("draws %s", async (_name, draw) => {
    const { doc, target } = setup(UNREADABLE, draw);
    await settle();
    expect(doc.parsedModel).toBeNull();
    expect(target.childElementCount).toBeGreaterThan(0);
    expect(doc.apply).not.toHaveBeenCalled();
  });

  it.each(PANELS)("keeps %s alive when the model becomes unreadable", async (_name, draw) => {
    const { doc, target } = setup(testModel(), draw);
    await settle();
    doc.model = { json: UNREADABLE, bundle: null };
    flushSync();
    await settle();
    expect(doc.parsedModel).toBeNull();
    expect(target.childElementCount).toBeGreaterThan(0);
    // 読める JSON に戻れば、また描ける
    doc.model = { json: testModel(), bundle: null };
    flushSync();
    await settle();
    expect(doc.apply).not.toHaveBeenCalled();
  });
});
