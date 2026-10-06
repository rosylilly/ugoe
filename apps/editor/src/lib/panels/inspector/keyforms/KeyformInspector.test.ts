// @vitest-environment happy-dom
// インスペクタのキーフォームの軸: 「キーへ移る」はスライダーをそのキーへ動かし、「キーを消す」は軸のキーを消すコマンドを送る
// (docs/editor.md#キーフォームの編集、docs/testing.md#エディタの部品のテスト)。押せる範囲は 24px (docs/ui.md#そのほか。T1634)。
import { describe, expect, it } from "vitest";
import { Selection } from "../../../selection.svelte";
import {
  all,
  FakeController,
  FakeDocument,
  render,
  settle,
} from "../../../viewport/tools/test/harness.svelte";
import KeyformInspector from "./KeyformInspector.svelte";

function setup() {
  const doc = new FakeDocument();
  const selection = new Selection();
  selection.set([{ kind: "mesh", id: "Quad" }]);
  const viewport = new FakeController();
  const { target } = render(KeyformInspector, {
    doc: doc.asDocument,
    selection,
    viewport: viewport.asController,
  });
  return { doc, viewport, target };
}

describe("KeyformInspector axis keys", () => {
  it("moves the slider to the key", async () => {
    const { viewport, target } = setup();
    const go = all<HTMLButtonElement>(target, ".chip button.go");
    expect(go.map((b) => b.textContent)).toEqual(["-30", "0", "30"]);
    go[2]?.click();
    await settle();
    expect(viewport.parameterValues.AngleX).toBe(30);
  });

  it("removes the key of the axis", async () => {
    const { doc, target } = setup();
    all<HTMLButtonElement>(target, ".chip button.remove")[0]?.click();
    await settle();
    expect(doc.commands).toEqual([
      {
        op: "remove_keyform_key",
        target: { kind: "mesh", id: "Quad" },
        parameter: "AngleX",
        key: -30,
      },
    ]);
  });
});
