// @vitest-environment happy-dom
// 物理の表示: 衝突体のドラッグから送るコマンド (docs/testing.md#エディタの部品のテスト)。
import { describe, expect, it } from "vitest";
import type { PhysicsCollider, PhysicsSettings } from "../../../ipc";
import { PhysicsView } from "../../../panels/physics/physicsView.svelte";
import type { Point } from "../geometry";
import {
  drag,
  FakeController,
  FakeDocument,
  get,
  pointer,
  render,
  screenOf,
  settle,
} from "../test/harness.svelte";
import PhysicsOverlay from "./PhysicsOverlay.svelte";

const HEAD: PhysicsCollider = {
  id: "Head",
  name: "Head",
  attach: null,
  shape: { type: "circle", center: [50, 50], radius: 10 },
};
/** 回転デフォーマ Rot (偽の WebPuppet では (20, 10) の平行移動) に取り付けたカプセル。 */
const ARM: PhysicsCollider = {
  id: "Arm",
  name: "Arm",
  attach: "Rot",
  shape: { type: "capsule", a: [0, 0], b: [0, 40], radius: 5 },
};

function setup() {
  const controller = new FakeController();
  const doc = new FakeDocument();
  doc.physics = {
    gravity: [0, 1],
    wind: [0, 0],
    fps: 60,
    rigs: [],
    colliders: [HEAD, ARM],
  } as PhysicsSettings;
  const view = new PhysicsView();
  view.active = true;
  const { target } = render(PhysicsOverlay, {
    controller: controller.asController,
    doc: doc.asDocument,
    view,
  });
  return { doc, view, target };
}

const shift = (p: Point, dx: number, dy: number): Point => [p[0] + dx, p[1] + dy];

/** 送ったコマンドの衝突体の形状。 */
function sentShape(doc: FakeDocument) {
  expect(doc.commands).toHaveLength(1);
  const command = doc.commands[0];
  if (command?.op !== "set_physics_collider") throw new Error(`unexpected ${command?.op}`);
  return command.collider.shape;
}

describe("PhysicsOverlay colliders", () => {
  it("moves a circle with one set_physics_collider on release", async () => {
    const { doc, view, target } = setup();
    const body = get(target, ".collider circle.body");
    const start = screenOf([50, 50]);
    drag(body, [start, shift(start, 10, 2), shift(start, 20, 6)]);
    await settle();
    expect(view.collider).toBe("Head");
    const shape = sentShape(doc);
    expect(shape.type).toBe("circle");
    if (shape.type !== "circle") return;
    expect(shape.center[0]).toBeCloseTo(60, 3);
    expect(shape.center[1]).toBeCloseTo(53, 3);
    expect(shape.radius).toBe(10);
  });

  it("moves one end of a capsule in the local space of its deformer", async () => {
    const { doc, target } = setup();
    const end = get(target, ".collider circle.end:nth-of-type(2)");
    // 端 B は局所座標 (0, 40)、キャンバス座標 (20, 50)
    const start = screenOf([20, 50]);
    expect([Number(end.getAttribute("cx")), Number(end.getAttribute("cy"))]).toEqual(start);
    drag(end, [start, shift(start, 0, 20)]);
    await settle();
    const shape = sentShape(doc);
    expect(shape.type).toBe("capsule");
    if (shape.type !== "capsule") return;
    expect(shape.a).toEqual([0, 0]);
    expect(shape.b[0]).toBeCloseTo(0, 3);
    expect(shape.b[1]).toBeCloseTo(50, 3);
  });

  it("previews the drag and sends nothing when it is cancelled", () => {
    const { doc, view, target } = setup();
    const body = () => get(target, ".collider circle.body");
    const start = screenOf([50, 50]);
    pointer(body(), "pointerdown", start);
    pointer(body(), "pointermove", shift(start, 30, 0));
    expect(Number(body().getAttribute("cx"))).toBe(start[0] + 30);
    pointer(body(), "pointercancel", shift(start, 30, 0));
    expect(doc.apply).not.toHaveBeenCalled();
    expect(Number(body().getAttribute("cx"))).toBe(start[0]);
    // 押した衝突体は選んだまま
    expect(view.collider).toBe("Head");
  });

  it("sends nothing for a press without a move", () => {
    const { doc, target } = setup();
    const body = get(target, ".collider circle.body");
    drag(body, [screenOf([50, 50])]);
    expect(doc.apply).not.toHaveBeenCalled();
  });

  it("leaves Alt + drag to the viewport", () => {
    const { doc, view, target } = setup();
    const body = get(target, ".collider circle.body");
    const start = screenOf([50, 50]);
    drag(body, [start, shift(start, 20, 0)], { altKey: true });
    expect(view.collider).toBeNull();
    expect(doc.apply).not.toHaveBeenCalled();
  });
});
