// @vitest-environment happy-dom
// デフォーマのギズモ: ポインタの操作から送るコマンド (docs/testing.md#エディタの部品のテスト)。
import { beforeEach, describe, expect, it, vi } from "vitest";
import { t } from "../../../i18n";
import { Selection } from "../../../selection.svelte";
import type { Point } from "../geometry";
import {
  deferred,
  dialogButton,
  drag,
  FakeController,
  FakeDocument,
  get,
  pointer,
  press,
  QUAD,
  render,
  screenOf,
  settle,
} from "../test/harness.svelte";
import DeformerGizmo from "./DeformerGizmo.svelte";

// 親の座標系への変換 (バックエンド) は恒等写像とする。
const ipc = vi.hoisted(() => ({
  canvasToLocal: vi.fn(async (_target: unknown, _values: unknown, targets: Point[]) => targets),
}));
vi.mock("../../../ipc", () => ({ ipc }));

beforeEach(() => ipc.canvasToLocal.mockClear());

function setup(deformerId: "Warp" | "Rot") {
  const controller = new FakeController();
  const doc = new FakeDocument();
  const selection = new Selection();
  selection.set([{ kind: "deformer", id: deformerId }]);
  const { target } = render(DeformerGizmo, {
    controller: controller.asController,
    doc: doc.asDocument,
    selection,
    deformerId,
  });
  const svg = get(target, "svg.gizmo");
  const at = (selector: string): Point => {
    const el = get(svg, selector);
    return el.tagName === "circle"
      ? [Number(el.getAttribute("cx")), Number(el.getAttribute("cy"))]
      : [Number(el.getAttribute("x")) + 4.5, Number(el.getAttribute("y")) + 4.5];
  };
  return { controller, doc, selection, svg, target, at };
}

const shift = (p: Point, dx: number, dy: number): Point => [p[0] + dx, p[1] + dy];
const ROT_KEYFORM = { origin: [50, 50], angle: 0, scale: 1, reflect_x: false, reflect_y: false };

describe("DeformerGizmo (warp)", () => {
  it("moves the dragged control point with one set_keyform_cell", async () => {
    const { doc, selection, svg } = setup("Warp");
    const start = screenOf(QUAD[1] ?? [0, 0]);
    drag(svg, [start, shift(start, 10, 4), shift(start, 20, 10)]);
    await settle();
    expect(selection.controlPoints.Warp).toEqual([1]);
    expect(ipc.canvasToLocal).toHaveBeenCalledWith(
      { kind: "deformer", id: "Warp" },
      {},
      [[110, 5]],
      [[100, 0]],
    );
    expect(doc.commands).toEqual([
      {
        op: "set_keyform_cell",
        target: { kind: "deformer", id: "Warp" },
        cell: {},
        keyform: {
          kind: "warp",
          points: [
            [0, 0],
            [110, 5],
            [100, 100],
            [0, 100],
          ],
        },
      },
    ]);
  });

  it("selects with a box drag inside the grid without sending anything", () => {
    const { doc, selection, svg } = setup("Warp");
    // 右の辺の 2 点 (1 と 2) を囲む
    drag(svg, [
      [190, 10],
      [220, 230],
    ]);
    expect(selection.controlPoints.Warp).toEqual([1, 2]);
    expect(doc.apply).not.toHaveBeenCalled();
  });

  it("sends nothing when the drag is cancelled", () => {
    const { doc, svg } = setup("Warp");
    const start = screenOf(QUAD[2] ?? [0, 0]);
    drag(svg, [start, shift(start, 30, 0)], {}, "pointercancel");
    expect(doc.apply).not.toHaveBeenCalled();
    expect(Number(get(svg, ".points rect:nth-child(3)").getAttribute("x"))).toBe(start[0] - 3.5);
  });
});

describe("DeformerGizmo (rotation)", () => {
  it("adds up the turns of the rotate handle beyond 180 degrees", async () => {
    const { doc, svg, at } = setup("Rot");
    const origin = at("circle.origin");
    const handle = at("circle.rotate");
    expect(handle).toEqual(shift(origin, 0, -60));
    // 時計回りに 3/4 周 (上 → 右 → 下 → 左)
    drag(svg, [handle, shift(origin, 60, 0), shift(origin, 0, 60), shift(origin, -60, 0)]);
    await settle();
    expect(doc.commands).toHaveLength(1);
    const command = doc.commands[0] as { keyform: { angle: number } };
    expect(command.keyform.angle).toBeCloseTo(270, 6);
  });

  it("snaps the rotation to 15 degrees with Shift", async () => {
    const { doc, svg, at } = setup("Rot");
    const origin = at("circle.origin");
    const r = (deg: number): Point => {
      const a = ((deg - 90) * Math.PI) / 180;
      return [origin[0] + 60 * Math.cos(a), origin[1] + 60 * Math.sin(a)];
    };
    drag(svg, [r(0), r(20), r(40)], { shiftKey: true });
    await settle();
    expect(doc.commands).toEqual([
      {
        op: "set_keyform_cell",
        target: { kind: "deformer", id: "Rot" },
        cell: {},
        keyform: { ...ROT_KEYFORM, angle: 45, kind: "rotation" },
      },
    ]);
  });

  it("scales with the scale handle", async () => {
    const { doc, svg, at } = setup("Rot");
    const origin = at("circle.origin");
    const handle = at("rect.scale");
    expect(handle).toEqual(shift(origin, 60, 0));
    drag(svg, [handle, shift(origin, 90, 0)]);
    await settle();
    const command = doc.commands[0] as { keyform: { scale: number } };
    expect(command.keyform.scale).toBeCloseTo(1.5, 6);
  });

  it("moves the origin of the current cell", async () => {
    const { doc, svg, at } = setup("Rot");
    const origin = at("circle.origin");
    drag(svg, [origin, shift(origin, 20, 6)]);
    await settle();
    expect(ipc.canvasToLocal).toHaveBeenCalledWith(
      { kind: "deformer", id: "Rot" },
      {},
      [[60, 53]],
      [[50, 50]],
    );
    expect(doc.commands).toEqual([
      {
        op: "set_keyform_cell",
        target: { kind: "deformer", id: "Rot" },
        cell: {},
        keyform: { ...ROT_KEYFORM, origin: [60, 53], kind: "rotation" },
      },
    ]);
  });

  it("moves the origin of every cell with Shift", async () => {
    const { doc, svg, at } = setup("Rot");
    const origin = at("circle.origin");
    drag(svg, [origin, shift(origin, 20, 6)], { shiftKey: true });
    await settle();
    expect(doc.commands).toEqual([{ op: "move_rotation_origin", id: "Rot", delta: [10, 3] }]);
  });

  it("sends nothing and puts the handles back when the drag is cancelled", () => {
    const { doc, svg, at } = setup("Rot");
    const origin = at("circle.origin");
    const handle = at("circle.rotate");
    pointer(svg, "pointerdown", handle);
    pointer(svg, "pointermove", shift(origin, 60, 0));
    expect(at("circle.rotate")).not.toEqual(handle);
    pointer(svg, "pointercancel", shift(origin, 60, 0));
    expect(doc.apply).not.toHaveBeenCalled();
    expect(at("circle.rotate")).toEqual(handle);
  });

  it("ignores presses while the previous edit is being sent", async () => {
    const { doc, svg, at } = setup("Rot");
    const gate = deferred();
    doc.pending = gate.promise;
    const origin = at("circle.origin");
    drag(svg, [at("rect.scale"), shift(origin, 90, 0)]);
    await settle();
    drag(svg, [origin, shift(origin, 20, 0)]);
    gate.resolve();
    await settle();
    expect(doc.commands.map((c) => c.op)).toEqual(["set_keyform_cell"]);
  });
});

describe("DeformerGizmo keys", () => {
  it("clears and selects all control points with Esc and Ctrl + A", () => {
    const { selection } = setup("Warp");
    const event = press(document.body, "a", { ctrlKey: true });
    expect(event.defaultPrevented).toBe(true);
    expect(selection.controlPoints.Warp).toEqual([0, 1, 2, 3]);
    press(document.body, "Escape");
    expect(selection.controlPoints.Warp).toEqual([]);
  });

  it("moves the picked control points with the arrow keys", async () => {
    const { doc, selection } = setup("Warp");
    selection.controlPoints = { Warp: [1] };
    await settle();
    expect(press(document.body, "ArrowDown").defaultPrevented).toBe(true);
    await settle();
    expect(doc.commands).toMatchObject([
      {
        op: "set_keyform_cell",
        keyform: {
          points: [
            [0, 0],
            [100, 0.5],
            [100, 100],
            [0, 100],
          ],
        },
      },
    ]);
  });

  it("picks the next and previous control point with ] and [ and reads out which one", () => {
    const { selection, target } = setup("Warp");
    const status = () => target.querySelector(".vp-band [role=status]")?.textContent?.trim();
    expect(status()).toBe("");
    expect(press(document.body, "]").defaultPrevented).toBe(true);
    expect(selection.controlPoints.Warp).toEqual([0]);
    expect(status()).toBe(t("viewport.point.current", { index: 1, total: 4 }));
    expect(target.querySelector("svg.gizmo rect.ring")).not.toBeNull();
    press(document.body, "[");
    expect(selection.controlPoints.Warp).toEqual([3]);
    const outside = document.createElement("button");
    document.body.append(outside);
    expect(press(outside, "]").defaultPrevented).toBe(false);
    outside.remove();
    expect(selection.controlPoints.Warp).toEqual([3]);
  });

  it("pans the view to a control point picked with ] or [ outside the view, but not inside", () => {
    const { controller } = setup("Warp");
    // 制御点の画面の位置は (10, 20) (210, 20) (210, 220) (10, 220)。ビューは 400×300 で、端の余白は 48
    controller.size = [400, 300];
    press(document.body, "[");
    expect(controller.panBy.mock.calls).toEqual([[38, 0]]);
    press(document.body, "[");
    expect(controller.panBy).toHaveBeenCalledTimes(1);
    // 2 → 3 → 0
    press(document.body, "]");
    press(document.body, "]");
    expect(controller.panBy.mock.calls).toEqual([
      [38, 0],
      [38, 0],
      [38, 28],
    ]);
  });

  it("leaves [ and ] alone for a rotation deformer", () => {
    const { selection } = setup("Rot");
    expect(press(document.body, "]").defaultPrevented).toBe(false);
    expect(selection.controlPoints.Rot).toBeUndefined();
  });

  it("moves the rotation origin with the arrow keys", async () => {
    const { doc } = setup("Rot");
    press(document.body, "ArrowRight", { shiftKey: true });
    await settle();
    expect(doc.commands).toEqual([
      {
        op: "set_keyform_cell",
        target: { kind: "deformer", id: "Rot" },
        cell: {},
        keyform: { ...ROT_KEYFORM, origin: [55, 50], kind: "rotation" },
      },
    ]);
  });

  it("does not take the keys pressed inside a dialog", () => {
    const { selection } = setup("Warp");
    const inDialog = dialogButton();
    const event = press(inDialog, "a", { metaKey: true });
    expect(event.defaultPrevented).toBe(false);
    expect(selection.controlPoints.Warp).toBeUndefined();
    selection.controlPoints = { Warp: [1] };
    press(inDialog, "Escape");
    expect(selection.controlPoints.Warp).toEqual([1]);
  });
});
