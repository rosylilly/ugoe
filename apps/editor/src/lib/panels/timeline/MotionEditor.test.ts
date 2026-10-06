// @vitest-environment happy-dom
// ドープシート: キーのドラッグを、前の適用を待つ間に続けて行っても、送る直前の (動かした後の) キーを動かすこと
// (docs/editor.md#パネルからの編集、docs/testing.md#エディタの部品のテスト)。
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Command, Curve, CurveTarget, Motion } from "../../ipc";
import {
  deferred,
  drag,
  FakeController,
  FakeDocument,
  render,
  settle,
} from "../../viewport/tools/test/harness.svelte";
import MotionEditor from "./MotionEditor.svelte";
import { findCurve } from "./timeline";
import { TimelineState } from "./timeline.svelte";

vi.mock("../../viewport/wasm", () => ({ loadUgoeWasm: async () => null }));

afterEach(() => {
  vi.restoreAllMocks();
});

const TARGET: CurveTarget = { kind: "parameter", id: "AngleX" };

function motion(): Motion {
  return {
    name: "Nod",
    duration: 2,
    looping: false,
    fade_in: 0,
    fade_out: 0,
    curves: [
      {
        target: TARGET,
        segments: [
          { type: "start", p: { t: 0, v: 0 } },
          { type: "linear", to: { t: 1, v: 10 } },
          { type: "linear", to: { t: 2, v: 0 } },
        ],
        fade_in: null,
        fade_out: null,
      },
    ],
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
  // happy-dom は幅を測れない (0) ので、レーン (目盛りの行) の幅を 600 にする。
  vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockReturnValue(600);
  const doc = new FakeDocument();
  doc.motions = [motion()];
  doc.onApply = (command) => {
    if (command.op === "remove_motion_key") {
      const current = doc.motions[0];
      const c = current ? findCurve(current, TARGET) : null;
      const index = c
        ? c.segments.findIndex((s) => s.type !== "start" && s.to.t === command.t)
        : -1;
      if (!current || !c || index < 0) return false;
      doc.motions = [
        { ...current, curves: [{ ...c, segments: c.segments.filter((_, i) => i !== index) }] },
      ];
      return true;
    }
    if (command.op !== "move_motion_key") return true;
    const current = doc.motions[0];
    const c = current ? findCurve(current, TARGET) : null;
    const next = c ? moveKey(c, command) : null;
    if (!current || !next) return false;
    doc.motions = [{ ...current, curves: [next] }];
    return true;
  };
  const controller = new FakeController();
  const timeline = new TimelineState(doc.asDocument, controller.asController);
  timeline.motionName = "Nod";
  const { target } = render(MotionEditor, {
    doc: doc.asDocument,
    viewport: controller.asController,
    timeline,
  });
  return { doc, timeline, target };
}

/** 時刻 1 のキーの印 (行の 2 番目のひし形)。 */
function diamond(target: HTMLElement): Element {
  const el = target.querySelectorAll("button.diamond")[1];
  if (!el) throw new Error("no key");
  return el;
}

/** 印を横に `dt` 秒ずらす距離 (CSS px)。レーンの幅 600 に、両端に長さの 2% の余白を付けた 2.08 秒を並べる。 */
function dx(dt: number): number {
  return Math.round((dt * 600) / 2.08);
}

async function settleAll(): Promise<void> {
  for (let i = 0; i < 10; i++) await settle();
}

/** パネルで Delete を押す (選んだキーの削除)。 */
function pressDelete(target: HTMLElement): void {
  const panel = target.querySelector(".motion-editor");
  if (!panel) throw new Error("no panel");
  panel.dispatchEvent(new KeyboardEvent("keydown", { key: "Delete", bubbles: true }));
}

describe("MotionEditor dope sheet drags while sending", () => {
  it("moves the key again from where the previous drag put it", async () => {
    const { doc, timeline, target } = setup();
    const gate = deferred();
    doc.gate = gate.promise;
    drag(diamond(target), [
      [100, 10],
      [100 + dx(0.5), 10],
    ]);
    await settle();
    // 表示は読み直していない (時刻 1 のまま) ので、同じ印をもう一度つかむ。
    drag(diamond(target), [
      [100, 10],
      [100 + dx(0.6), 10],
    ]);
    gate.resolve();
    await settleAll();
    expect(doc.commands).toEqual([
      { op: "move_motion_key", motion: "Nod", target: TARGET, t: 1, to: { t: 1.5, v: 10 } },
      { op: "move_motion_key", motion: "Nod", target: TARGET, t: 1.5, to: { t: 1.6, v: 10 } },
    ]);
    expect(timeline.selectedKey?.t).toBe(1.6);
  });

  it("drags the key in the curve graph after a pending dope sheet drag", async () => {
    const { doc, timeline, target } = setup();
    const gate = deferred();
    doc.gate = gate.promise;
    // ドープシートの印を押すとその行を選び、下にカーブのグラフを出す。
    drag(diamond(target), [
      [100, 10],
      [100 + dx(0.5), 10],
    ]);
    await settle();
    // グラフの時刻 1 のキー (値 10 は一番上)。幅 600 に -0.04..2.04 秒、上の余白 8。
    const hit = target.querySelectorAll(".graph rect.hit.grab")[1];
    if (!hit) throw new Error("no graph key");
    const x = (t: number) => ((t + 0.04) * 600) / 2.08;
    drag(hit, [
      [x(1), 8],
      [x(1.6), 8],
    ]);
    gate.resolve();
    await settleAll();
    expect(doc.commands).toEqual([
      { op: "move_motion_key", motion: "Nod", target: TARGET, t: 1, to: { t: 1.5, v: 10 } },
      { op: "move_motion_key", motion: "Nod", target: TARGET, t: 1.5, to: { t: 1.6, v: 10 } },
    ]);
    expect(timeline.selectedKey?.t).toBe(1.6);
  });
});

describe("MotionEditor key removal while sending", () => {
  it("removes the key moved by a pending drag", async () => {
    const { doc, timeline, target } = setup();
    const gate = deferred();
    doc.gate = gate.promise;
    drag(diamond(target), [
      [100, 10],
      [100 + dx(0.5), 10],
    ]);
    await settle();
    // 選択はまだ時刻 1 を指している。
    pressDelete(target);
    gate.resolve();
    await settleAll();
    expect(doc.commands).toEqual([
      { op: "move_motion_key", motion: "Nod", target: TARGET, t: 1, to: { t: 1.5, v: 10 } },
      { op: "remove_motion_key", motion: "Nod", target: TARGET, t: 1.5 },
    ]);
    expect(timeline.selectedKey).toBeNull();
  });

  it("does not send the removal when the key is gone", async () => {
    const { doc, target } = setup();
    const gate = deferred();
    doc.gate = gate.promise;
    drag(diamond(target), [
      [100, 10],
      [100 + dx(0.5), 10],
    ]);
    await settle();
    // 同じキーを 2 度消す (1 度目の削除を送っている間に、もう一度押す)。
    pressDelete(target);
    await settle();
    pressDelete(target);
    gate.resolve();
    await settleAll();
    expect(doc.commands).toEqual([
      { op: "move_motion_key", motion: "Nod", target: TARGET, t: 1, to: { t: 1.5, v: 10 } },
      { op: "remove_motion_key", motion: "Nod", target: TARGET, t: 1.5 },
    ]);
  });
});
