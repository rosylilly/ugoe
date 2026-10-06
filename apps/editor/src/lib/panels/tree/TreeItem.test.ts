// @vitest-environment happy-dom
// ツリーの行の名前の変更 (TextField): 開いたら全体を選び、Enter で確定・Esc でやめ、欄の中の押下を行に渡さないこと
// (docs/editor.md#ツリー-左、docs/ui.md#足りない部品と口-t0917)。
import { describe, expect, it, vi } from "vitest";
import { all, get, render, settle } from "../../viewport/tools/test/harness.svelte";
import { buildTree, flatten, indexModel, type TreeRow } from "./build";
import TreeItem from "./TreeItem.svelte";
import { testModel } from "./testModel";

function faceRow(): TreeRow {
  const rows = flatten(buildTree(indexModel(testModel()), "parts"), new Set());
  const row = rows.find((r) => r.node.ref.kind === "part" && r.node.ref.id === "Face");
  if (!row) throw new Error("no Face row");
  return row;
}

function setup(renaming: boolean) {
  const handlers = {
    onselect: vi.fn(),
    ontoggle: vi.fn(),
    onrenamestart: vi.fn(),
    onrename: vi.fn(),
    onpointerdown: vi.fn(),
  };
  const { target } = render(TreeItem, {
    row: faceRow(),
    id: "row-face",
    selected: false,
    active: true,
    expanded: true,
    renaming,
    drop: null,
    ...handlers,
  });
  return { target, ...handlers };
}

function key(el: Element, key: string): void {
  el.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true }));
}

describe("TreeItem", () => {
  it("opens the rename field with the whole name selected", async () => {
    const { target } = setup(true);
    await settle();
    const input = get<HTMLInputElement>(target, ".rename input");
    expect(input.value).toBe("Face");
    expect(document.activeElement).toBe(input);
    expect([input.selectionStart, input.selectionEnd]).toEqual([0, 4]);
    // 行の高さに収める (押せる大きさは 24px)
    expect(input.classList.contains("ugoe-text-field__input")).toBe(true);
  });

  it("commits the typed name on Enter only once", async () => {
    const { target, onrename } = setup(true);
    await settle();
    const input = get<HTMLInputElement>(target, ".rename input");
    input.value = "Head";
    input.dispatchEvent(new Event("input", { bubbles: true }));
    key(input, "Enter");
    input.dispatchEvent(new FocusEvent("blur"));
    expect(onrename).toHaveBeenCalledTimes(1);
    expect(onrename).toHaveBeenCalledWith("Head");
  });

  it("cancels the rename on Escape and keeps keys away from the tree", async () => {
    const { target, onrename } = setup(true);
    await settle();
    const input = get<HTMLInputElement>(target, ".rename input");
    input.value = "Head";
    input.dispatchEvent(new Event("input", { bubbles: true }));
    key(input, "Escape");
    expect(onrename).toHaveBeenCalledWith(null);
    // 木 (Svelte の委譲したキーの処理) へは伝えない
    const arrow = new KeyboardEvent("keydown", {
      key: "ArrowDown",
      bubbles: true,
      cancelable: true,
    });
    input.dispatchEvent(arrow);
    expect(arrow.cancelBubble).toBe(true);
  });

  it("does not pass presses inside the field to the row", async () => {
    const { target, onselect, onpointerdown, onrenamestart } = setup(true);
    await settle();
    const input = get<HTMLInputElement>(target, ".rename input");
    input.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true, button: 0 }));
    input.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    input.dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));
    expect(onpointerdown).not.toHaveBeenCalled();
    expect(onselect).not.toHaveBeenCalled();
    expect(onrenamestart).not.toHaveBeenCalled();
  });

  it("selects and starts renaming from the row", () => {
    const { target, onselect, onrenamestart } = setup(false);
    expect(all(target, "input")).toHaveLength(0);
    const row = get(target, "[role=treeitem]");
    row.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    row.dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));
    expect(onselect).toHaveBeenCalledTimes(1);
    expect(onrenamestart).toHaveBeenCalledTimes(1);
  });
});
