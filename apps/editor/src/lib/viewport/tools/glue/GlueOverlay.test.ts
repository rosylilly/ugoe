// @vitest-environment happy-dom
// グルーの頂点の組の表示: 組を編集するモードを抜けるキー操作と、送っている間の組の追加・削除
// (docs/testing.md#エディタの部品のテスト)。
import { flushSync } from "svelte";
import { describe, expect, it } from "vitest";
import type { GluePair } from "../../../ipc";
import { GlueView } from "../../../panels/glues/glueView.svelte";
import { ViewportScene } from "../scene.svelte";
import {
  all,
  deferred,
  dialogButton,
  FakeController,
  FakeDocument,
  pointer,
  press,
  render,
  settle,
  testModel,
} from "../test/harness.svelte";
import GlueOverlay from "./GlueOverlay.svelte";

/** テストのモデルのグルー Seam の組を `pairs` にした JSON。 */
function modelWithPairs(pairs: GluePair[]): string {
  const model = JSON.parse(testModel());
  model.glues[0].pairs = pairs;
  return JSON.stringify(model);
}

function setup(pairs?: GluePair[]) {
  const controller = new FakeController();
  const doc = new FakeDocument();
  if (pairs) doc.model = { json: modelWithPairs(pairs), bundle: null };
  // 本物の読み直しの代わりに、送ったグルーを写しに当てる。
  doc.onApply = (command) => {
    if (command.op === "set_glue") {
      doc.model = { json: modelWithPairs(command.glue.pairs), bundle: null };
    }
    return true;
  };
  const view = new GlueView();
  view.active = true;
  view.glue = "Seam";
  const scene = new ViewportScene(controller.asController, doc.asDocument);
  const { target } = render(GlueOverlay, { doc: doc.asDocument, scene, view });
  view.setPairMode("add");
  flushSync();
  return { view, doc, target };
}

/** 送った `set_glue` の組。 */
function sentPairs(doc: FakeDocument): GluePair[][] {
  return doc.commands.map((c) => (c.op === "set_glue" ? c.glue.pairs : []));
}

async function settleAll(): Promise<void> {
  for (let i = 0; i < 10; i++) await settle();
}

/** `side` の端の `index` 番目の頂点を押す。 */
function vertex(target: HTMLElement, side: "a" | "b", index: number): void {
  const el = all(target, `circle.vertex.${side}`)[index];
  if (!el) throw new Error(`no vertex ${side}${index}`);
  pointer(el, "pointerdown", [0, 0]);
}

describe("GlueOverlay keys", () => {
  it("leaves the pair mode with Esc", () => {
    const { view } = setup();
    press(document.body, "Escape");
    expect(view.pairMode).toBe("off");
  });

  it("does not take Esc pressed inside a dialog", () => {
    const { view } = setup();
    press(dialogButton(), "Escape");
    expect(view.pairMode).toBe("add");
  });
});

describe("GlueOverlay pairs while sending", () => {
  it("adds a pair made while the previous one is applied to the latest glue", async () => {
    const { doc, target } = setup();
    const gate = deferred();
    doc.gate = gate.promise;
    vertex(target, "a", 2);
    vertex(target, "b", 3);
    await settle();
    vertex(target, "a", 3);
    vertex(target, "b", 2);
    gate.resolve();
    await settleAll();
    expect(sentPairs(doc)).toEqual([
      [
        { a: 1, b: 0, weight: 1 },
        { a: 2, b: 3, weight: 1 },
      ],
      // 前の組を読み直した後のグルーに足す (前の組を戻さない)。
      [
        { a: 1, b: 0, weight: 1 },
        { a: 2, b: 3, weight: 1 },
        { a: 3, b: 2, weight: 1 },
      ],
    ]);
  });

  it("removes the pair that was clicked even after an earlier removal", async () => {
    const { view, doc, target } = setup([
      { a: 1, b: 0, weight: 1 },
      { a: 2, b: 3, weight: 1 },
    ]);
    view.setPairMode("remove");
    await settle();
    const gate = deferred();
    doc.gate = gate.promise;
    const markers = all(target, "rect.marker");
    expect(markers).toHaveLength(2);
    pointer(markers[0] as Element, "pointerdown", [0, 0]);
    await settle();
    // 送っている間は前の組の印が残っている。2 つ目の印は 2 番目の組 (2, 3)。
    pointer(markers[1] as Element, "pointerdown", [0, 0]);
    gate.resolve();
    await settleAll();
    expect(sentPairs(doc)).toEqual([[{ a: 2, b: 3, weight: 1 }], []]);
  });
});

describe("GlueOverlay highlighted pair (T1651)", () => {
  it("draws only the pair the panel points at, thicker and on top", () => {
    const { view, target } = setup([
      { a: 1, b: 0, weight: 1 },
      { a: 2, b: 3, weight: 1 },
    ]);
    view.setPairMode("off");
    flushSync();
    expect(all(target, "g.highlight")).toHaveLength(0);
    view.hoveredPair = { a: 2, b: 3 };
    flushSync();
    const groups = all(target, "g.highlight");
    expect(groups).toHaveLength(1);
    expect(groups[0]?.getAttribute("data-pair")).toBe("1");
    // 線 (縁取りと本体) と両端の 2 つの点。ほかの組の線はそのまま。
    expect(all(target, "g.highlight line")).toHaveLength(2);
    expect(all(target, "g.highlight circle")).toHaveLength(2);
    expect(all(target, "line.pair")).toHaveLength(3);
    view.hoveredPair = null;
    flushSync();
    expect(all(target, "g.highlight")).toHaveLength(0);
  });

  it("prefers the hovered pair over the focused one and ignores pairs the glue lacks", () => {
    const { view, target } = setup([
      { a: 1, b: 0, weight: 1 },
      { a: 2, b: 3, weight: 1 },
    ]);
    view.focusedPair = { a: 1, b: 0 };
    view.hoveredPair = { a: 2, b: 3 };
    flushSync();
    expect(all(target, "g.highlight")[0]?.getAttribute("data-pair")).toBe("1");
    view.hoveredPair = null;
    flushSync();
    expect(all(target, "g.highlight")[0]?.getAttribute("data-pair")).toBe("0");
    view.focusedPair = { a: 3, b: 3 };
    flushSync();
    expect(all(target, "g.highlight")).toHaveLength(0);
  });
});
