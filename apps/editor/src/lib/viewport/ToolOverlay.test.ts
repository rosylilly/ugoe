// @vitest-environment happy-dom
// ビューポートのツールの切り替え: メッシュ編集モードのキー操作 (docs/testing.md#エディタの部品のテスト)。
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
