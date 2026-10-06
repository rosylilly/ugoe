// @vitest-environment happy-dom
// ツリーの行の名前の変更 (TextField): 開いたら全体を選び、Enter で確定・Esc でやめ、欄の中の押下を行に渡さないこと
// (docs/editor.md#ツリー-左、docs/ui.md#足りない部品と口-t0917)。
// 本体の札とほかの札を持つ行では、本体の札を 1 つの札にまとめて名前の幅を残し、札の意味を読み上げで残すこと
// (docs/ui.md#エディタのツリーの札と選んだ行-t1654)。マスク専用かつ非表示の行も、マスクの札を非表示の札にまとめること
// (docs/ui.md#エディタのツリーの行の札と強制色-t1670)。
import { describe, expect, it, vi } from "vitest";
import { all, get, render, settle } from "../../viewport/tools/test/harness.svelte";
import { buildTree, flatten, indexModel, type TreeRow } from "./build";
import TreeItem from "./TreeItem.svelte";
import { testModel } from "./testModel";

/** 見本のモデルの Ghost (非表示のパーツの中のメッシュ) をマスク専用にしたもの。 */
function maskModel() {
  const model = testModel();
  return {
    ...model,
    meshes: model.meshes.map((m) => (m.id === "Ghost" ? { ...m, mask_only: true } : m)),
  };
}

function findRow(kind: string, id: string, model = testModel()): TreeRow {
  const rows = flatten(buildTree(indexModel(model), "parts"), new Set());
  const row = rows.find((r) => r.node.ref.kind === kind && r.node.ref.id === id);
  if (!row) throw new Error(`no ${id} row`);
  return row;
}

function setup(
  renaming: boolean,
  {
    id = "Face",
    base = false,
    row = findRow("part", id),
  }: { id?: string; base?: boolean; row?: TreeRow } = {},
) {
  const handlers = {
    onselect: vi.fn(),
    ontoggle: vi.fn(),
    onrenamestart: vi.fn(),
    onrename: vi.fn(),
    onpointerdown: vi.fn(),
  };
  const { target } = render(TreeItem, {
    row,
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

  it("folds the mask tag into the hidden tag on a mask-only hidden row", () => {
    const { target } = setup(false, { row: findRow("mesh", "Ghost", maskModel()) });
    const row = get(target, "[role=treeitem]");
    // マスクは行の印と斜体の名前で見えるので、見える札は非表示の 1 つ。読み上げは「mask hidden」
    expect(row.classList.contains("mask-only")).toBe(true);
    expect(get(target, ".name").textContent).toBe("Ghost");
    const tags = all(target, ".ed-tag");
    expect(tags).toHaveLength(1);
    const tag = tags[0] as HTMLElement;
    expect(tag.classList.contains("base-merged")).toBe(false);
    expect(get(tag, ".ugoe-visually-hidden").textContent?.trim()).toBe("mask");
    expect(tag.textContent?.replace(/\s+/g, " ").trim()).toBe("mask hidden");
    expect(tag.title).toBe(
      "Mask only: not drawn as color\nOpacity is 0 at the default parameter values",
    );
  });

  it("folds both the base and mask tags into the hidden tag", () => {
    const { target } = setup(false, { base: true, row: findRow("mesh", "Ghost", maskModel()) });
    const tags = all(target, ".ed-tag");
    expect(tags).toHaveLength(1);
    const tag = tags[0] as HTMLElement;
    expect(tag.classList.contains("base-merged")).toBe(true);
    expect(tag.textContent?.replace(/\s+/g, " ").trim()).toBe("base mask hidden");
    expect(tag.title.split("\n")).toHaveLength(3);
  });

  it("folds the base tag into the mask tag", () => {
    const { target } = setup(false, { base: true, row: findRow("mesh", "EyeMask") });
    const tags = all(target, ".ed-tag");
    expect(tags).toHaveLength(1);
    expect(tags[0]?.classList.contains("base-merged")).toBe(true);
    expect(tags[0]?.textContent?.replace(/\s+/g, " ").trim()).toBe("base mask");
  });
});
