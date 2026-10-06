// @vitest-environment happy-dom
// フォーム編集ツール: ポインタの操作から送るコマンド (docs/testing.md#エディタの部品のテスト)。
import { beforeEach, describe, expect, it, vi } from "vitest";
import { t } from "../../../i18n";
import { Selection } from "../../../selection.svelte";
import type { Point } from "../geometry";
import {
  all,
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
import FormTool from "./FormTool.svelte";

// 親の座標系との変換 (バックエンド) は恒等写像とする。
const ipc = vi.hoisted(() => ({
  localToCanvas: vi.fn(async (_target: unknown, _values: unknown, points: Point[]) => points),
  canvasToLocal: vi.fn(async (_target: unknown, _values: unknown, targets: Point[]) => targets),
}));
vi.mock("../../../ipc", () => ({ ipc }));

beforeEach(() => {
  ipc.localToCanvas.mockClear();
  ipc.canvasToLocal.mockClear();
});

/** メッシュ Quad を選び、フォーム編集を有効にして (ToolOverlay の「Form edit」) 点が出るまで待つ。 */
async function setup() {
  const controller = new FakeController();
  const doc = new FakeDocument();
  const selection = new Selection();
  selection.set([{ kind: "mesh", id: "Quad" }]);
  const { target } = render(FormTool, {
    controller: controller.asController,
    doc: doc.asDocument,
    selection,
    enabled: true,
  });
  await settle();
  const svg = get(target, "svg.form");
  const handles = () => all(svg, "circle.handle");
  const handle = (i: number) => {
    const el = handles()[i];
    if (!el) throw new Error(`no handle ${i}`);
    return el;
  };
  const centers = (): Point[] =>
    handles().map((c) => [Number(c.getAttribute("cx")), Number(c.getAttribute("cy"))]);
  return { controller, doc, selection, target, svg, handle, centers };
}

const shift = (p: Point, dx: number, dy: number): Point => [p[0] + dx, p[1] + dy];
const screen = (i: number) => screenOf(QUAD[i] ?? [0, 0]);

describe("FormTool", () => {
  it("shows the points of the current cell", async () => {
    const { centers } = await setup();
    expect(ipc.localToCanvas).toHaveBeenLastCalledWith({ kind: "mesh", id: "Quad" }, {}, QUAD);
    expect(centers()).toEqual(QUAD.map(screenOf));
  });

  it("writes the dragged point to the current cell with one set_keyform_cell", async () => {
    const { doc, handle, centers } = await setup();
    const start = screen(2);
    drag(handle(2), [start, shift(start, 6, 4), shift(start, 10, -20)]);
    await settle();
    expect(ipc.canvasToLocal).toHaveBeenCalledWith(
      { kind: "mesh", id: "Quad" },
      {},
      [[105, 90]],
      [[100, 100]],
    );
    expect(doc.commands).toEqual([
      {
        op: "set_keyform_cell",
        target: { kind: "mesh", id: "Quad" },
        cell: { AngleX: 0 },
        keyform: {
          kind: "mesh",
          positions: [
            [0, 0],
            [100, 0],
            [105, 90],
            [0, 100],
          ],
        },
      },
    ]);
    // 読み直すまでは元の形 (偽のドキュメントはモデルを変えない)
    expect(centers()).toEqual(QUAD.map(screenOf));
  });

  it("writes to the cell of the current parameter value", async () => {
    const { controller, doc, handle } = await setup();
    controller.setParameter("AngleX", 30);
    await settle();
    const start = screen(0);
    drag(handle(0), [start, shift(start, -8, 0)]);
    await settle();
    expect(doc.commands).toMatchObject([{ op: "set_keyform_cell", cell: { AngleX: 30 } }]);
  });

  it("previews the drag and sends nothing when it is cancelled", async () => {
    const { doc, handle, centers } = await setup();
    const start = screen(1);
    pointer(handle(1), "pointerdown", start);
    pointer(handle(1), "pointermove", shift(start, 12, 0));
    expect(centers()[1]).toEqual(shift(start, 12, 0));
    pointer(handle(1), "pointercancel", shift(start, 12, 0));
    expect(doc.apply).not.toHaveBeenCalled();
    expect(centers()).toEqual(QUAD.map(screenOf));
  });

  it("moves every picked point together", async () => {
    const { doc, handle } = await setup();
    pointer(handle(0), "pointerdown", screen(0));
    pointer(handle(0), "pointerup", screen(0));
    pointer(handle(3), "pointerdown", screen(3), { shiftKey: true });
    pointer(handle(3), "pointerup", screen(3), { shiftKey: true });
    drag(handle(3), [screen(3), shift(screen(3), 0, 10)]);
    await settle();
    expect(doc.commands).toMatchObject([
      {
        keyform: {
          positions: [
            [0, 5],
            [100, 0],
            [100, 100],
            [0, 105],
          ],
        },
      },
    ]);
  });

  it("does not edit off the keys", async () => {
    const { controller, target } = await setup();
    controller.setParameter("AngleX", 10);
    await settle();
    expect(target.querySelector("svg.form")).toBeNull();
    expect(target.querySelector(".vp-band .warn")).not.toBeNull();
  });

  it("ignores pointer events while the previous edit is being sent", async () => {
    const { doc, handle } = await setup();
    const gate = deferred();
    doc.gate = gate.promise;
    const start = screen(1);
    drag(handle(1), [start, shift(start, 20, 0)]);
    await settle();
    drag(handle(2), [screen(2), shift(screen(2), 20, 0)]);
    // 離した後のポインタの移動と離す操作で、送っている編集を送り直さない
    pointer(handle(1), "pointermove", shift(start, 40, 0));
    pointer(handle(1), "pointerup", shift(start, 40, 0));
    gate.resolve();
    await settle();
    expect(doc.apply).toHaveBeenCalledTimes(1);
  });
});

describe("FormTool keys", () => {
  it("selects all points with Ctrl + A and clears them with Esc", async () => {
    const { target } = await setup();
    const selected = () => all(target, "circle.handle.selected").length;
    const event = press(document.body, "a", { ctrlKey: true });
    expect(event.defaultPrevented).toBe(true);
    expect(selected()).toBe(4);
    press(document.body, "Escape");
    expect(selected()).toBe(0);
  });

  it("moves the picked points with the arrow keys, ten pixels with Shift", async () => {
    const { doc, handle } = await setup();
    pointer(handle(2), "pointerdown", screen(2));
    pointer(handle(2), "pointerup", screen(2));
    const event = press(document.body, "ArrowRight");
    expect(event.defaultPrevented).toBe(true);
    await settle();
    press(document.body, "ArrowUp", { shiftKey: true });
    await settle();
    // 画面の 1px は偽のビューの拡大率 2 でキャンバスの 0.5
    expect(doc.commands).toMatchObject([
      {
        op: "set_keyform_cell",
        keyform: {
          positions: [
            [0, 0],
            [100, 0],
            [100.5, 100],
            [0, 100],
          ],
        },
      },
      {
        op: "set_keyform_cell",
        keyform: {
          positions: [
            [0, 0],
            [100, 0],
            [100, 95],
            [0, 100],
          ],
        },
      },
    ]);
  });

  it("picks the next and previous point with ] and [, then moves it with the arrow keys", async () => {
    const { doc, target } = await setup();
    const selected = () =>
      all(target, "circle.handle").flatMap((c, i) => (c.classList.contains("selected") ? [i] : []));
    const status = () => target.querySelector(".vp-band [role=status]")?.textContent?.trim();
    expect(press(document.body, "[").defaultPrevented).toBe(true);
    expect(selected()).toEqual([3]);
    expect(status()).toBe(t("viewport.point.current", { index: 4, total: 4 }));
    press(document.body, "]");
    press(document.body, "]");
    expect(selected()).toEqual([1]);
    expect(target.querySelector("svg.form circle.ring")).not.toBeNull();
    press(document.body, "ArrowDown", { shiftKey: true });
    await settle();
    expect(doc.commands).toMatchObject([
      {
        op: "set_keyform_cell",
        keyform: {
          positions: [
            [0, 0],
            [100, 5],
            [100, 100],
            [0, 100],
          ],
        },
      },
    ]);
    // ほかの数を選ぶと何個を選んだかに戻る
    press(document.body, "a", { ctrlKey: true });
    expect(status()).toBe(t("viewport.form.selected", { count: 4, total: 4 }));
    expect(target.querySelector("svg.form circle.ring")).toBeNull();
  });

  it("pans the view to a point picked with ] outside the view, but not for a point inside or a click", async () => {
    const { controller, handle } = await setup();
    // 点の画面の位置は (10, 20) (210, 20) (210, 220) (10, 220)。ビューは 400×300 で、端の余白は 48
    controller.size = [400, 300];
    press(document.body, "]");
    press(document.body, "]");
    expect(controller.panBy.mock.calls).toEqual([
      [38, 28],
      [0, 28],
    ]);
    press(document.body, "]");
    expect(controller.panBy).toHaveBeenCalledTimes(2);
    // ポインタで選んだときは動かさない
    pointer(handle(0), "pointerdown", screen(0));
    pointer(handle(0), "pointerup", screen(0));
    expect(handle(0).classList.contains("selected")).toBe(true);
    expect(controller.panBy).toHaveBeenCalledTimes(2);
  });

  it("leaves [ and ] to the parts outside the viewport", async () => {
    const { target } = await setup();
    const outside = document.createElement("button");
    document.body.append(outside);
    expect(press(outside, "]").defaultPrevented).toBe(false);
    outside.remove();
    expect(all(target, "circle.handle.selected")).toHaveLength(0);
  });

  it("leaves the arrow keys alone without picked points or outside the viewport", async () => {
    const { doc, handle } = await setup();
    expect(press(document.body, "ArrowRight").defaultPrevented).toBe(false);
    pointer(handle(2), "pointerdown", screen(2));
    pointer(handle(2), "pointerup", screen(2));
    const outside = document.createElement("button");
    document.body.append(outside);
    expect(press(outside, "ArrowRight").defaultPrevented).toBe(false);
    outside.remove();
    await settle();
    expect(doc.apply).not.toHaveBeenCalled();
  });

  it("does not take the keys pressed inside a dialog", async () => {
    const { target, handle } = await setup();
    const selected = () => all(target, "circle.handle.selected").length;
    const inDialog = dialogButton();
    const event = press(inDialog, "a", { ctrlKey: true });
    expect(event.defaultPrevented).toBe(false);
    expect(selected()).toBe(0);
    pointer(handle(1), "pointerdown", screen(1));
    pointer(handle(1), "pointerup", screen(1));
    press(inDialog, "Escape");
    expect(selected()).toBe(1);
  });
});
