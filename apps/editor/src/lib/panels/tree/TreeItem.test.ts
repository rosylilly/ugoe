// @vitest-environment happy-dom
// ツリーの行の名前の変更 (TextField): 開いたら全体を選び、Enter で確定・Esc でやめ、欄の中の押下を行に渡さないこと
// (docs/editor.md#ツリー-左、docs/ui.md#足りない部品と口-t0917)。
// 本体の札とほかの札を持つ行では、本体の札を 1 つの札にまとめて名前の幅を残し、札の意味を読み上げで残すこと
// (docs/ui.md#エディタのツリーの札と選んだ行-t1654)。
import { describe, expect, it, vi } from "vitest";
import { all, get, render, settle } from "../../viewport/tools/test/harness.svelte";
import { buildTree, flatten, indexModel, type TreeRow } from "./build";
import TreeItem from "./TreeItem.svelte";
import { testModel } from "./testModel";

function partRow(id: string): TreeRow {
  const rows = flatten(buildTree(indexModel(testModel()), "parts"), new Set());
  const row = rows.find((r) => r.node.ref.kind === "part" && r.node.ref.id === id);
  if (!row) throw new Error(`no ${id} row`);
  return row;
}

function setup(renaming: boolean, { id = "Face", base = false } = {}) {
  const handlers = {
    onselect: vi.fn(),
    ontoggle: vi.fn(),
    onrenamestart: vi.fn(),
    onrename: vi.fn(),
    onpointerdown: vi.fn(),
  };
  const { target } = render(TreeItem, {
    row: partRow(id),
    id: "row-face",
    base,
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

  it("folds the base tag into the other tag and keeps both for screen readers", () => {
    const { target } = setup(false, { id: "Hidden", base: true });
    expect(get(target, ".name").textContent).toBe("Hidden");
    const tags = all(target, ".ed-tag");
    // 見える札は 1 つ (本体は破線の枠にまとめる)。読み上げは「base hidden」
    expect(tags).toHaveLength(1);
    const tag = tags[0] as HTMLElement;
    expect(tag.classList.contains("base-merged")).toBe(true);
    expect(get(tag, ".ugoe-visually-hidden").textContent?.trim()).toBe("base");
    expect(tag.textContent?.replace(/\s+/g, " ").trim()).toBe("base hidden");
    expect(tag.title).toBe(
      "Part of the base model: read-only in a module project\nOpacity is 0 at the default parameter values",
    );
    expect(get(target, "[role=treeitem]").textContent).toContain("base hidden");
  });

  it("shows a single tag as is", () => {
    const base = setup(false, { base: true }).target;
    const baseTags = all(base, ".ed-tag");
    expect(baseTags.map((tag) => tag.textContent)).toEqual(["base"]);
    expect(baseTags[0]?.classList.contains("base-merged")).toBe(false);
    const hidden = setup(false, { id: "Hidden" }).target;
    const hiddenTags = all(hidden, ".ed-tag");
    expect(hiddenTags.map((tag) => tag.textContent)).toEqual(["hidden"]);
    expect(all(hidden, ".ugoe-visually-hidden")).toHaveLength(0);
    expect((hiddenTags[0] as HTMLElement).title).toBe(
      "Opacity is 0 at the default parameter values",
    );
  });
});
