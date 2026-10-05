// @vitest-environment happy-dom
// スロットのパネル: 送っている間の確定を捨てず、送る直前の (読み直した後の) スロットから組み立てること
// (docs/editor.md#パネルからの編集、docs/testing.md#エディタの部品のテスト)。
import { flushSync } from "svelte";
import { describe, expect, it } from "vitest";
import type { Command, Slot } from "../../ipc";
import { Selection } from "../../selection.svelte";
import {
  deferred,
  FakeDocument,
  get,
  render,
  settle,
  testModel,
} from "../../viewport/tools/test/harness.svelte";
import SlotsPanel from "./SlotsPanel.svelte";
import { SlotsView } from "./slotsView.svelte";

function slot(id: string, min: number, max: number): Slot {
  return {
    id,
    anchor: null,
    draw_order: { min, max },
    replaces: [],
    parameters: [],
    mask_sources: [],
    part: null,
    colliders: [],
    pose_parts: [],
  };
}

/** テストのモデルにスロットを足した JSON。 */
function modelWith(slots: Slot[]): string {
  return JSON.stringify({ ...JSON.parse(testModel()), pose: { groups: [] }, slots });
}

function setup() {
  const doc = new FakeDocument();
  let slots = [slot("Hat", 100, 200)];
  doc.model = { json: modelWith(slots), bundle: null };
  // 本物の読み直しの代わりに、送ったスロットのコマンドを写しに当てる。
  doc.onApply = (command: Command) => {
    if (command.op === "set_slot") {
      if (!slots.some((s) => s.id === command.id)) return false;
      slots = slots.map((s) => (s.id === command.id ? command.slot : s));
    } else if (command.op === "remove_slot") {
      slots = slots.filter((s) => s.id !== command.id);
    } else if (command.op === "add_slot") {
      slots = [...slots, command.slot];
    }
    doc.model = { json: modelWith(slots), bundle: null };
    return true;
  };
  const view = new SlotsView();
  view.slot = "Hat";
  const { target } = render(SlotsPanel, {
    doc: doc.asDocument,
    selection: new Selection(),
    view,
  });
  return { doc, view, target };
}

/** aria-labelledby が `suffix` で終わる欄。 */
function field(target: HTMLElement, suffix: string): HTMLInputElement {
  return get<HTMLInputElement>(target, `input[aria-labelledby$="${suffix}"]`);
}

/** 欄に文字を入れて Enter で確定する (数値の欄)。 */
function enter(input: HTMLInputElement, text: string): void {
  input.focus();
  input.value = text;
  input.dispatchEvent(new Event("input", { bubbles: true }));
  flushSync();
  input.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
  flushSync();
}

/** 名前の欄を書き換えて確定する。 */
function rename(target: HTMLElement, id: string): void {
  const input = get<HTMLInputElement>(target, 'input[id$="-name"]');
  input.value = id;
  input.dispatchEvent(new Event("change", { bubbles: true }));
  flushSync();
}

async function settleAll(): Promise<void> {
  for (let i = 0; i < 10; i++) await settle();
}

describe("SlotsPanel edits while sending", () => {
  it("keeps a range commit made while the previous one is applied", async () => {
    const { doc, target } = setup();
    const gate = deferred();
    doc.pending = gate.promise;
    enter(field(target, "-from"), "120");
    await settle();
    enter(field(target, "-to"), "180");
    gate.resolve();
    await settleAll();
    expect(doc.commands).toEqual([
      { op: "set_slot", id: "Hat", slot: slot("Hat", 120, 200) },
      // 前の適用を読み直した後のスロットから組み立てる (下端の 120 を戻さない)。
      { op: "set_slot", id: "Hat", slot: slot("Hat", 120, 180) },
    ]);
  });

  it("sends edits after a rename to the new ID", async () => {
    const { doc, view, target } = setup();
    const gate = deferred();
    doc.pending = gate.promise;
    rename(target, "Cap");
    await settle();
    enter(field(target, "-to"), "180");
    gate.resolve();
    await settleAll();
    expect(doc.commands).toEqual([
      { op: "set_slot", id: "Hat", slot: slot("Cap", 100, 200) },
      { op: "set_slot", id: "Cap", slot: slot("Cap", 100, 180) },
    ]);
    expect(view.slot).toBe("Cap");
  });

  it("does not disable the fields while sending", async () => {
    const { doc, target } = setup();
    const gate = deferred();
    doc.pending = gate.promise;
    enter(field(target, "-from"), "120");
    await settle();
    expect(field(target, "-to").disabled).toBe(false);
    expect(get<HTMLInputElement>(target, 'input[id$="-name"]').disabled).toBe(false);
    gate.resolve();
    await settleAll();
  });
});
