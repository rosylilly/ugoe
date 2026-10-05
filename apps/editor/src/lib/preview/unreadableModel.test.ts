// @vitest-environment happy-dom
// 読めないモデル (`doc.parsedModel` が null) でも、プレビューの部品 (PreviewPanel・DifferenceOverlay) が例外を出さずに
// 描けること (docs/testing.md#エディタの部品のテスト)。描いた後で読めなくなる (読み直しの JSON が壊れている) 場合も
// 確かめる。WebPuppet と PreviewMode は偽物。
import { flushSync } from "svelte";
import { describe, expect, it, vi } from "vitest";
import type { DifferenceItem } from "../ipc";
import { LightingView } from "../panels/lighting/lightingView.svelte";
import {
  FakeController,
  FakeDocument,
  fakePuppet,
  QUAD,
  render,
  settle,
  testModel,
} from "../viewport/tools/test/harness.svelte";
import type { WebPuppet } from "../viewport/wasm";
import DifferenceOverlay from "./DifferenceOverlay.svelte";
import PreviewPanel from "./PreviewPanel.svelte";
import { FakePreview } from "./test/fakePreview.svelte";

// 部品が呼ぶバックエンドはどれも答えない。呼ばれた名前を記録する (描くだけでは何も呼ばないはず)。
const ipcCalls = vi.hoisted((): string[] => []);
vi.mock("../ipc", () => ({
  ipc: new Proxy(
    {},
    {
      get: (_target, name) =>
        vi.fn(() => {
          ipcCalls.push(String(name));
          return new Promise(() => {});
        }),
    },
  ),
}));

const UNREADABLE = "{ not json";

/**
 * プレビューのテストのモデル。`testModel` に、プレビューのパネルが読む部分 (キャンバス・エフェクト・ポーズ・スロット・
 * アバタープロファイル) を足し、メッシュ Quad をモジュール Mod の要素 (`Mod/Quad`) にする。
 */
function previewModel(): string {
  const model = JSON.parse(testModel());
  const effect = { parameters: ["AngleX"] };
  const part = (id: string) => ({
    id,
    name: id,
    parent: null,
    keyforms: { bindings: [], keyforms: [{ opacity: 1 }] },
  });
  model.meta = { canvas: { width: 200, height: 100 } };
  model.effects = { eye_blink: effect, breath: effect, lip_sync: effect };
  model.parts = [part("ArmA"), part("ArmB")];
  model.pose = {
    fade_duration: 0.5,
    groups: [
      {
        entries: [
          { part: "ArmA", links: [] },
          { part: "ArmB", links: [] },
        ],
      },
    ],
  };
  model.slots = [{ id: "Hat", draw_order: { min: 0, max: 1000 } }];
  model.avatar = { slots: ["Hat"] };
  model.meshes[0].id = "Mod/Quad";
  return JSON.stringify(model);
}

/** 違いの行の要素 (モジュール Mod の中の ID)。 */
const item = (fields: Partial<DifferenceItem>): DifferenceItem => ({
  kind: "mesh",
  id: "Quad",
  text: "vertices 4 → 6",
  element: "Mod/Quad",
  meshes: ["Quad"],
  parts: [],
  ...fields,
});

/** モジュール Mod を付けた WebPuppet の偽物 (要素の ID に接頭辞 `Mod/` が付く)。 */
function modulePuppet(): WebPuppet {
  const flat = Float32Array.from(QUAD.flat());
  return {
    ...(fakePuppet() as unknown as Record<string, unknown>),
    drawable_ids: () => ["Mod/Quad"],
    drawable_positions: (i: number) => (i === 0 ? flat : undefined),
    warp_points: (id: string) => (id === "Mod/Warp" ? flat : undefined),
    rotation_frame: (id: string) =>
      id === "Mod/Rot" ? Float32Array.from([50, 50, 1, 0, 0, 1]) : undefined,
    physics_json: () =>
      JSON.stringify({
        gravity: [0, 1],
        wind: [0, 0],
        fps: 60,
        rigs: [],
        colliders: [
          {
            id: "Mod/Collar",
            name: "Collar",
            attach: null,
            shape: { type: "circle", center: [50, 50], radius: 10 },
          },
        ],
      }),
  } as unknown as WebPuppet;
}

function setup(json: string) {
  ipcCalls.length = 0;
  const doc = new FakeDocument();
  doc.model = { json, bundle: null };
  doc.parameters = [{ id: "AngleX", name: "Angle X", min: -30, max: 30, default: 0 }] as never;
  doc.motions = [{ name: "Idle", duration: 1, looping: true }] as never;
  doc.expressions = [{ name: "Smile" }] as never;
  const controller = new FakeController();
  controller.puppet = modulePuppet();
  // ライティングの節も出す (キャンバスの大きさをモデルから読む)
  controller.usesLighting = true;
  const preview = new FakePreview(controller.asController);
  return { doc, controller, preview };
}

/** モデルが読めなくなった後、また読めるようにする (読み直しの JSON が壊れた後で直る)。 */
async function breakAndRestore(doc: FakeDocument, json: string, check: () => void) {
  doc.model = { json: UNREADABLE, bundle: null };
  flushSync();
  await settle();
  expect(doc.parsedModel).toBeNull();
  check();
  doc.model = { json, bundle: null };
  flushSync();
  await settle();
}

/** 部品が何も送っていない (コマンド・プレビューの操作・バックエンド・ビューの移動)。 */
function expectNothingSent(doc: FakeDocument, preview: FakePreview, controller: FakeController) {
  expect(doc.apply).not.toHaveBeenCalled();
  expect(preview.calls).toEqual([]);
  expect(ipcCalls).toEqual([]);
  expect(controller.zoomAt).not.toHaveBeenCalled();
  expect(controller.panBy).not.toHaveBeenCalled();
}

describe("PreviewPanel with an unreadable model", () => {
  function draw(json: string) {
    const env = setup(json);
    const { target } = render(PreviewPanel, {
      preview: env.preview.asPreview,
      doc: env.doc.asDocument,
      viewport: env.controller.asController,
      lighting: new LightingView(),
    });
    return { ...env, target };
  }

  /** ポーズのグループの選択欄 (モデルのポーズから作る) の数。 */
  const poseSelects = (target: HTMLElement) =>
    target.querySelectorAll('section[aria-label="Pose"] .pose').length;

  it("draws the panel", async () => {
    const { doc, controller, preview, target } = draw(UNREADABLE);
    await settle();
    expect(doc.parsedModel).toBeNull();
    expect(target.querySelector(".preview-panel")).not.toBeNull();
    // モデルから作るもの (ポーズのグループ) は出さず、ドキュメントのもの (パラメータ) は出す
    expect(poseSelects(target)).toBe(0);
    expect(target.textContent).toContain("Angle X");
    expectNothingSent(doc, preview, controller);
  });

  it("keeps the panel alive when the model becomes unreadable", async () => {
    const { doc, controller, preview, target } = draw(previewModel());
    await settle();
    // 読めるうちはポーズのグループを出している
    expect(poseSelects(target)).toBe(1);
    await breakAndRestore(doc, previewModel(), () => {
      expect(target.querySelector(".preview-panel")).not.toBeNull();
      expect(poseSelects(target)).toBe(0);
    });
    expect(poseSelects(target)).toBe(1);
    expectNothingSent(doc, preview, controller);
  });
});

describe("DifferenceOverlay with an unreadable model", () => {
  function draw(json: string, highlighted: DifferenceItem) {
    const env = setup(json);
    env.preview.dressUp.highlight = { module: "Mod", item: highlighted };
    const { target } = render(DifferenceOverlay, {
      controller: env.controller.asController,
      doc: env.doc.asDocument,
      preview: env.preview.asPreview,
    });
    return { ...env, target };
  }

  const outline = (target: HTMLElement) =>
    target.querySelector<SVGPathElement>("path.outline")?.getAttribute("d") ?? null;

  const CASES: [string, DifferenceItem][] = [
    ["a mesh", item({})],
    ["a warp deformer", item({ kind: "deformer", id: "Warp", element: "Mod/Warp" })],
    ["a rotation deformer", item({ kind: "deformer", id: "Rot", element: "Mod/Rot" })],
    ["a collider", item({ kind: "collider", id: "Collar", element: "Mod/Collar", meshes: [] })],
  ];

  it.each(CASES)("highlights %s", async (_name, highlighted) => {
    const { doc, controller, preview, target } = draw(UNREADABLE, highlighted);
    await settle();
    expect(doc.parsedModel).toBeNull();
    // WebPuppet から描けるものは描き、知らせを出す
    expect(target.querySelector("svg.layer")).not.toBeNull();
    expect(target.querySelector('[role="status"]')?.textContent).toContain(highlighted.id);
    expectNothingSent(doc, preview, controller);
  });

  it.each(CASES)(
    "keeps the highlight of %s alive when the model becomes unreadable",
    async (_name, highlighted) => {
      const { doc, controller, preview, target } = draw(previewModel(), highlighted);
      await settle();
      expect(target.querySelector("svg.layer")).not.toBeNull();
      await breakAndRestore(doc, previewModel(), () => {
        expect(target.querySelector("svg.layer")).not.toBeNull();
        expect(target.querySelector('[role="status"]')).not.toBeNull();
      });
      expect(target.querySelector("svg.layer")).not.toBeNull();
      expectNothingSent(doc, preview, controller);
    },
  );

  it("outlines a mesh by its convex hull while the model is unreadable", async () => {
    const { doc, target } = draw(previewModel(), item({}));
    await settle();
    // 読めるうちは作業用のモデルの三角形の外周 (辺ごとの線。閉じない)
    const edges = outline(target);
    expect(edges).not.toBeNull();
    expect(edges).not.toContain("Z");
    await breakAndRestore(doc, previewModel(), () => {
      // 読めないと三角形が分からないので凸包 (閉じた折れ線)
      expect(outline(target)).toMatch(/Z$/);
    });
    expect(outline(target)).toBe(edges);
  });
});
