// @vitest-environment happy-dom
// メッシュ編集ツール: ポインタの操作から送るコマンド (docs/testing.md#エディタの部品のテスト)。
import { describe, expect, it } from "vitest";
import { Selection } from "../../selection.svelte";
import type { Point } from "./geometry";
import MeshEditTool from "./MeshEditTool.svelte";
import { ViewportScene } from "./scene.svelte";
import {
  all,
  deferred,
  drag,
  FakeController,
  FakeDocument,
  get,
  pointer,
  QUAD,
  render,
  screenOf,
  settle,
} from "./test/harness.svelte";

function setup() {
  const controller = new FakeController();
  const doc = new FakeDocument();
  const selection = new Selection();
  selection.set([{ kind: "mesh", id: "Quad" }]);
  const scene = new ViewportScene(controller.asController, doc.asDocument);
  const { target } = render(MeshEditTool, {
    scene,
    doc: doc.asDocument,
    selection,
    meshId: "Quad",
  });
  const svg = get(target, "svg.tool");
  /** 頂点の四角の中心 (CSS ピクセル)。 */
  const vertices = (): Point[] =>
    all(svg, ".vertices rect").map((r) => [
      Number(r.getAttribute("x")) + 3.5,
      Number(r.getAttribute("y")) + 3.5,
    ]);
  return { controller, doc, selection, svg, vertices };
}

const screen = (i: number) => screenOf(QUAD[i] ?? [0, 0]);
const shift = (p: Point, dx: number, dy: number): Point => [p[0] + dx, p[1] + dy];

describe("MeshEditTool", () => {
  it("moves the dragged vertex with one move_mesh_vertices on release", async () => {
    const { doc, selection, svg, vertices } = setup();
    const start = screen(1);
    drag(svg, [start, shift(start, 10, 6), shift(start, 20, 10)]);
    await settle();
    expect(selection.vertices.Quad).toEqual([1]);
    // CSS ピクセルの移動量をビューの拡大率 (2) で割ってキャンバス座標にする
    expect(doc.commands).toEqual([
      { op: "move_mesh_vertices", id: "Quad", moves: [{ index: 1, delta: [10, 5] }] },
    ]);
    expect(vertices()).toEqual(QUAD.map(screenOf));
  });

  it("previews the move while dragging without sending anything", () => {
    const { doc, svg, vertices } = setup();
    const start = screen(2);
    pointer(svg, "pointerdown", start);
    pointer(svg, "pointermove", shift(start, 8, -4));
    expect(vertices()[2]).toEqual(shift(start, 8, -4));
    expect(doc.apply).not.toHaveBeenCalled();
  });

  it("sends nothing and restores the mesh when the drag is cancelled", () => {
    const { doc, svg, vertices } = setup();
    const start = screen(1);
    drag(svg, [start, shift(start, 30, 30)], {}, "pointercancel");
    expect(doc.apply).not.toHaveBeenCalled();
    expect(vertices()).toEqual(QUAD.map(screenOf));
  });

  it("treats a tiny drag as a click", () => {
    const { doc, selection, svg } = setup();
    const start = screen(3);
    drag(svg, [start, shift(start, 1, 1)]);
    expect(selection.vertices.Quad).toEqual([3]);
    expect(doc.apply).not.toHaveBeenCalled();
  });

  it("moves every selected vertex together", async () => {
    const { doc, selection, svg } = setup();
    selection.vertices = { Quad: [0, 2] };
    const start = screen(2);
    drag(svg, [start, shift(start, -4, 8)]);
    await settle();
    expect(doc.commands).toEqual([
      {
        op: "move_mesh_vertices",
        id: "Quad",
        moves: [
          { index: 0, delta: [-2, 4] },
          { index: 2, delta: [-2, 4] },
        ],
      },
    ]);
  });

  it("selects with a box drag and clears with a click on nothing", () => {
    const { doc, selection, svg } = setup();
    // 頂点 0 と 1 を囲む (上の辺)
    drag(svg, [shift(screen(0), -10, -10), shift(screen(1), 10, 10)]);
    expect(selection.vertices.Quad).toEqual([0, 1]);
    drag(svg, [[400, 400]]);
    expect(selection.vertices.Quad).toEqual([]);
    expect(doc.apply).not.toHaveBeenCalled();
  });

  it("ignores presses while the previous edit is being sent", async () => {
    const { doc, selection, svg, vertices } = setup();
    const gate = deferred();
    doc.gate = gate.promise;
    const start = screen(1);
    drag(svg, [start, shift(start, 20, 0)]);
    expect(doc.apply).toHaveBeenCalledTimes(1);
    // 送り終わるまで、送った移動のプレビューを残す (元の位置に一瞬戻らない)。離した後のポインタの移動では動かさない
    expect(vertices()[1]).toEqual(shift(start, 20, 0));
    pointer(svg, "pointermove", shift(start, 40, 0));
    expect(vertices()[1]).toEqual(shift(start, 20, 0));
    // 送っている間に別の頂点を押しても受けない (離したときの編集は捨てられるので、ずらした表示も出さない)
    const other = screen(3);
    pointer(svg, "pointerdown", other, { pointerId: 2 });
    pointer(svg, "pointermove", shift(other, 0, 20), { pointerId: 2 });
    expect(selection.vertices.Quad).toEqual([1]);
    expect(vertices()[3]).toEqual(other);
    pointer(svg, "pointerup", shift(other, 0, 20), { pointerId: 2 });
    gate.resolve();
    await settle();
    expect(doc.apply).toHaveBeenCalledTimes(1);
    expect(vertices()).toEqual(QUAD.map(screenOf));
  });

  it("adds a vertex with Ctrl + click at the canvas position", async () => {
    const { doc, svg } = setup();
    pointer(svg, "pointerdown", screenOf([30, 40]), { ctrlKey: true });
    await settle();
    expect(doc.commands).toEqual([{ op: "add_mesh_vertex", id: "Quad", position: [30, 40] }]);
  });

  it("leaves the middle button and Alt + drag to the viewport", () => {
    const { doc, selection, svg } = setup();
    const start = screen(1);
    drag(svg, [start, shift(start, 20, 0)], { button: 1 });
    drag(svg, [start, shift(start, 20, 0)], { altKey: true });
    expect(selection.vertices.Quad).toBeUndefined();
    expect(doc.apply).not.toHaveBeenCalled();
  });
});
