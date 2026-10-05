// @vitest-environment happy-dom
// カーブのグラフ: キーの時刻と値の欄を、前の適用を待つ間に続けて確定しても、送る直前の (動かした後の) キーを動かすこと
// (docs/editor.md#パネルからの編集、docs/testing.md#エディタの部品のテスト)。
import { flushSync } from "svelte";
import { describe, expect, it, vi } from "vitest";
import type { Command, Curve, CurveTarget, Motion } from "../../ipc";
import {
  deferred,
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
      { type: "linear", to: { t: 1, v: 10 } },
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
    return s.type === "start" ? { ...s, p: command.to } : { ...s, to: command.to };
  });
  return { ...c, segments };
}

function setup() {
  const doc = new FakeDocument();
  doc.motions = [motion(curve())];
  doc.onApply = (command) => {
    if (command.op !== "move_motion_key") return true;
    const current = doc.motions[0];
    const c = current ? findCurve(current, TARGET) : null;
    const next = c ? moveKey(c, command) : null;
    if (!current || !next) return false;
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
