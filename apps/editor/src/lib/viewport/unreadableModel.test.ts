// @vitest-environment happy-dom
// 読めないモデル (`doc.parsedModel` が null) でも、ビューポートに重ねる部品が例外を出さずに描けること
// (docs/testing.md#エディタの部品のテスト)。描いた後で読めなくなる (読み直しの JSON が壊れている) 場合も確かめる。
import { flushSync } from "svelte";
import { describe, expect, it, vi } from "vitest";
import type { PhysicsSettings } from "../ipc";
import { GlueView } from "../panels/glues/glueView.svelte";
import { LightingView } from "../panels/lighting/lightingView.svelte";
import { PhysicsView } from "../panels/physics/physicsView.svelte";
import { VerifyView } from "../panels/verify/verifyView.svelte";
import { type ElementRef, Selection } from "../selection.svelte";
import ToolOverlay from "./ToolOverlay.svelte";
import LightingOverlay from "./tools/lighting/LightingOverlay.svelte";
import {
  FakeController,
  FakeDocument,
  render,
  settle,
  testModel,
} from "./tools/test/harness.svelte";

// 部品が呼ぶバックエンドはどれも答えない (読めないモデルでは呼ばないはずだが、呼んでも止まらないように)。
vi.mock("../ipc", () => ({
  ipc: new Proxy({}, { get: () => vi.fn(() => new Promise(() => {})) }),
}));

const UNREADABLE = "{ not json";

/** 物理のパネルの衝突体 (モデルが読めなくても物理の設定は別に届く)。 */
const PHYSICS = {
  gravity: [0, 1],
  wind: [0, 0],
  fps: 60,
  rigs: [],
  colliders: [
    {
      id: "Head",
      name: "Head",
      attach: "Rot",
      shape: { type: "circle", center: [0, 0], radius: 5 },
    },
  ],
} as unknown as PhysicsSettings;

/**
 * ToolOverlay を、重ねる表示 (深度・はみ出し・物理・グルー) をすべて出す状態で描く。
 * `json` を最初のモデルにし、`primary` を選ぶ (`meshEdit` ならメッシュ編集モード)。
 */
function setup(json: string, primary: ElementRef | null, meshEdit = false) {
  const controller = new FakeController();
  controller.showDepth = true;
  const doc = new FakeDocument();
  doc.model = { json, bundle: null };
  doc.physics = PHYSICS;
  const selection = new Selection();
  if (primary) selection.set([primary]);
  if (meshEdit && primary) {
    selection.meshEdit = primary.id;
    selection.vertices = { [primary.id]: [0] };
  }
  const physics = new PhysicsView();
  physics.active = true;
  physics.collider = "Head";
  const verify = new VerifyView();
  verify.active = true;
  verify.showOverflows = true;
  const glue = new GlueView();
  glue.active = true;
  glue.glue = "Seam";
  glue.setPairMode("add");
  const { target } = render(ToolOverlay, {
    controller: controller.asController,
    doc: doc.asDocument,
    selection,
    physics,
    verify,
    glue,
  });
  return { doc, target };
}

/**
 * モデルから作る円 (頂点・取っ手・点) の数。衝突体は物理の設定 (`doc.physics`。モデルとは別に届く) から描くので数えない。
 */
function modelHandles(target: HTMLElement): number {
  return Array.from(target.querySelectorAll("circle")).filter((c) => !c.closest(".collider"))
    .length;
}

const CASES: [string, ElementRef | null, boolean][] = [
  ["nothing selected", null, false],
  ["a mesh (form edit)", { kind: "mesh", id: "Quad" }, false],
  ["a mesh in mesh edit mode", { kind: "mesh", id: "Quad" }, true],
  ["a warp deformer (gizmo)", { kind: "deformer", id: "Warp" }, false],
  ["a rotation deformer (gizmo)", { kind: "deformer", id: "Rot" }, false],
];

describe("viewport overlays with an unreadable model", () => {
  it.each(CASES)("draws the tool overlay with %s", async (_name, primary, meshEdit) => {
    const { doc, target } = setup(UNREADABLE, primary, meshEdit);
    await settle();
    expect(doc.parsedModel).toBeNull();
    // モデルの取っ手も頂点も出さない (操作の対象が無い)
    expect(modelHandles(target)).toBe(0);
    expect(doc.apply).not.toHaveBeenCalled();
  });

  it.each(CASES)(
    "keeps the tool overlay alive when the model becomes unreadable with %s",
    async (_name, primary, meshEdit) => {
      const { doc, target } = setup(testModel(), primary, meshEdit);
      await settle();
      // 読めるうちは描いている (グルーの組の頂点など)
      expect(modelHandles(target)).toBeGreaterThan(0);
      doc.model = { json: UNREADABLE, bundle: null };
      flushSync();
      await settle();
      expect(doc.parsedModel).toBeNull();
      expect(modelHandles(target)).toBe(0);
      expect(doc.apply).not.toHaveBeenCalled();
      // 読める JSON に戻れば、また描ける
      doc.model = { json: testModel(), bundle: null };
      flushSync();
      await settle();
      expect(doc.parsedModel).not.toBeNull();
      expect(modelHandles(target)).toBeGreaterThan(0);
    },
  );

  it("draws the light handles without the canvas size", async () => {
    const controller = new FakeController();
    const doc = new FakeDocument();
    doc.model = { json: UNREADABLE, bundle: null };
    const view = new LightingView();
    view.active = true;
    const { target } = render(LightingOverlay, {
      controller: controller.asController,
      doc: doc.asDocument,
      view,
    });
    await settle();
    // 平行光源の円の位置はキャンバスの大きさから決めるので、読めなければ取っ手を出さない
    expect(target.querySelectorAll("circle").length).toBe(0);
  });
});
