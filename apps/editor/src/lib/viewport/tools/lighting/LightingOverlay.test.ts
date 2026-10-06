// @vitest-environment happy-dom
// 光源の取っ手: ドラッグ中は表示だけを変え、離したときに 1 回だけプロジェクトに残すこと
// (docs/editor.md#ライティングの画面、docs/testing.md#エディタの部品のテスト)。
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { PreviewLight, PreviewLighting } from "../../../ipc";
import { LightingView } from "../../../panels/lighting/lightingView.svelte";
import type { Point } from "../geometry";
import {
  all,
  drag,
  FakeController,
  FakeDocument,
  pointer,
  render,
  screenOf,
  settle,
  testModel,
} from "../test/harness.svelte";
import LightingOverlay from "./LightingOverlay.svelte";

// 光源はモデルに入らないので、コマンドではなく ipc.setPreviewLighting で残す。
const ipc = vi.hoisted(() => ({
  setPreviewLighting: vi.fn(async (_lighting: unknown) => ({})),
  getPreviewLighting: vi.fn(async () => {
    throw new Error("not used");
  }),
}));
vi.mock("../../../ipc", () => ({ ipc }));

beforeEach(() => {
  ipc.setPreviewLighting.mockClear();
});

const SUN: PreviewLight = {
  type: "directional",
  direction: [0, 0, -1],
  color: [1, 1, 1],
  intensity: 0.7,
};
const LAMP: PreviewLight = {
  type: "point",
  position: [20, 30, 40],
  color: [1, 1, 1],
  intensity: 1,
  range: null,
};

/**
 * キャンバス 100×100 のモデルで、正面から当てる平行光源と点光源を置いて描く。
 * 平行光源の円は中心 (110, 120) (= キャンバスの中央 (50, 50))・半径 70 (= 100 × 2 × 0.35)。
 */
function setup() {
  const controller = new FakeController();
  const doc = new FakeDocument();
  doc.model = {
    json: JSON.stringify({
      ...JSON.parse(testModel()),
      meta: { canvas: { width: 100, height: 100, origin: [0, 0] } },
    }),
    bundle: null,
  };
  const view = new LightingView();
  view.active = true;
  view.settings = {
    enabled: true,
    ambient: [1, 1, 1],
    ambient_intensity: 0.45,
    lights: [SUN, LAMP],
  };
  const { target } = render(LightingOverlay, {
    controller: controller.asController,
    doc: doc.asDocument,
    view,
  });
  // 押すのは見える印に重ねた透明な円 (押せる範囲)
  const handles = () => all<SVGCircleElement>(target, "circle.hit");
  const marks = () => all<SVGCircleElement>(target, "circle.handle");
  return { doc, view, handles, marks };
}

const DIAL_CENTER: Point = [110, 120];
const DIAL_RADIUS = 70;

function centerOf(el: Element): Point {
  return [Number(el.getAttribute("cx")), Number(el.getAttribute("cy"))];
}

/** 残した設定 (1 回だけ送ったこと)。 */
function saved(): PreviewLighting {
  expect(ipc.setPreviewLighting).toHaveBeenCalledTimes(1);
  return ipc.setPreviewLighting.mock.calls[0]?.[0] as PreviewLighting;
}

describe("LightingOverlay", () => {
  it("draws the directional handle on the dial and the point handle on the canvas", () => {
    const { handles } = setup();
    const [sun, lamp] = handles();
    if (!sun || !lamp) throw new Error("no handles");
    // 正面から当てる光は円の中心
    expect(centerOf(sun)).toEqual(DIAL_CENTER);
    expect(centerOf(lamp)).toEqual(screenOf([20, 30]));
  });

  it("gives each handle a 24px target around the smaller mark", () => {
    const { handles, marks } = setup();
    expect(handles().map((h) => h.getAttribute("r"))).toEqual(["12", "12"]);
    expect(handles().map((h) => h.getAttribute("role"))).toEqual(["button", "button"]);
    expect(marks().map((m) => m.getAttribute("r"))).toEqual(["9", "9"]);
    expect(marks().map(centerOf)).toEqual(handles().map(centerOf));
  });

  it("previews a directional drag and saves once on release", async () => {
    const { view, handles } = setup();
    const sun = () => handles()[0] as SVGCircleElement;
    const side: Point = [DIAL_CENTER[0] + DIAL_RADIUS, DIAL_CENTER[1]];
    pointer(sun(), "pointerdown", DIAL_CENTER);
    pointer(sun(), "pointermove", [DIAL_CENTER[0] + 30, DIAL_CENTER[1]]);
    pointer(sun(), "pointermove", side);
    // ドラッグ中は表示だけが変わり、何も残さない
    expect(ipc.setPreviewLighting).not.toHaveBeenCalled();
    expect(centerOf(sun())).toEqual(side);
    expect(view.settings.lights[0]).toMatchObject({ direction: [-1, 0, 0] });
    pointer(sun(), "pointerup", side);
    await settle();
    const lights = saved().lights;
    expect(lights[0]).toEqual({ ...SUN, direction: [-1, 0, 0] });
    expect(lights[1]).toEqual(LAMP);
    expect(view.selected).toBe(0);
  });

  it("moves a point light in canvas coordinates and keeps its depth", async () => {
    const { view, handles } = setup();
    const lamp = handles()[1] as SVGCircleElement;
    const start = screenOf([20, 30]);
    drag(lamp, [start, screenOf([40, 35]), screenOf([60, 70])]);
    await settle();
    expect(saved().lights[1]).toEqual({ ...LAMP, position: [60, 70, 40] });
    expect(view.selected).toBe(1);
  });

  it("restores the light and saves nothing when the drag is cancelled", () => {
    const { view, handles } = setup();
    const lamp = () => handles()[1] as SVGCircleElement;
    const start = screenOf([20, 30]);
    pointer(lamp(), "pointerdown", start);
    pointer(lamp(), "pointermove", screenOf([60, 70]));
    expect(centerOf(lamp())).toEqual(screenOf([60, 70]));
    pointer(lamp(), "pointercancel", screenOf([60, 70]));
    expect(ipc.setPreviewLighting).not.toHaveBeenCalled();
    expect(centerOf(lamp())).toEqual(start);
    expect(view.settings.lights[1]).toEqual(LAMP);
  });

  it("saves nothing for a press without a move", async () => {
    const { view, handles } = setup();
    drag(handles()[1] as SVGCircleElement, [screenOf([20, 30])]);
    await settle();
    expect(ipc.setPreviewLighting).not.toHaveBeenCalled();
    expect(view.selected).toBe(1);
  });

  it("ignores buttons other than the primary one", async () => {
    const { handles } = setup();
    const start = screenOf([20, 30]);
    drag(handles()[1] as SVGCircleElement, [start, screenOf([60, 70])], { button: 2 });
    await settle();
    expect(ipc.setPreviewLighting).not.toHaveBeenCalled();
  });
});
