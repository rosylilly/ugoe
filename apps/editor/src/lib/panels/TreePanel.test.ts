// @vitest-environment happy-dom
// ツリー: 選んだ要素を見せるために祖先を開くのは、主な選択が変わったときだけ (T1614)。
// 編集の後の読み直しで、利用者が畳んだ祖先を開き直さない (docs/testing.md#エディタの部品のテスト)。
// キーボード操作 (矢印・Home / End・F2・Delete) と空の状態 (docs/editor.md#ツリー-左、docs/ui.md#デザイン。T0674)。
import { flushSync } from "svelte";
import { describe, expect, it, vi } from "vitest";
import { Selection } from "../selection.svelte";
import { FakeDocument, press, render, settle } from "../viewport/tools/test/harness.svelte";
import TreePanel from "./TreePanel.svelte";
import { testModel } from "./tree/testModel";

function setup(model: object = testModel()) {
  const doc = new FakeDocument();
  doc.model = { json: JSON.stringify(model), bundle: null };
  const selection = new Selection();
  const onfileaction = vi.fn();
  const { target } = render(TreePanel, { doc: doc.asDocument, selection, onfileaction });
  return { doc, selection, target, onfileaction };
}

/** `key` の行 (無ければ null)。 */
const row = (key: string) => document.querySelector<HTMLElement>(`[data-key="${key}"]`);

/** 行の開閉のボタンを押す。 */
function toggle(key: string): void {
  const button = row(key)?.querySelector<HTMLButtonElement>("button.twisty");
  if (!button) throw new Error(`no toggle for ${key}`);
  button.click();
  flushSync();
}

describe("TreePanel reveal", () => {
  it("opens the ancestors only when the primary selection changes", () => {
    const { doc, selection } = setup();
    toggle("part:Face");
    expect(row("mesh:EyeL")).toBeNull();
    // 外で選ぶと祖先を開く。
    selection.set([{ kind: "mesh", id: "EyeL" }]);
    flushSync();
    expect(row("part:Face")?.getAttribute("aria-expanded")).toBe("true");
    expect(row("mesh:EyeL")).not.toBeNull();
    // 利用者が畳んだ祖先は、編集の後の読み直しで開き直さない。
    toggle("part:Face");
    const model = testModel();
    model.parts = model.parts.map((p) => (p.id === "Hidden" ? { ...p, name: "Hidden2" } : p));
    doc.model = { json: JSON.stringify(model), bundle: null };
    flushSync();
    expect(row("part:Face")?.getAttribute("aria-expanded")).toBe("false");
    expect(row("mesh:EyeL")).toBeNull();
    // 主な選択が変われば、また開く。
    selection.set([{ kind: "mesh", id: "EyeR" }]);
    flushSync();
    expect(row("part:Face")?.getAttribute("aria-expanded")).toBe("true");
  });

  it("opens the ancestors once the selected element appears in the tree", () => {
    const { doc, selection } = setup();
    toggle("part:Face");
    // 足した要素を、読み直す前に選ぶ (ツリーにまだ無い)。
    selection.set([{ kind: "mesh", id: "EyeNew" }]);
    flushSync();
    const model = testModel();
    const eyeL = model.meshes.find((m) => m.id === "EyeL");
    if (!eyeL) throw new Error("no EyeL");
    model.meshes.push({ ...eyeL, id: "EyeNew", name: "EyeNew" });
    doc.model = { json: JSON.stringify(model), bundle: null };
    flushSync();
    expect(row("mesh:EyeNew")).not.toBeNull();
  });
});

const selected = (selection: Selection) => selection.items.map((r) => `${r.kind}:${r.id}`);

describe("TreePanel keyboard", () => {
  it("moves the selection with the arrows, Home and End", async () => {
    const { selection, target } = setup();
    await settle();
    const tree = target.querySelector("[role=tree]") as HTMLElement;
    press(tree, "ArrowDown");
    expect(selected(selection)).toEqual(["part:Root"]);
    press(tree, "ArrowDown");
    expect(selected(selection)).toEqual(["part:Face"]);
    press(tree, "ArrowDown", { shiftKey: true });
    expect(selected(selection)).toEqual(["part:Face", "part:Eyes"]);
    press(tree, "End");
    const rows = target.querySelectorAll("[role=treeitem]");
    const last = rows[rows.length - 1] as HTMLElement;
    expect(tree.getAttribute("aria-activedescendant")).toBe(last.id);
    press(tree, "Home");
    expect(selected(selection)).toEqual(["part:Root"]);
  });

  it("collapses and expands with left and right", async () => {
    const { target } = setup();
    await settle();
    const tree = target.querySelector("[role=tree]") as HTMLElement;
    const count = () => target.querySelectorAll("[role=treeitem]").length;
    press(tree, "ArrowDown");
    const open = count();
    press(tree, "ArrowLeft");
    expect(count()).toBe(1);
    press(tree, "ArrowRight");
    expect(count()).toBe(open);
  });

  it("starts renaming with F2 and asks before deleting with Delete", async () => {
    const { target } = setup();
    await settle();
    const tree = target.querySelector("[role=tree]") as HTMLElement;
    press(tree, "ArrowDown");
    press(tree, "F2");
    await settle();
    expect(target.querySelector(".rename input")).not.toBeNull();
    const input = target.querySelector(".rename input") as HTMLInputElement;
    press(input, "Escape");
    await settle();
    expect(target.querySelector(".rename input")).toBeNull();
    press(tree, "Delete");
    await settle();
    expect(document.querySelector("[role=alertdialog]")).not.toBeNull();
  });
});

describe("TreePanel empty state", () => {
  it("shows the next step outside the tree when there are no rows", async () => {
    const model = { ...testModel(), parts: [], deformers: [], meshes: [] };
    const { target, onfileaction } = setup(model);
    await settle();
    expect(target.querySelector("[role=tree]")).toBeNull();
    const button = [...target.querySelectorAll("button")].find(
      (b) => b.textContent === "Import PSD…",
    );
    if (!button) throw new Error("no Import PSD… button");
    button.click();
    expect(onfileaction).toHaveBeenCalledWith("importPsd");
  });
});
