// @vitest-environment happy-dom
// フォーム編集ツール: ポインタの操作から送るコマンド (docs/testing.md#エディタの部品のテスト)。
import { beforeEach, describe, expect, it, vi } from "vitest";
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

/** メッシュ Quad を選び、フォーム編集を有効にして点が出るまで待つ。 */
async function setup() {
  const controller = new FakeController();
  const doc = new FakeDocument();
  const selection = new Selection();
  selection.set([{ kind: "mesh", id: "Quad" }]);
  const { target } = render(FormTool, {
    controller: controller.asController,
    doc: doc.asDocument,
    selection,
  });
  get<HTMLButtonElement>(target, ".bar button[aria-pressed]").click();
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
    expect(target.querySelector(".bar .warn")).not.toBeNull();
  });

  it("ignores pointer events while the previous edit is being sent", async () => {
    const { doc, handle } = await setup();
    const gate = deferred();
    doc.pending = gate.promise;
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
