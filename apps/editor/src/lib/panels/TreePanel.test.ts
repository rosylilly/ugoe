// @vitest-environment happy-dom
// ツリー: 選んだ要素を見せるために祖先を開くのは、主な選択が変わったときだけ (T1614)。
// 編集の後の読み直しで、利用者が畳んだ祖先を開き直さない (docs/testing.md#エディタの部品のテスト)。
import { flushSync } from "svelte";
import { describe, expect, it } from "vitest";
import { Selection } from "../selection.svelte";
import { FakeDocument, render } from "../viewport/tools/test/harness.svelte";
import TreePanel from "./TreePanel.svelte";
import { testModel } from "./tree/testModel";

function setup() {
  const doc = new FakeDocument();
  doc.model = { json: JSON.stringify(testModel()), bundle: null };
  const selection = new Selection();
  const { target } = render(TreePanel, { doc: doc.asDocument, selection });
  return { doc, selection, target };
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
