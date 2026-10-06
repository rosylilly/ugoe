// @vitest-environment happy-dom
// ビューポートのツールの切り替え: メッシュ編集モードのキー操作 (docs/testing.md#エディタの部品のテスト)。
import { flushSync } from "svelte";
import { describe, expect, it, vi } from "vitest";
import { GlueView } from "../panels/glues/glueView.svelte";
import { PhysicsView } from "../panels/physics/physicsView.svelte";
import { VerifyView } from "../panels/verify/verifyView.svelte";
import { Selection } from "../selection.svelte";
import ToolOverlay from "./ToolOverlay.svelte";
import {
  dialogButton,
  FakeController,
  FakeDocument,
  pointer,
  press,
  render,
  settle,
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
  render(ToolOverlay, {
    controller: controller.asController,
    doc: doc.asDocument,
    selection,
    physics: new PhysicsView(),
    verify: new VerifyView(),
    glue: new GlueView(),
  });
  return { doc, selection };
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
  function setupHover(selected: boolean) {
    const selection = new Selection();
    if (selected) selection.set([{ kind: "mesh", id: "Quad" }]);
    const { target } = render(ToolOverlay, {
      controller: new FakeController().asController,
      doc: new FakeDocument().asDocument,
      selection,
      physics: new PhysicsView(),
      verify: new VerifyView(),
      glue: new GlueView(),
    });
    return { selection, target };
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

  it("does not change the hover while a button is held", async () => {
    const { selection, target } = setupHover(false);
    await settle();
    const svg = target.querySelector("svg.layer") as SVGSVGElement;
    pointer(svg, "pointermove", [60, 60], { buttons: 1 });
    expect(selection.hover).toBeNull();
  });
});
