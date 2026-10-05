// @vitest-environment happy-dom
// カーブのグラフ: キーの時刻と値の欄の確定と、キー・取っ手のドラッグを、前の適用を待つ間に続けて行っても、送る直前の
// (動かした後の) キーを動かすこと (docs/editor.md#パネルからの編集、docs/testing.md#エディタの部品のテスト)。
import { flushSync } from "svelte";
import { describe, expect, it, vi } from "vitest";
import type { Command, Curve, CurveTarget, Motion } from "../../ipc";
import {
  deferred,
  drag,
  FakeController,
  FakeDocument,
  get,
  render,
  settle,
} from "../../viewport/tools/test/harness.svelte";
import CurveGraph from "./CurveGraph.svelte";
import { findCurve } from "./timeline";
import { TimelineState } from "./timeline.svelte";

vi.mock("../../viewport/wasm", () => ({ loadUgoeWasm: async () => null }));

const TARGET: CurveTarget = { kind: "parameter", id: "AngleX" };

function curve(): Curve {
  return {
    target: TARGET,
    segments: [
      { type: "start", p: { t: 0, v: 0 } },
      { type: "bezier", c1: { t: 0.3, v: 3 }, c2: { t: 0.7, v: 8 }, to: { t: 1, v: 10 } },
      { type: "linear", to: { t: 2, v: 0 } },
    ],
    fade_in: null,
    fade_out: null,
  };
}

function motion(c: Curve): Motion {
  return {
    name: "Nod",
    duration: 2,
    looping: false,
    fade_in: 0,
    fade_out: 0,
    curves: [c],
    events: [],
  };
}

/** `move_motion_key` を当てる (無いキーを動かそうとしたら失敗。本物の NotFound の代わり)。 */
function moveKey(c: Curve, command: Extract<Command, { op: "move_motion_key" }>): Curve | null {
  const index = c.segments.findIndex((s) => (s.type === "start" ? s.p.t : s.to.t) === command.t);
  if (index < 0) return null;
  const segments = c.segments.map((s, i) => {
    if (i !== index) return s;
    if (s.type === "start") return { ...s, p: command.to };
    // 取っ手は区間の中に収める (本物の move_motion_key と同じく、検証を通る形にする)。
    if (s.type === "bezier") {
      const t1 = command.to.t;
      return {
        ...s,
        c2: { t: Math.min(s.c2.t, t1), v: s.c2.v },
        to: command.to,
      };
    }
    return { ...s, to: command.to };
  });
  return { ...c, segments };
}

function setup() {
  const doc = new FakeDocument();
  doc.motions = [motion(curve())];
  doc.onApply = (command) => {
    const current = doc.motions[0];
    const c = current ? findCurve(current, TARGET) : null;
    if (!current || !c) return false;
    if (command.op === "set_motion_curve_fade") {
      doc.motions = [
        {
          ...current,
          curves: [{ ...c, fade_in: command.fade_in ?? null, fade_out: command.fade_out ?? null }],
        },
      ];
      return true;
    }
    if (command.op !== "move_motion_key") return true;
    const next = moveKey(c, command);
    if (!next) return false;
    doc.motions = [{ ...current, curves: [next] }];
    return true;
  };
  const timeline = new TimelineState(doc.asDocument, new FakeController().asController);
  timeline.motionName = "Nod";
  timeline.selectKey(TARGET, 1);
  const first = doc.motions[0] as Motion;
  const { target } = render(CurveGraph, {
    doc: doc.asDocument,
    timeline,
    motion: first,
    row: { target: TARGET, label: "AngleX", curve: findCurve(first, TARGET), parameter: null },
    view: { t0: 0, t1: 2, width: 200 },
    onmessage: () => {},
  });
  return { doc, timeline, target };
}

/** aria-labelledby が `suffix` で終わる数値の欄。 */
function field(target: HTMLElement, suffix: string): HTMLInputElement {
  return get<HTMLInputElement>(target, `input[aria-labelledby$="${suffix}"]`);
}

/** 欄に文字を入れて Enter で確定する。 */
function enter(input: HTMLInputElement, text: string): void {
  input.focus();
  input.value = text;
  input.dispatchEvent(new Event("input", { bubbles: true }));
  flushSync();
  input.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
  flushSync();
}

async function settleAll(): Promise<void> {
  for (let i = 0; i < 10; i++) await settle();
}

describe("CurveGraph key fields while sending", () => {
  it("moves the key from where the previous commit put it", async () => {
    const { doc, timeline, target } = setup();
    const gate = deferred();
    doc.pending = gate.promise;
    enter(field(target, "-t"), "1.5");
    await settle();
    enter(field(target, "-t"), "1.6");
    gate.resolve();
    await settleAll();
    expect(doc.commands).toEqual([
      { op: "move_motion_key", motion: "Nod", target: TARGET, t: 1, to: { t: 1.5, v: 10 } },
      { op: "move_motion_key", motion: "Nod", target: TARGET, t: 1.5, to: { t: 1.6, v: 10 } },
    ]);
    expect(timeline.selectedKey?.t).toBe(1.6);
  });

  it("sets the value of the key moved by a pending time commit", async () => {
    const { doc, timeline, target } = setup();
    const gate = deferred();
    doc.pending = gate.promise;
    enter(field(target, "-t"), "1.5");
    await settle();
    enter(field(target, "-v"), "4");
    gate.resolve();
    await settleAll();
    expect(doc.commands).toEqual([
      { op: "move_motion_key", motion: "Nod", target: TARGET, t: 1, to: { t: 1.5, v: 10 } },
      { op: "move_motion_key", motion: "Nod", target: TARGET, t: 1.5, to: { t: 1.5, v: 4 } },
    ]);
    expect(timeline.selectedKey?.t).toBe(1.5);
  });
});

/**
 * グラフの点 (時刻, 値) の位置 (happy-dom の getBoundingClientRect は 0 なので clientX / clientY)。
 * 幅 200 で 0..2 秒、高さ 94 (上下に 8 の余白) で値は 0..10 (カーブの値の範囲)。
 */
function at(t: number, v: number): [number, number] {
  return [t * 100, 8 + 94 - (v / 10) * 94];
}

/** 時刻 1 のキー (2 番目の印) の押せる範囲。 */
function keyHit(target: HTMLElement): Element {
  const hit = target.querySelectorAll("rect.hit.grab")[1];
  if (!hit) throw new Error("no key");
  return hit;
}

/** 時刻 1 のキーで終わるベジェの区間の取っ手 (c2) の押せる範囲。 */
function handleHit(target: HTMLElement): Element {
  const hit = target.querySelectorAll("rect.hit.move")[1];
  if (!hit) throw new Error("no handle");
  return hit;
}

describe("CurveGraph drags while sending", () => {
  it("moves the key again from where the previous drag put it", async () => {
    const { doc, timeline, target } = setup();
    const gate = deferred();
    doc.pending = gate.promise;
    // 表示は読み直していない (時刻 1 のまま) ので、同じ印をもう一度つかむ。
    drag(keyHit(target), [at(1, 10), at(1.5, 10)]);
    await settle();
    drag(keyHit(target), [at(1, 10), at(1.6, 10)]);
    gate.resolve();
    await settleAll();
    expect(doc.commands).toEqual([
      { op: "move_motion_key", motion: "Nod", target: TARGET, t: 1, to: { t: 1.5, v: 10 } },
      { op: "move_motion_key", motion: "Nod", target: TARGET, t: 1.5, to: { t: 1.6, v: 10 } },
    ]);
    expect(timeline.selectedKey?.t).toBe(1.6);
  });

  it("moves the handle of the key moved by a pending drag", async () => {
    const { doc, target } = setup();
    const gate = deferred();
    doc.pending = gate.promise;
    drag(keyHit(target), [at(1, 10), at(1.5, 10)]);
    await settle();
    drag(handleHit(target), [at(0.7, 8), at(0.9, 5)]);
    gate.resolve();
    await settleAll();
    expect(doc.commands).toEqual([
      { op: "move_motion_key", motion: "Nod", target: TARGET, t: 1, to: { t: 1.5, v: 10 } },
      {
        op: "set_motion_segment",
        motion: "Nod",
        target: TARGET,
        t: 1.5,
        segment: { type: "bezier", c2: { t: 0.9, v: 5 } },
      },
    ]);
  });

  it("drags the key moved by a pending field commit", async () => {
    const { doc, target } = setup();
    const gate = deferred();
    doc.pending = gate.promise;
    enter(field(target, "-t"), "1.5");
    await settle();
    drag(keyHit(target), [at(1, 10), at(1.6, 10)]);
    gate.resolve();
    await settleAll();
    expect(doc.commands).toEqual([
      { op: "move_motion_key", motion: "Nod", target: TARGET, t: 1, to: { t: 1.5, v: 10 } },
      { op: "move_motion_key", motion: "Nod", target: TARGET, t: 1.5, to: { t: 1.6, v: 10 } },
    ]);
  });
});

/** 区間の種類のボタン (直線・ベジェ・ステップ・逆ステップの順)。 */
function kindButton(target: HTMLElement, index: number): HTMLButtonElement {
  const button = target.querySelectorAll<HTMLButtonElement>(".kinds button")[index];
  if (!button) throw new Error("no kind button");
  return button;
}

/** カーブのフェードの欄 (0 = フェードイン、1 = フェードアウト) に値を入れて確定する。 */
function setFade(target: HTMLElement, index: number, text: string): void {
  const input = target.querySelectorAll<HTMLInputElement>('.fields input[type="number"]')[index];
  if (!input) throw new Error("no fade field");
  input.value = text;
  input.dispatchEvent(new Event("change", { bubbles: true }));
  flushSync();
}

describe("CurveGraph segment and fade edits while sending", () => {
  it("sets the kind of the segment ending at the key moved by a pending drag", async () => {
    const { doc, target } = setup();
    const gate = deferred();
    doc.pending = gate.promise;
    drag(keyHit(target), [at(1, 10), at(1.5, 10)]);
    await settle();
    // 表示 (選択) はまだ時刻 1 を指している。
    kindButton(target, 2).click();
    gate.resolve();
    await settleAll();
    expect(doc.commands).toEqual([
      { op: "move_motion_key", motion: "Nod", target: TARGET, t: 1, to: { t: 1.5, v: 10 } },
      {
        op: "set_motion_segment",
        motion: "Nod",
        target: TARGET,
        t: 1.5,
        segment: { type: "stepped" },
      },
    ]);
  });

  it("does not send the kind when the key is gone", async () => {
    const { doc, target } = setup();
    const gate = deferred();
    doc.pending = gate.promise;
    // 先に並べた編集 (ドープシートの削除など) が、選んだキーを消す。
    const remove: Command = { op: "remove_motion_key", motion: "Nod", target: TARGET, t: 1 };
    doc.onApply = (command) => {
      if (command.op !== "remove_motion_key") return true;
      const segments = curve().segments.filter((s) => s.type !== "bezier");
      doc.motions = [motion({ ...curve(), segments })];
      return true;
    };
    void doc.send(() => remove);
    kindButton(target, 2).click();
    gate.resolve();
    await settleAll();
    expect(doc.commands).toEqual([remove]);
  });

  it("keeps the other fade committed while sending", async () => {
    const { doc, target } = setup();
    const gate = deferred();
    doc.pending = gate.promise;
    setFade(target, 0, "0.5");
    await settle();
    // 表示のカーブはまだフェードインが無い。
    setFade(target, 1, "0.3");
    gate.resolve();
    await settleAll();
    expect(doc.commands).toEqual([
      { op: "set_motion_curve_fade", motion: "Nod", target: TARGET, fade_in: 0.5, fade_out: null },
      { op: "set_motion_curve_fade", motion: "Nod", target: TARGET, fade_in: 0.5, fade_out: 0.3 },
    ]);
  });
});
