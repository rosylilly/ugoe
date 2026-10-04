import { describe, expect, it } from "vitest";
import type { PhysicsCollider, PhysicsRig } from "../../../generated/types";
import type { Point } from "../geometry";
import {
  anchorOf,
  canvasToLocal,
  capsulePath,
  changeShapeKind,
  dragShape,
  type Jacobian,
  type MapPoint,
  reattach,
  restParticles,
  worldShape,
} from "./physicsGeometry";

// "Head": 原点 (100, 50) で 90 度回した拡大率 2 のアフィン。"Warp": (x, y) → (x + x·y, y) の双線形。
const map: MapPoint = (d, [x, y]) => {
  if (d === null) return [x, y];
  if (d === "Head") return [100 - 2 * y, 50 + 2 * x];
  if (d === "Warp") return [x + x * y, y];
  return null;
};
const jacobian: Jacobian = (d, [x, y]) => {
  if (d === null) return [1, 0, 0, 1];
  if (d === "Head") return [0, 2, -2, 0];
  if (d === "Warp") return [1 + y, 0, x, 1];
  return null;
};
const guess = (): Point => [0, 0];

const close = (a: Point | null | undefined, b: Point) => {
  expect(a).not.toBeNull();
  expect(a?.[0]).toBeCloseTo(b[0], 3);
  expect(a?.[1]).toBeCloseTo(b[1], 3);
};

const rig = (outputs: PhysicsRig["outputs"], frame?: string): PhysicsRig => ({
  id: "R",
  name: "R",
  inputs: [],
  outputs,
  particles: [
    { position: [1, 0], mobility: 1, delay: 1, acceleration: 1, radius: 0, collision_radius: 0 },
    { position: [1, 10], mobility: 1, delay: 1, acceleration: 1, radius: 10, collision_radius: 0 },
  ],
  normalization: {
    position: { min: -1, default: 0, max: 1 },
    angle: { min: -1, default: 0, max: 1 },
  },
  colliders: [],
  ...(frame ? { frame } : {}),
});

const output = (deformer: string) => ({
  target: { type: "deformer_angle" as const, deformer },
  particle: 1,
  source: "angle" as const,
  scale: 1,
  weight: 100,
  reflect: false,
});

describe("particles", () => {
  it("anchors the root to the first deformer output, then the frame", () => {
    expect(anchorOf(rig([output("Head")], "Warp"))).toBe("Head");
    expect(anchorOf(rig([], "Warp"))).toBe("Warp");
    expect(anchorOf(rig([]))).toBeNull();
  });

  it("places the rest chain at the anchor", () => {
    // 根元 (1,0) は Head で (100, 52) に写る。粒子 1 は根元から (0, 10)。
    const points = restParticles(rig([output("Head")]), map);
    expect(points).toEqual([
      [100, 52],
      [100, 62],
    ]);
    expect(restParticles(rig([output("Nope")]), map)).toBeNull();
  });
});

describe("colliders", () => {
  const circle: PhysicsCollider = {
    id: "C",
    name: "C",
    attach: "Head",
    shape: { type: "circle", center: [5, 0], radius: 8 },
  };
  const capsule: PhysicsCollider = {
    id: "K",
    name: "K",
    attach: "Warp",
    shape: { type: "capsule", a: [0, 0], b: [1, 1], radius: 3 },
  };

  it("maps shapes to the canvas without scaling the radius", () => {
    expect(worldShape(circle, map)).toEqual({ type: "circle", center: [100, 60], radius: 8 });
    expect(worldShape(capsule, map)).toEqual({ type: "capsule", a: [0, 0], b: [2, 1], radius: 3 });
    expect(worldShape({ ...circle, attach: "Nope" }, map)).toBeNull();
  });

  it("inverts affine and bilinear maps", () => {
    close(canvasToLocal(map, jacobian, "Head", [90, 60], [0, 0]), [5, 5]);
    close(canvasToLocal(map, jacobian, "Warp", [3, 0.5], [0, 0]), [2, 0.5]);
    close(canvasToLocal(map, jacobian, null, [3, 4], [0, 0]), [3, 4]);
    expect(canvasToLocal(map, jacobian, "Nope", [0, 0], [0, 0])).toBeNull();
  });

  it("drags whole shapes or capsule ends in canvas space", () => {
    // Head の局所座標で (5,0) → キャンバス (100,60)。右へ 4 動かすと局所の y が -2。
    const moved = dragShape(circle, "body", [4, 0], map, jacobian);
    expect(moved?.type).toBe("circle");
    if (moved?.type === "circle") close(moved.center, [5, -2]);
    const end = dragShape(capsule, "b", [0, 1], map, jacobian);
    if (end?.type !== "capsule") throw new Error("capsule expected");
    expect(end.a).toEqual([0, 0]);
    // b はキャンバス (2,1) → (2,2)。局所 (x, 2) で x·3 = 2。
    close(end.b, [2 / 3, 2]);
  });

  it("reattaches keeping the canvas position", () => {
    const detached = reattach(circle, null, map, jacobian, guess);
    expect(detached?.attach).toBeUndefined();
    expect(detached?.shape).toEqual({ type: "circle", center: [100, 60], radius: 8 });
    const back = reattach(detached as PhysicsCollider, "Head", map, jacobian, guess);
    expect(back?.attach).toBe("Head");
    if (back?.shape.type === "circle") close(back.shape.center, [5, 0]);
  });

  it("switches between circles and capsules in canvas space", () => {
    const shape = changeShapeKind({ ...circle, attach: null }, "capsule", map, jacobian, guess);
    expect(shape).toEqual({ type: "capsule", a: [-3, 0], b: [13, 0], radius: 8 });
    const round = changeShapeKind(capsule, "circle", map, jacobian, guess);
    if (round?.type !== "circle") throw new Error("circle expected");
    // キャンバスの中点 (1, 0.5) を局所座標に戻す: x · 1.5 = 1。
    close(round.center, [2 / 3, 0.5]);
    expect(changeShapeKind(circle, "circle", map, jacobian, guess)).toBe(circle.shape);
  });
});

describe("drawing", () => {
  it("outlines capsules", () => {
    expect(capsulePath([0, 0], [10, 0], 2)).toBe("M0,2L10,2A2,2 0 0,0 10,-2L0,-2A2,2 0 0,0 0,2Z");
    expect(capsulePath([0, 0], [0, 0], 2)).toContain("a2,2");
  });
});
