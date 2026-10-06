// @vitest-environment happy-dom
// インスペクタのキーフォームの軸: 「キーへ移る」はスライダーをそのキーへ動かし、「キーを消す」は軸のキーを消すコマンドを送る
// (docs/editor.md#キーフォームの編集、docs/testing.md#エディタの部品のテスト)。押せる範囲は 24px (docs/ui.md#そのほか。T1634)。
// 回転デフォーマの角度と拡大率の欄は、確定で今のセルに set_keyform_cell を送る (T1636)。
import { flushSync } from "svelte";
import { describe, expect, it } from "vitest";
import type { Command } from "../../../ipc";
import { Selection } from "../../../selection.svelte";
import {
  all,
  FakeController,
  FakeDocument,
  render,
  settle,
} from "../../../viewport/tools/test/harness.svelte";
import KeyformInspector from "./KeyformInspector.svelte";

function setup(selected: "Quad" | "Rot" = "Quad") {
  const doc = new FakeDocument();
  const selection = new Selection();
  selection.set([{ kind: selected === "Quad" ? "mesh" : "deformer", id: selected }]);
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

const ROT_KEYFORM = { origin: [50, 50], angle: 0, scale: 1, reflect_x: false, reflect_y: false };

/** 欄 (見えるラベルで探す) に文字を入れて Enter で確定する。 */
function enter(target: HTMLElement, label: string, text: string): void {
  const input = rotationField(target, label);
  input.focus();
  input.value = text;
  input.dispatchEvent(new Event("input", { bubbles: true }));
  flushSync();
  input.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
  flushSync();
}

function rotationField(target: HTMLElement, label: string): HTMLInputElement {
  const id = all(target, ".rotation label").find((l) => l.textContent === label)?.id;
  const input = target.querySelector<HTMLInputElement>(`input[aria-labelledby="${id}"]`);
  if (!id || !input) throw new Error(`no field: ${label}`);
  return input;
}

/** 回転デフォーマ Rot の格子を AngleX の 2 つのキー (-30・30) の軸にしたモデル。 */
function withRotAxis(doc: FakeDocument): void {
  const model = JSON.parse(doc.model?.json ?? "{}");
  const rot = model.deformers.find((d: { id: string }) => d.id === "Rot");
  rot.kind.keyforms = {
    bindings: [{ parameter: "AngleX", keys: [-30, 30] }],
    keyforms: [ROT_KEYFORM, { ...ROT_KEYFORM, angle: 20, scale: 2 }],
  };
  doc.model = { json: JSON.stringify(model), bundle: null };
  flushSync();
}

/** 適用した set_keyform_cell をモデルに書き込む (本物の読み直しの代わり)。 */
function applyCells(doc: FakeDocument): void {
  doc.onApply = (command: Command) => {
    if (command.op !== "set_keyform_cell") return;
    const model = JSON.parse(doc.model?.json ?? "{}");
    const rot = model.deformers.find((d: { id: string }) => d.id === command.target.id);
    const { kind: _, ...keyform } = command.keyform as Record<string, unknown>;
    rot.kind.keyforms.keyforms[0] = keyform;
    doc.model = { json: JSON.stringify(model), bundle: null };
    return true;
  };
}

describe("KeyformInspector rotation fields", () => {
  it("shows the angle and scale of the current cell", () => {
    const { target } = setup("Rot");
    expect(rotationField(target, "Angle (this cell)").value).toBe("0");
    expect(rotationField(target, "Scale (this cell)").value).toBe("1");
  });

  it("sends set_keyform_cell with the committed angle", async () => {
    const { doc, target } = setup("Rot");
    enter(target, "Angle (this cell)", "12.5");
    await settle();
    expect(doc.commands).toEqual([
      {
        op: "set_keyform_cell",
        target: { kind: "deformer", id: "Rot" },
        cell: {},
        keyform: { ...ROT_KEYFORM, angle: 12.5, kind: "rotation" },
      },
    ]);
  });

  it("sends set_keyform_cell with the committed scale, kept positive", async () => {
    const { doc, target } = setup("Rot");
    enter(target, "Scale (this cell)", "0");
    await settle();
    expect(doc.commands).toEqual([
      {
        op: "set_keyform_cell",
        target: { kind: "deformer", id: "Rot" },
        cell: {},
        keyform: { ...ROT_KEYFORM, scale: 0.001, kind: "rotation" },
      },
    ]);
  });

  it("builds each commit from the latest model (keeps the other field)", async () => {
    const { doc, target } = setup("Rot");
    applyCells(doc);
    enter(target, "Angle (this cell)", "30");
    enter(target, "Scale (this cell)", "1.5");
    for (let i = 0; i < 4; i++) await settle();
    expect(doc.commands.map((c) => c.op === "set_keyform_cell" && c.keyform)).toEqual([
      { ...ROT_KEYFORM, angle: 30, kind: "rotation" },
      { ...ROT_KEYFORM, angle: 30, scale: 1.5, kind: "rotation" },
    ]);
  });

  it("writes the cell of the current key", async () => {
    const { doc, viewport, target } = setup("Rot");
    withRotAxis(doc);
    viewport.setParameter("AngleX", 30);
    flushSync();
    expect(rotationField(target, "Angle (this cell)").value).toBe("20");
    enter(target, "Scale (this cell)", "3");
    await settle();
    expect(doc.commands).toEqual([
      {
        op: "set_keyform_cell",
        target: { kind: "deformer", id: "Rot" },
        cell: { AngleX: 30 },
        keyform: { ...ROT_KEYFORM, angle: 20, scale: 3, kind: "rotation" },
      },
    ]);
  });

  it("disables the empty fields between keys", () => {
    const { doc, target } = setup("Rot");
    withRotAxis(doc);
    for (const label of ["Angle (this cell)", "Scale (this cell)"]) {
      const input = rotationField(target, label);
      expect(input.disabled).toBe(true);
      expect(input.value).toBe("");
    }
  });

  it("has no rotation fields for a mesh", () => {
    const { target } = setup();
    expect(all(target, ".rotation")).toEqual([]);
  });
});
