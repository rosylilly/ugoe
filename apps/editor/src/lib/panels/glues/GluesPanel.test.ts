// @vitest-environment happy-dom
// グルーのパネルの数値の欄 (docs/editor.md#グルーの編集、docs/testing.md#エディタの部品のテスト。T1642):
// 強さの軸のキーの欄・新しい軸の最初のキーの欄は打つたびに (oninput) ボタンの可否を決め、空・同じキー・読めない文字を
// 送らない。範囲の外の値は欄が範囲に収める。強さ・距離の欄は確定で送る。
import { flushSync } from "svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ParameterDef } from "../../document.svelte";
import { Selection } from "../../selection.svelte";
import {
  all,
  deferred,
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

describe("GluesPanel pair list (T1648)", () => {
  // Seam は Quad の頂点 2 (番号は 1 から) と Quad2 の頂点 1 の組を持つ。両方のメッシュは 4 頂点。
  function vertexField(target: HTMLElement, side: "A" | "B"): HTMLInputElement {
    return labelledField(target, `Vertex of ${side}`);
  }

  it("lists each pair with its vertex numbers counted from 1", () => {
    const { target } = setup(null);
    const rows = all(target, 'ul[aria-label="Vertex pairs of Seam"] li');
    expect(rows.map((r) => r.textContent?.trim())).toEqual(["A 2 ↔ B 1"]);
  });

  it("removes one pair with one set_glue", async () => {
    const { doc, target } = setup(null);
    const remove = get<HTMLButtonElement>(
      target,
      'button[aria-label="Remove the pair of vertex 2 of Quad and vertex 1 of Quad2"]',
    );
    await click(remove);
    expect(doc.commands).toHaveLength(1);
    const [command] = doc.commands;
    if (command?.op !== "set_glue") throw new Error("not set_glue");
    expect(command.glue.pairs).toEqual([]);
    expect(command.distance).toBeUndefined();
  });

  it("adds a pair from the two vertex numbers with one set_glue and empties the fields", async () => {
    const { doc, target } = setup(null);
    const add = button(target, "Add pair");
    expect(add.disabled).toBe(true);
    type(vertexField(target, "A"), "4");
    expect(add.disabled).toBe(true);
    type(vertexField(target, "B"), "3");
    expect(add.disabled).toBe(false);
    await click(add);
    expect(doc.commands).toHaveLength(1);
    const [command] = doc.commands;
    if (command?.op !== "set_glue") throw new Error("not set_glue");
    expect(command.glue.pairs).toEqual([
      { a: 1, b: 0, weight: 1 },
      { a: 3, b: 2, weight: 1 },
    ]);
    expect(vertexField(target, "A").value).toBe("");
    expect(vertexField(target, "B").value).toBe("");
  });

  it("does not add a pair that exists or a vertex the mesh does not have", () => {
    const { target } = setup(null);
    const add = button(target, "Add pair");
    type(vertexField(target, "A"), "2");
    type(vertexField(target, "B"), "1");
    expect(add.disabled).toBe(true);
    expect(target.textContent).toContain("These vertices are already a pair.");
    // 整数でない番号は送らない (範囲の外は欄が 1..4 に収める)
    type(vertexField(target, "B"), "1.5");
    expect(add.disabled).toBe(true);
    type(vertexField(target, "B"), "9");
    expect(add.disabled).toBe(false);
  });
});

describe("GluesPanel pair highlight (T1651)", () => {
  // Seam の組は Quad の頂点 2 と Quad2 の頂点 1 (0 から数えて a 1・b 0)。
  function row(target: HTMLElement): HTMLElement {
    return get(target, 'ul[aria-label="Vertex pairs of Seam"] li');
  }

  function removeButton(target: HTMLElement): HTMLButtonElement {
    return get<HTMLButtonElement>(
      target,
      'button[aria-label="Remove the pair of vertex 2 of Quad and vertex 1 of Quad2"]',
    );
  }

  it("highlights the pair while its remove button has focus, without sending a command", () => {
    const { doc, target, view } = setup(null);
    expect(view.highlightedPair).toBeNull();
    removeButton(target).focus();
    flushSync();
    expect(view.highlightedPair).toEqual({ a: 1, b: 0 });
    expect(row(target).classList.contains("highlighted")).toBe(true);
    removeButton(target).blur();
    flushSync();
    expect(view.highlightedPair).toBeNull();
    expect(row(target).classList.contains("highlighted")).toBe(false);
    expect(doc.commands).toEqual([]);
  });

  it("highlights the pair while the pointer is over its row", () => {
    const { target, view } = setup(null);
    row(target).dispatchEvent(new PointerEvent("pointerenter"));
    flushSync();
    expect(view.highlightedPair).toEqual({ a: 1, b: 0 });
    row(target).dispatchEvent(new PointerEvent("pointerleave"));
    flushSync();
    expect(view.highlightedPair).toBeNull();
  });

  it("forgets the highlight when the pair goes away", async () => {
    const { doc, target, view } = setup(null);
    row(target).dispatchEvent(new PointerEvent("pointerenter"));
    flushSync();
    // 組を外した後の読み直し (行は pointerleave を出さずに消える)。
    const model = JSON.parse(doc.model?.json ?? "{}");
    model.glues[0].pairs = [];
    doc.model = { json: JSON.stringify(model), bundle: null };
    await settle();
    flushSync();
    expect(view.highlightedPair).toBeNull();
  });

  it("forgets the highlight when another glue is chosen", () => {
    const { target, view } = setup(null);
    removeButton(target).focus();
    flushSync();
    expect(view.highlightedPair).not.toBeNull();
    view.glue = null;
    flushSync();
    expect(view.highlightedPair).toBeNull();
  });
});

describe("GluesPanel pair pointed at in the viewport (T1664)", () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  /** Seam の組を (a 0..count-1, b 0) にしたパネル。 */
  function setupPairs(count: number) {
    const ctx = setup(null);
    const model = JSON.parse(ctx.doc.model?.json ?? "{}");
    model.glues[0].pairs = Array.from({ length: count }, (_, a) => ({ a, b: 0, weight: 1 }));
    ctx.doc.model = { json: JSON.stringify(model), bundle: null };
    flushSync();
    return ctx;
  }

  function list(target: HTMLElement): HTMLUListElement {
    return get<HTMLUListElement>(target, 'ul[aria-label="Vertex pairs of Seam"]');
  }

  /** 一覧の見える所を 0..100、`i` 番目の行を 30 * i .. 30 * i + 20 (一覧を送った分だけ上へ) にする。 */
  function layOut(target: HTMLElement): void {
    const ul = list(target);
    const rect = (top: number, bottom: number) =>
      ({ top, bottom, left: 0, right: 100, width: 100, height: bottom - top }) as DOMRect;
    ul.getBoundingClientRect = () => rect(0, 100);
    Array.from(ul.children).forEach((li, i) => {
      (li as HTMLElement).getBoundingClientRect = () =>
        rect(30 * i - ul.scrollTop, 30 * i + 20 - ul.scrollTop);
    });
  }

  it("highlights the row of the pair and forgets it, without sending a command", () => {
    const { doc, target, view } = setupPairs(2);
    view.viewportPair = { a: 1, b: 0 };
    flushSync();
    const rows = all(target, 'ul[aria-label="Vertex pairs of Seam"] li');
    expect(rows.map((r) => r.classList.contains("highlighted"))).toEqual([false, true]);
    view.viewportPair = null;
    flushSync();
    expect(rows.some((r) => r.classList.contains("highlighted"))).toBe(false);
    expect(doc.commands).toEqual([]);
  });

  it("scrolls the list to a hidden row once the pointer rests on its pair", () => {
    vi.useFakeTimers();
    const { target, view } = setupPairs(8);
    layOut(target);
    const ul = list(target);
    view.viewportPair = { a: 6, b: 0 };
    flushSync();
    // 通り過ぎる間は送らない
    vi.advanceTimersByTime(100);
    expect(ul.scrollTop).toBe(0);
    vi.advanceTimersByTime(200);
    // 行 6 (180..200) の下端を一覧の下端 (100) に合わせる
    expect(ul.scrollTop).toBe(100);
    // 見えている行では動かさない
    view.viewportPair = { a: 5, b: 0 };
    flushSync();
    vi.advanceTimersByTime(300);
    expect(ul.scrollTop).toBe(100);
    // 上に隠れた行は上端に合わせる
    view.viewportPair = { a: 1, b: 0 };
    flushSync();
    vi.advanceTimersByTime(300);
    expect(ul.scrollTop).toBe(30);
  });

  it("does not scroll when the pointer leaves the pair before resting", () => {
    vi.useFakeTimers();
    const { target, view } = setupPairs(8);
    layOut(target);
    view.viewportPair = { a: 7, b: 0 };
    flushSync();
    vi.advanceTimersByTime(100);
    view.viewportPair = null;
    flushSync();
    vi.advanceTimersByTime(500);
    expect(list(target).scrollTop).toBe(0);
  });
});

describe("GluesPanel while sending (T1652)", () => {
  it("cannot add a key, an axis or a pair until the sent edit is applied", async () => {
    const { doc, target } = setup();
    type(field(target, "New key of AngleX"), "12");
    type(labelledField(target, "Vertex of A"), "4");
    type(labelledField(target, "Vertex of B"), "3");
    expect(button(target, "Axis").disabled).toBe(false);
    expect(button(target, "Add pair").disabled).toBe(false);
    const gate = deferred();
    doc.gate = gate.promise;
    await click(button(target, "Key"));
    expect(doc.pending).toBe(1);
    expect(button(target, "Key").disabled).toBe(true);
    expect(button(target, "Axis").disabled).toBe(true);
    expect(button(target, "Add pair").disabled).toBe(true);
    // 押せないボタンを押しても送らない
    await click(button(target, "Axis"));
    await click(button(target, "Add pair"));
    expect(doc.commands).toHaveLength(1);

    gate.resolve();
    await settle();
    expect(doc.pending).toBe(0);
    expect(button(target, "Axis").disabled).toBe(false);
    expect(button(target, "Add pair").disabled).toBe(false);
    type(field(target, "New key of AngleX"), "5");
    expect(button(target, "Key").disabled).toBe(false);
  });
});
