// @vitest-environment happy-dom
// グルーのパネルの数値の欄 (docs/editor.md#グルーの編集、docs/testing.md#エディタの部品のテスト。T1642):
// 強さの軸のキーの欄・新しい軸の最初のキーの欄は打つたびに (oninput) ボタンの可否を決め、空・同じキー・読めない文字を
// 送らない。範囲の外の値は欄が範囲に収める。強さ・距離の欄は確定で送る。
import { flushSync } from "svelte";
import { describe, expect, it } from "vitest";
import type { ParameterDef } from "../../document.svelte";
import { Selection } from "../../selection.svelte";
import {
  all,
  FakeDocument,
  get,
  press,
  render,
  settle,
  testModel,
} from "../../viewport/tools/test/harness.svelte";
import GluesPanel from "./GluesPanel.svelte";
import { GlueView } from "./glueView.svelte";

function parameter(id: string, min: number, max: number, value: number): ParameterDef {
  return { id, name: id, min, max, default: value, kind: "normal", mirror: null, group: null };
}

/**
 * グルー Seam (Quad と Quad2) を選んだパネル。`keys` を渡すと、Seam の強さは AngleX のその キーの軸を持つ。
 * パラメータは AngleX (-30..30、既定 0) と EyeOpen (0..1、既定 1)。
 */
function setup(keys: number[] | null = [0]) {
  const doc = new FakeDocument();
  const model = JSON.parse(testModel());
  model.meshes.push({ ...model.meshes[0], id: "Quad2", name: "Quad2" });
  model.glues[0] = {
    ...model.glues[0],
    mesh_b: "Quad2",
    keyforms: keys
      ? {
          bindings: [{ parameter: "AngleX", keys }],
          keyforms: keys.map(() => ({ intensity: 1 })),
        }
      : { bindings: [], keyforms: [{ intensity: 1 }] },
  };
  doc.model = { json: JSON.stringify(model), bundle: null };
  doc.parameters = [parameter("AngleX", -30, 30, 0), parameter("EyeOpen", 0, 1, 1)];
  const view = new GlueView();
  view.glue = "Seam";
  const { target } = render(GluesPanel, { doc: doc.asDocument, selection: new Selection(), view });
  return { doc, target, view };
}

/** 欄に文字を打つ (input を送り、Svelte の更新を流す。確定はしない)。 */
function type(input: HTMLInputElement, text: string): void {
  input.focus();
  input.value = text;
  input.dispatchEvent(new Event("input", { bubbles: true }));
  flushSync();
}

/** 欄に文字を打って Enter で確定する。 */
function enter(input: HTMLInputElement, text: string): void {
  type(input, text);
  press(input, "Enter");
}

function field(target: HTMLElement, label: string): HTMLInputElement {
  return get<HTMLInputElement>(target, `input[aria-label="${label}"]`);
}

/** 読み上げの名前が `label` の欄 (`aria-labelledby` か見えるラベル)。 */
function labelledField(target: HTMLElement, label: string): HTMLInputElement {
  const input = all<HTMLInputElement>(target, "input").find((i) =>
    (i.getAttribute("aria-labelledby") ?? "")
      .split(/\s+/)
      .some((id) => document.getElementById(id)?.textContent?.trim() === label),
  );
  if (!input) throw new Error(`no field: ${label}`);
  return input;
}

function button(target: HTMLElement, text: string): HTMLButtonElement {
  const found = all<HTMLButtonElement>(target, "button").find(
    (b) => b.textContent?.trim() === text,
  );
  if (!found) throw new Error(`no button: ${text}`);
  return found;
}

/** ボタンを押す。ポインタで押したときと同じく先にフォーカスを移す (打っていた欄はフォーカスを外して確定する)。 */
async function click(el: HTMLElement): Promise<void> {
  el.focus();
  flushSync();
  el.click();
  await settle();
}

describe("GluesPanel new key of an intensity axis", () => {
  it("cannot add a key while the field is empty", () => {
    const { target } = setup();
    expect(field(target, "New key of AngleX").value).toBe("");
    expect(button(target, "Key").disabled).toBe(true);
  });

  it("enables the button as a key is typed, before it is committed", () => {
    const { target } = setup();
    const input = field(target, "New key of AngleX");
    type(input, "15");
    expect(button(target, "Key").disabled).toBe(false);
    // 欄を空にし直すと押せない
    type(input, "");
    expect(button(target, "Key").disabled).toBe(true);
  });

  it("cannot add a key the axis already has", () => {
    const { target } = setup([0, 10]);
    const input = field(target, "New key of AngleX");
    type(input, "10");
    expect(button(target, "Key").disabled).toBe(true);
    // キーフォームの格子と同じ許容差で同じキーとみなす
    type(input, "0.000001");
    expect(button(target, "Key").disabled).toBe(true);
  });

  it("clamps a key out of the parameter's range to its end", async () => {
    const { doc, target } = setup([0, 30]);
    const input = field(target, "New key of AngleX");
    // 範囲に収めると 30 (もうあるキー) になるので押せない
    type(input, "100");
    expect(button(target, "Key").disabled).toBe(true);
    type(input, "-100");
    expect(button(target, "Key").disabled).toBe(false);
    await click(button(target, "Key"));
    expect(doc.commands).toEqual([
      {
        op: "add_keyform_key",
        target: { kind: "glue", id: "Seam" },
        parameter: "AngleX",
        key: -30,
      },
    ]);
  });

  it("keeps the last readable value while the text cannot be read", () => {
    const { target } = setup();
    const input = field(target, "New key of AngleX");
    type(input, "abc");
    expect(button(target, "Key").disabled).toBe(true);
    type(input, "5");
    type(input, "5x");
    expect(button(target, "Key").disabled).toBe(false);
  });

  it("sends add_keyform_key with the typed key and empties the field", async () => {
    const { doc, target } = setup();
    const input = field(target, "New key of AngleX");
    type(input, "12.5");
    await click(button(target, "Key"));
    expect(doc.commands).toEqual([
      {
        op: "add_keyform_key",
        target: { kind: "glue", id: "Seam" },
        parameter: "AngleX",
        key: 12.5,
      },
    ]);
    expect(input.value).toBe("");
    expect(button(target, "Key").disabled).toBe(true);
  });

  it("sends a key committed with Enter", async () => {
    const { doc, target } = setup();
    enter(field(target, "New key of AngleX"), "-7");
    expect(button(target, "Key").disabled).toBe(false);
    await click(button(target, "Key"));
    expect(doc.commands).toEqual([
      { op: "add_keyform_key", target: { kind: "glue", id: "Seam" }, parameter: "AngleX", key: -7 },
    ]);
  });
});

describe("GluesPanel first key of a new intensity axis", () => {
  it("uses the parameter's default while the field is empty", async () => {
    // AngleX は軸にあるので、足せるのは EyeOpen (既定 1) だけ
    const { doc, target } = setup();
    const input = field(target, "First key of the new axis");
    expect(input.value).toBe("");
    expect(input.placeholder).toBe("1");
    expect(button(target, "Axis").disabled).toBe(false);
    await click(button(target, "Axis"));
    expect(doc.commands).toEqual([
      {
        op: "add_keyform_key",
        target: { kind: "glue", id: "Seam" },
        parameter: "EyeOpen",
        key: 1,
      },
    ]);
  });

  it("sends the typed key (clamped to the range) and empties the field", async () => {
    const { doc, target } = setup(null);
    // 軸が無ければ先頭の AngleX を足す
    const input = field(target, "First key of the new axis");
    expect(input.placeholder).toBe("0");
    type(input, "45");
    expect(button(target, "Axis").disabled).toBe(false);
    await click(button(target, "Axis"));
    expect(doc.commands).toEqual([
      {
        op: "add_keyform_key",
        target: { kind: "glue", id: "Seam" },
        parameter: "AngleX",
        key: 30,
      },
    ]);
    expect(input.value).toBe("");
  });

  it("hides the row when every parameter is an axis", () => {
    const { doc, target } = setup();
    doc.parameters = [parameter("AngleX", -30, 30, 0)];
    flushSync();
    expect(target.querySelector('input[aria-label="First key of the new axis"]')).toBeNull();
    expect(all(target, "button").some((b) => b.textContent?.trim() === "Axis")).toBe(false);
  });
});

describe("GluesPanel committed fields", () => {
  it("sends set_glue with the committed intensity of the keyform", async () => {
    const { doc, target } = setup([0, 10]);
    enter(labelledField(target, "AngleX = 10"), "0.25");
    await settle();
    expect(doc.commands).toHaveLength(1);
    const [command] = doc.commands;
    expect(command?.op).toBe("set_glue");
    if (command?.op !== "set_glue") throw new Error("not set_glue");
    expect(command.glue.keyforms.keyforms.map((k) => k.intensity)).toEqual([1, 0.25]);
    expect(command.distance).toBeUndefined();
  });

  it("does not send an intensity whose text cannot be read", async () => {
    const { doc, target } = setup();
    const input = labelledField(target, "AngleX = 0");
    enter(input, "abc");
    await settle();
    expect(doc.commands).toEqual([]);
    expect(input.value).toBe("1");
  });

  it("pairs again within the committed distance, clamped to zero", async () => {
    const { doc, target } = setup();
    enter(labelledField(target, "Pair again within"), "-5");
    expect(button(target, "Pair").disabled).toBe(false);
    await click(button(target, "Pair"));
    expect(doc.commands).toHaveLength(1);
    const [command] = doc.commands;
    if (command?.op !== "set_glue") throw new Error("not set_glue");
    expect(command.glue.id).toBe("Seam");
    expect(command.distance).toBe(0);
  });

  it("creates a glue within the committed distance", async () => {
    const { doc, target, view } = setup();
    const distance = labelledField(target, "Distance");
    expect(distance.value).toBe("8");
    // 空にしても (空を持てない欄なので) 前の値に戻る
    enter(distance, "");
    expect(distance.value).toBe("8");
    enter(distance, "12");
    const create = button(target, "Glue");
    expect(create.disabled).toBe(false);
    doc.onApply = () => {
      const model = JSON.parse(doc.model?.json ?? "{}");
      model.glues.push({ ...model.glues[0], id: "Glue2" });
      doc.model = { json: JSON.stringify(model), bundle: null };
      return true;
    };
    await click(create);
    expect(doc.commands).toEqual([
      { op: "create_glue", mesh_a: "Quad", mesh_b: "Quad2", distance: 12 },
    ]);
    expect(view.glue).toBe("Glue2");
  });
});
