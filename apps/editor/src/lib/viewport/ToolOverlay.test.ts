// @vitest-environment happy-dom
// ビューポートのツールの切り替え: メッシュ編集モードのキー操作 (docs/testing.md#エディタの部品のテスト)。
import { flushSync } from "svelte";
import { describe, expect, it, vi } from "vitest";
import { t } from "../i18n";
import { GlueView } from "../panels/glues/glueView.svelte";
import { PhysicsView } from "../panels/physics/physicsView.svelte";
import { VerifyView } from "../panels/verify/verifyView.svelte";
import { Selection } from "../selection.svelte";
import ToolOverlay from "./ToolOverlay.svelte";
import {
  dialogButton,
  FakeController,
  FakeDocument,
  fakePuppet,
  pointer,
  press,
  render,
  settle,
  testModel,
} from "./tools/test/harness.svelte";

// メッシュ編集モードでは IPC を呼ばない (フォーム編集・ギズモ・はみ出しの表示は出ない)。
vi.mock("../ipc", () => ({ ipc: {} }));

/** メッシュ Quad のメッシュ編集モードで、頂点 0 を選んだ状態。 */
function setup() {
  const controller = new FakeController();
  const doc = new FakeDocument();
  const selection = new Selection();
  selection.set([{ kind: "mesh", id: "Quad" }]);
  selection.meshEdit = "Quad";
  selection.vertices = { Quad: [0] };
  const { target } = render(ToolOverlay, {
    controller: controller.asController,
    doc: doc.asDocument,
    selection,
    physics: new PhysicsView(),
    verify: new VerifyView(),
    glue: new GlueView(),
  });
  return { controller, doc, selection, target };
}

describe("ToolOverlay mesh edit keys", () => {
  it("deletes the selected vertices with Delete", async () => {
    const { doc, selection } = setup();
    const event = press(document.body, "Delete");
    expect(event.defaultPrevented).toBe(true);
    await settle();
    expect(doc.commands).toEqual([{ op: "remove_mesh_vertices", id: "Quad", indices: [0] }]);
    expect(selection.vertices.Quad).toEqual([]);
  });

  it("selects every vertex with Ctrl (Cmd) + A and leaves the mode with Esc", () => {
    const { selection } = setup();
    press(document.body, "a", { metaKey: true });
    expect(selection.vertices.Quad).toEqual([0, 1, 2, 3]);
    press(document.body, "Escape");
    expect(selection.meshEdit).toBeNull();
  });

  it("pans the view to a vertex picked with ] or [ outside the view, but not inside", () => {
    const { controller } = setup();
    // 頂点の画面の位置は (10, 20) (210, 20) (210, 220) (10, 220)。ビューは 400×300 で、端の余白は 48
    controller.size = [400, 300];
    press(document.body, "]");
    expect(controller.panBy.mock.calls).toEqual([[0, 28]]);
    press(document.body, "]");
    expect(controller.panBy).toHaveBeenCalledTimes(1);
    press(document.body, "]");
    expect(controller.panBy.mock.calls).toEqual([
      [0, 28],
      [38, 0],
    ]);
  });

  it("pans a vertex hidden under a tall strip into view below it (T1653)", () => {
    // 左上の並び (.strip) の高さを 200px にする (happy-dom は大きさを測らない)
    const height = vi
      .spyOn(HTMLElement.prototype, "offsetHeight", "get")
      .mockImplementation(function (this: HTMLElement) {
        return this.classList.contains("strip") ? 200 : 0;
      });
    try {
      const { controller, selection } = setup();
      expect(controller.covered.top).toBe(200);
      controller.size = [400, 300];
      selection.vertices = { Quad: [1] };
      // 頂点 2 の画面の位置は (210, 220)。ビュー全体の余白 (48) の内側だが、帯の下の端から 48 より上にある
      press(document.body, "]");
      expect(selection.vertices.Quad).toEqual([2]);
      expect(controller.panBy.mock.calls).toEqual([[0, 200 + 48 - 220]]);
    } finally {
      height.mockRestore();
    }
  });

  it("moves the selected vertices with the arrow keys", async () => {
    const { doc } = setup();
    const event = press(document.body, "ArrowLeft", { shiftKey: true });
    expect(event.defaultPrevented).toBe(true);
    await settle();
    // 画面の 10px は偽のビューの拡大率 2 でキャンバスの 5
    expect(doc.commands).toEqual([
      { op: "move_mesh_vertices", id: "Quad", moves: [{ index: 0, delta: [-5, 0] }] },
    ]);
  });

  it("picks the next and previous vertex with ] and [ and reads out which one", () => {
    const { doc, selection, target } = setup();
    const status = () => target.querySelector(".vp-band [role=status]")?.textContent?.trim();
    expect(press(document.body, "]").defaultPrevented).toBe(true);
    expect(selection.vertices.Quad).toEqual([1]);
    expect(status()).toBe(t("viewport.point.current", { index: 2, total: 4 }));
    expect(target.querySelector("svg.tool rect.ring")).not.toBeNull();
    press(document.body, "[");
    press(document.body, "[");
    expect(selection.vertices.Quad).toEqual([3]);
    // 矢印を使う部品 (ツリーなど) にフォーカスがあるときは奪わない
    const outside = document.createElement("button");
    document.body.append(outside);
    try {
      expect(press(outside, "]").defaultPrevented).toBe(false);
    } finally {
      outside.remove();
    }
    expect(selection.vertices.Quad).toEqual([3]);
    expect(doc.apply).not.toHaveBeenCalled();
  });

  it("picks a second vertex with Shift + ] and adds their midpoint with I (T1648)", async () => {
    const { doc, selection, target } = setup();
    const status = () => target.querySelector(".vp-band [role=status]")?.textContent?.trim();
    const button = () =>
      [...target.querySelectorAll<HTMLButtonElement>(".toolbar button")].find(
        (b) => b.textContent?.trim() === t("viewport.meshEdit.addMidpoint.label"),
      );
    // 1 つだけ選んでいる間は足せない
    expect(button()?.disabled).toBe(true);
    expect(press(document.body, "i").defaultPrevented).toBe(false);
    expect(press(document.body, "}", { shiftKey: true }).defaultPrevented).toBe(true);
    expect(selection.vertices.Quad).toEqual([0, 1]);
    expect(status()).toBe(t("viewport.point.pair", { a: 1, b: 2, total: 4 }));
    expect(button()?.disabled).toBe(false);
    // 2 つ目だけを次へ (1 つ目は残す)
    press(document.body, "}", { shiftKey: true });
    expect(selection.vertices.Quad).toEqual([0, 2]);
    press(document.body, "{", { shiftKey: true });
    expect(selection.vertices.Quad).toEqual([0, 1]);
    expect(doc.apply).not.toHaveBeenCalled();
    // 頂点 0 (0, 0) と 1 (100, 0) の中点。足した頂点 (4 番) を選ぶ
    doc.onApply = () => {
      const model = JSON.parse(doc.model?.json ?? "{}");
      model.meshes[0].uvs.push([50, 0]);
      for (const k of model.meshes[0].keyforms.keyforms) k.positions.push([50, 0]);
      doc.model = { json: JSON.stringify(model), bundle: null };
      return true;
    };
    expect(press(document.body, "i").defaultPrevented).toBe(true);
    await settle();
    expect(doc.commands).toEqual([{ op: "add_mesh_vertex", id: "Quad", position: [50, 0] }]);
    expect(selection.vertices.Quad).toEqual([4]);
  });

  it("adds the midpoint from the toolbar button", async () => {
    const { doc, selection, target } = setup();
    selection.vertices = { Quad: [1, 2] };
    flushSync();
    const button = [...target.querySelectorAll<HTMLButtonElement>(".toolbar button")].find(
      (b) => b.textContent?.trim() === t("viewport.meshEdit.addMidpoint.label"),
    );
    button?.click();
    await settle();
    expect(doc.commands).toEqual([{ op: "add_mesh_vertex", id: "Quad", position: [100, 50] }]);
  });

  it("does not take Esc, Delete or Ctrl + A pressed inside a dialog", async () => {
    const { doc, selection } = setup();
    const inDialog = dialogButton();
    expect(press(inDialog, "Delete").defaultPrevented).toBe(false);
    expect(press(inDialog, "Backspace").defaultPrevented).toBe(false);
    expect(press(inDialog, "a", { ctrlKey: true }).defaultPrevented).toBe(false);
    press(inDialog, "Escape");
    await settle();
    expect(doc.apply).not.toHaveBeenCalled();
    expect(selection.vertices.Quad).toEqual([0]);
    expect(selection.meshEdit).toBe("Quad");
  });

  it("leaves Backspace to a text field", async () => {
    const { doc } = setup();
    const input = document.createElement("input");
    document.body.append(input);
    try {
      expect(press(input, "Backspace").defaultPrevented).toBe(false);
      await settle();
      expect(doc.apply).not.toHaveBeenCalled();
    } finally {
      input.remove();
    }
  });
});

describe("ToolOverlay hover", () => {
  function setupHover(selected: boolean, controller = new FakeController()) {
    const selection = new Selection();
    if (selected) selection.set([{ kind: "mesh", id: "Quad" }]);
    const doc = new FakeDocument();
    const { target } = render(ToolOverlay, {
      controller: controller.asController,
      doc: doc.asDocument,
      selection,
      physics: new PhysicsView(),
      verify: new VerifyView(),
      glue: new GlueView(),
    });
    return { selection, target, doc };
  }

  it("outlines the mesh under the pointer and shares it with the tree", async () => {
    const { selection, target } = setupHover(false);
    await settle();
    const svg = target.querySelector("svg.layer") as SVGSVGElement;
    pointer(svg, "pointermove", [60, 60], { buttons: 0 });
    expect(selection.hover).toEqual({ kind: "mesh", id: "Quad" });
    await settle();
    expect(target.querySelectorAll("path.outline.hover")).toHaveLength(1);
    svg.dispatchEvent(new PointerEvent("pointerleave"));
    flushSync();
    expect(selection.hover).toBeNull();
  });

  it("draws a hover outline for a hovered tree row but not over a selected mesh", async () => {
    const { selection, target } = setupHover(true);
    selection.hover = { kind: "mesh", id: "Quad" };
    await settle();
    expect(target.querySelectorAll("path.outline.hover")).toHaveLength(0);
    selection.clear();
    await settle();
    expect(target.querySelectorAll("path.outline.hover")).toHaveLength(1);
  });

  it("outlines every mesh inside a hovered part or deformer row in one path", async () => {
    // 偽の評価結果に 2 つ目のメッシュ (Quad と同じ形) を足す
    const controller = new FakeController();
    const puppet = fakePuppet();
    controller.puppet = Object.assign(puppet, {
      drawable_ids: () => ["Quad", "Quad2"],
      drawable_positions: (i: number) => (i < 2 ? fakePuppet().drawable_positions(0) : undefined),
    });
    const { selection, target, doc } = setupHover(false, controller);
    // Body (パーツ) に Quad と Quad2、Quad2 は Warp (デフォーマ) の下
    const model = JSON.parse(testModel());
    const quad = model.meshes[0];
    model.parts = [
      {
        id: "Body",
        name: "Body",
        parent: null,
        keyforms: { bindings: [], keyforms: [{ opacity: 1 }] },
      },
    ];
    model.meshes = [
      { ...quad, part: "Body" },
      { ...quad, id: "Quad2", name: "Quad2", part: "Body", parent: "Warp" },
    ];
    doc.model = { json: JSON.stringify(model), bundle: null };
    const paths = () => [...target.querySelectorAll("path.outline.hover")];
    /** パスの中の線分の数 (四角形 1 つで 4 本)。 */
    const segments = () => paths().map((p) => (p.getAttribute("d") ?? "").split("M").length - 1);
    selection.hover = { kind: "part", id: "Body" };
    await settle();
    expect(segments()).toEqual([8]);
    selection.hover = { kind: "deformer", id: "Warp" };
    await settle();
    expect(segments()).toEqual([4]);
    // 選んでいるメッシュは選択の輪郭だけ
    selection.hover = { kind: "part", id: "Body" };
    selection.set([{ kind: "mesh", id: "Quad" }]);
    await settle();
    expect(segments()).toEqual([4]);
    // 中にメッシュの無いデフォーマ
    selection.hover = { kind: "deformer", id: "Rot" };
    await settle();
    expect(paths()).toHaveLength(0);
  });

  it("does not change the hover while a button is held", async () => {
    const { selection, target } = setupHover(false);
    await settle();
    const svg = target.querySelector("svg.layer") as SVGSVGElement;
    pointer(svg, "pointermove", [60, 60], { buttons: 1 });
    expect(selection.hover).toBeNull();
  });
});
