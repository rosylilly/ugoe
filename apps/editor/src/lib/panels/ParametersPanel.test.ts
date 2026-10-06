// @vitest-environment happy-dom
// パラメータのスライダーの下のキーの印: 押せる範囲 (24px) のどこを押してもそのキーへ移り、隣の印と範囲が重なる所では
// 近い方のキーへ移る (docs/ui.md#そのほか、docs/testing.md#エディタの部品のテスト。T1634)。
import { flushSync } from "svelte";
import { describe, expect, it } from "vitest";
import { Selection } from "../selection.svelte";
import {
  all,
  FakeController,
  FakeDocument,
  render,
  testModel,
} from "../viewport/tools/test/harness.svelte";
import ParametersPanel from "./ParametersPanel.svelte";

/** 印の帯の幅 (CSS px)。値域 -30〜30 のキー -30・0・30 の印の中心は 8・108・208。 */
const WIDTH = 216;

function setup(keys: number[] = [-30, 0, 30]) {
  const doc = new FakeDocument();
  const model = JSON.parse(testModel());
  model.meshes[0].keyforms = {
    bindings: [{ parameter: "AngleX", keys }],
    keyforms: keys.map(() => ({ positions: [] })),
  };
  doc.model = { json: JSON.stringify(model), bundle: null };
  const selection = new Selection();
  selection.set([{ kind: "mesh", id: "Quad" }]);
  const viewport = new FakeController();
  const parameters = [
    {
      id: "AngleX",
      name: "Angle X",
      min: -30,
      max: 30,
      default: 0,
      kind: "normal" as const,
      mirror: null,
      group: null,
    },
  ];
  const { target } = render(ParametersPanel, {
    doc: doc.asDocument,
    selection,
    viewport: viewport.asController,
    parameters,
  });
  // happy-dom はレイアウトをしないので、帯の大きさを与える。
  const strip = target.querySelector<HTMLElement>(".markers");
  if (!strip) throw new Error("no marker strip");
  strip.getBoundingClientRect = () => new DOMRect(0, 0, WIDTH, 24);
  const markers = () => all<HTMLButtonElement>(target, "button.marker");
  return { viewport, markers };
}

/** 帯の左端から `x` の所でポインタで押す。 */
function clickAt(marker: HTMLButtonElement, x: number): void {
  marker.dispatchEvent(new MouseEvent("click", { bubbles: true, clientX: x, detail: 1 }));
  flushSync();
}

describe("ParametersPanel key markers", () => {
  it("draws one marker per key of the selected element", () => {
    const { markers } = setup();
    expect(markers().map((m) => m.getAttribute("aria-label"))).toEqual([
      "Go to key -30",
      "Go to key 0",
      "Go to key 30",
    ]);
  });

  it("moves to the key from either edge of the 24px target", () => {
    const { viewport, markers } = setup();
    const [, zero, last] = markers();
    if (!zero || !last) throw new Error("no markers");
    clickAt(zero, 108 - 11);
    expect(viewport.parameterValues.AngleX).toBe(0);
    clickAt(last, 208 + 11);
    expect(viewport.parameterValues.AngleX).toBe(30);
    clickAt(zero, 108 + 11);
    expect(viewport.parameterValues.AngleX).toBe(0);
  });

  it("picks the nearer key where two targets overlap", () => {
    // キー 0 と 3 の印の中心は 108 と 118 (押せる範囲が 14px 重なる)。後の印が上に描かれる。
    const { viewport, markers } = setup([0, 3]);
    const [zero, three] = markers();
    if (!zero || !three) throw new Error("no markers");
    clickAt(three, 110);
    expect(viewport.parameterValues.AngleX).toBe(0);
    clickAt(zero, 116);
    expect(viewport.parameterValues.AngleX).toBe(3);
  });

  it("moves to the marker's own key when pressed from the keyboard", () => {
    const { viewport, markers } = setup([0, 3]);
    const three = markers()[1];
    if (!three) throw new Error("no marker");
    // Enter・Space の click は detail が 0 で、位置を持たない。
    three.dispatchEvent(new MouseEvent("click", { bubbles: true, detail: 0 }));
    flushSync();
    expect(viewport.parameterValues.AngleX).toBe(3);
  });
});
