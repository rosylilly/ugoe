// @vitest-environment happy-dom
// インスペクタの推定の節の欄 (docs/editor.md#推定とミラー、docs/testing.md#エディタの部品のテスト。T1642):
// 推定の設定は誤りがあれば適用できない、ミラーの軸と許す差は空なら送らない、左右の対の軸は打つたびに (oninput)
// 「Set partner」の可否を決める。どれも確定で送るコマンドとその値を確かめる。
import { flushSync } from "svelte";
import { describe, expect, it, vi } from "vitest";
import type { Command } from "../../../ipc";
import { Selection } from "../../../selection.svelte";
import {
  all,
  FakeController,
  FakeDocument,
  get,
  press,
  render,
  settle,
  testModel,
} from "../../../viewport/tools/test/harness.svelte";
import InferenceSection from "./InferenceSection.svelte";

const QUAD = { kind: "mesh", id: "Quad" } as const;

/**
 * メッシュ Quad (AngleX の軸。点の外接矩形の中心は (50, 50)) を選んだ節。Quad の対の相手の候補に QuadR を置く。
 * `pair` を渡すと Quad をそれと対にする。
 */
function setup(pair: { mirror: string; axis: number | null } | null = null) {
  const doc = new FakeDocument();
  const model = JSON.parse(testModel());
  model.meshes.push({ ...model.meshes[0], id: "QuadR", name: "QuadR" });
  if (pair) {
    model.meshes[0].mirror = pair.mirror;
    model.meshes[0].mirror_axis = pair.axis;
  }
  doc.model = { json: JSON.stringify(model), bundle: null };
  const selection = new Selection();
  selection.set([QUAD]);
  const { target } = render(InferenceSection, {
    doc: doc.asDocument,
    selection,
    viewport: new FakeController().asController,
  });
  return { doc, target };
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

/** `aria-labelledby` の先の文字が `label` の要素。 */
function labelled<E extends HTMLElement>(target: HTMLElement, selector: string, label: string): E {
  const found = all<E>(target, selector).find((el) =>
    (el.getAttribute("aria-labelledby") ?? "")
      .split(/\s+/)
      .some((id) => document.getElementById(id)?.textContent?.trim() === label),
  );
  if (!found) throw new Error(`not found: ${selector} labelled ${label}`);
  return found;
}

function field(target: HTMLElement, label: string): HTMLInputElement {
  return labelled<HTMLInputElement>(target, "input", label);
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

/**
 * ラベルが `label` の Select で `option` を選ぶ。今の項目から矢印で移って Enter で選ぶ
 * (packages/ui の Select のテストと同じ操作)。
 */
async function choose(target: HTMLElement, label: string, option: string): Promise<void> {
  const trigger = labelled<HTMLElement>(target, '[aria-haspopup="listbox"]', label);
  trigger.focus();
  press(trigger, "ArrowDown");
  await vi.waitFor(() => expect(trigger.getAttribute("aria-expanded")).toBe("true"));
  const items = [...document.querySelectorAll('[role="option"]')];
  const wanted = items.find((i) => i.textContent?.trim() === option);
  if (!wanted) throw new Error(`no option: ${option}`);
  await vi.waitFor(() => expect(items.some((i) => i.hasAttribute("data-highlighted"))).toBe(true));
  // 下へ回って目当ての項目まで移る (一覧の端で止まるなら上へ)
  for (const key of ["ArrowDown", "ArrowUp"]) {
    for (let i = 0; i < items.length && !wanted.hasAttribute("data-highlighted"); i++) {
      press(document.activeElement ?? trigger, key);
    }
  }
  press(document.activeElement ?? trigger, "Enter");
  await vi.waitFor(() => expect(trigger.getAttribute("aria-expanded")).toBe("false"));
  // 次に開く Select の項目と混ざらないよう、一覧が body から消えるのを待つ
  await vi.waitFor(() => expect(document.querySelector('[role="option"]')).toBeNull());
}

describe("InferenceSection inference settings", () => {
  it("cannot apply the settings it already has", () => {
    const { target } = setup();
    expect(button(target, "Apply settings").disabled).toBe(true);
  });

  it("applies depth rotation with the committed numbers", async () => {
    const { doc, target } = setup();
    await choose(target, "Method", "Depth rotation");
    // 既定: 拡大 1、中心は点の外接矩形の中心、yaw は格子の軸の AngleX
    expect(field(target, "Pivot x").value).toBe("50");
    expect(button(target, "Apply settings").disabled).toBe(false);
    enter(field(target, "Angle scale"), "2");
    enter(field(target, "Pivot depth"), "-10");
    await click(button(target, "Apply settings"));
    expect(doc.commands).toEqual<Command[]>([
      {
        op: "set_keyform_inference",
        target: QUAD,
        inference: {
          method: "depth_rotation",
          angle_scale: 2,
          pivot: [50, 50],
          pivot_depth: -10,
          yaw: "AngleX",
        },
      },
    ]);
  });

  it("keeps the previous number when a field is emptied", async () => {
    const { doc, target } = setup();
    await choose(target, "Method", "Depth rotation");
    const scale = field(target, "Angle scale");
    enter(scale, "3");
    // 空を持てない欄なので、空や読めない文字は確定前の値に戻る (NaN を送らない)
    enter(scale, "");
    expect(scale.value).toBe("3");
    enter(scale, "abc");
    expect(scale.value).toBe("3");
    expect(target.querySelector(".error")).toBeNull();
    await click(button(target, "Apply settings"));
    const [command] = doc.commands;
    if (command?.op !== "set_keyform_inference") throw new Error("not set_keyform_inference");
    expect(command.inference).toMatchObject({ angle_scale: 3 });
  });

  it("cannot apply depth rotation without a yaw or pitch", async () => {
    const { doc, target } = setup();
    await choose(target, "Method", "Depth rotation");
    await choose(target, "Yaw", "(none)");
    expect(get(target, ".error").textContent?.trim()).toBe("Choose a yaw or pitch parameter");
    expect(button(target, "Apply settings").disabled).toBe(true);
    await choose(target, "Pitch", "Angle X");
    expect(target.querySelector(".error")).toBeNull();
    await click(button(target, "Apply settings"));
    const [command] = doc.commands;
    if (command?.op !== "set_keyform_inference") throw new Error("not set_keyform_inference");
    expect(command.inference).toMatchObject({ pitch: "AngleX" });
    expect(command.inference).not.toHaveProperty("yaw");
  });
});

describe("InferenceSection mirror", () => {
  it("leaves out the axis and tolerance while their fields are empty", async () => {
    const { doc, target } = setup();
    expect(field(target, "Axis").value).toBe("");
    expect(field(target, "Tolerance").value).toBe("");
    await click(button(target, "Mirror"));
    expect(doc.commands).toEqual<Command[]>([
      { op: "mirror_keyforms", source: QUAD, target: QUAD, overwrite_authored: false },
    ]);
  });

  it("sends the committed axis and tolerance, the tolerance clamped to zero", async () => {
    const { doc, target } = setup();
    enter(field(target, "Axis"), "-12.5");
    enter(field(target, "Tolerance"), "-1");
    expect(field(target, "Tolerance").value).toBe("0");
    await click(button(target, "Mirror"));
    // 空にし直すと送らない
    enter(field(target, "Axis"), "");
    await click(button(target, "Mirror"));
    expect(doc.commands).toEqual<Command[]>([
      {
        op: "mirror_keyforms",
        source: QUAD,
        target: QUAD,
        axis: -12.5,
        tolerance: 0,
        overwrite_authored: false,
      },
      {
        op: "mirror_keyforms",
        source: QUAD,
        target: QUAD,
        tolerance: 0,
        overwrite_authored: false,
      },
    ]);
  });

  it("mirrors from the partner first and offers to overwrite authored cells", async () => {
    const { doc, target } = setup({ mirror: "QuadR", axis: null });
    const partner = { kind: "mesh", id: "QuadR" } as const;
    doc.apply.mockImplementationOnce(async (command: Command) => {
      doc.commands.push(command);
      return {
        warnings: [],
        mirrored: [{ source: partner, target: QUAD, written: 0, unedited: 0, kept_authored: 2 }],
      };
    });
    await click(button(target, "Mirror"));
    await click(button(target, "Overwrite authored cells and mirror"));
    expect(doc.commands).toEqual<Command[]>([
      { op: "mirror_keyforms", source: partner, target: QUAD, overwrite_authored: false },
      { op: "mirror_keyforms", source: partner, target: QUAD, overwrite_authored: true },
    ]);
  });
});

describe("InferenceSection mirror partner", () => {
  it("enables Set partner as the axis is typed, before it is committed", async () => {
    const { doc, target } = setup();
    const axis = field(target, "Mirror axis");
    const set = () => button(target, "Set partner");
    expect(axis.value).toBe("");
    expect(set().disabled).toBe(true);
    type(axis, "64");
    expect(set().disabled).toBe(false);
    // 空にし直すと今の値 (軸なし) と同じなので押せない
    type(axis, "");
    expect(set().disabled).toBe(true);
    // 読めない文字の間は前の読めた値のまま
    type(axis, "abc");
    expect(set().disabled).toBe(true);
    type(axis, "64");
    await click(set());
    expect(doc.commands).toEqual<Command[]>([{ op: "set_mirror_pair", target: QUAD, axis: 64 }]);
  });

  it("sends the chosen partner", async () => {
    const { doc, target } = setup();
    await choose(target, "Partner", "QuadR");
    expect(button(target, "Set partner").disabled).toBe(false);
    await click(button(target, "Set partner"));
    expect(doc.commands).toEqual<Command[]>([
      { op: "set_mirror_pair", target: QUAD, mirror: "QuadR" },
    ]);
  });

  it("leaves out the axis when its field is emptied", async () => {
    const { doc, target } = setup({ mirror: "QuadR", axis: 50 });
    const axis = field(target, "Mirror axis");
    expect(axis.value).toBe("50");
    expect(button(target, "Set partner").disabled).toBe(true);
    type(axis, "");
    expect(button(target, "Set partner").disabled).toBe(false);
    await click(button(target, "Set partner"));
    expect(doc.commands).toEqual<Command[]>([
      { op: "set_mirror_pair", target: QUAD, mirror: "QuadR" },
    ]);
  });

  it("unpairs when (none) is chosen", async () => {
    const { doc, target } = setup({ mirror: "QuadR", axis: 50 });
    await choose(target, "Partner", "(none, symmetric)");
    await click(button(target, "Set partner"));
    expect(doc.commands).toEqual<Command[]>([{ op: "set_mirror_pair", target: QUAD, axis: 50 }]);
  });
});
