import { describe, expect, it } from "vitest";
import {
  angleBetween,
  flatten,
  gridEdges,
  gridOutline,
  movePoints,
  rotationFrame,
  rotationHandles,
  rotationSense,
  scaleFactor,
  snapAngle,
  toPoints,
  transformFrame,
} from "./gizmo";

describe("points", () => {
  it("converts between flat arrays and points", () => {
    expect(toPoints([1, 2, 3, 4, 5])).toEqual([
      [1, 2],
      [3, 4],
    ]);
    expect(
      flatten([
        [1, 2],
        [3, 4],
      ]),
    ).toEqual([1, 2, 3, 4]);
  });

  it("moves only the selected points", () => {
    const points: [number, number][] = [
      [0, 0],
      [1, 0],
      [2, 0],
    ];
    expect(movePoints(points, [0, 2], [1, 5])).toEqual([
      [1, 5],
      [1, 0],
      [3, 5],
    ]);
    expect(points[0]).toEqual([0, 0]);
  });
});

describe("gridEdges", () => {
  it("connects neighbours of a 2x1 grid", () => {
    // 0 1 2
    // 3 4 5
    expect(gridEdges(2, 1)).toEqual([
      [0, 1],
      [0, 3],
      [1, 2],
      [1, 4],
      [2, 5],
      [3, 4],
      [4, 5],
    ]);
  });

  it("walks around the outline once", () => {
    // 0 1 2
    // 3 4 5
    // 6 7 8
    expect(gridOutline(2, 2)).toEqual([0, 1, 2, 5, 8, 7, 6, 3]);
    expect(gridOutline(1, 1)).toEqual([0, 1, 3, 2]);
  });

  it("has (cols+1)*rows + cols*(rows+1) edges", () => {
    expect(gridEdges(3, 4)).toHaveLength(4 * 4 + 3 * 5);
  });
});

describe("rotation frame", () => {
  it("reads WebPuppet.rotation_frame and rejects bad input", () => {
    expect(rotationFrame([1, 2, 3, 4, 5, 6])).toEqual({ origin: [1, 2], x: [3, 4], y: [5, 6] });
    expect(rotationFrame([1, 2, 3])).toBeNull();
    expect(rotationFrame([1, 2, 3, 4, 5, Number.NaN])).toBeNull();
    expect(rotationFrame(undefined)).toBeNull();
  });

  it("maps to screen with the view transform", () => {
    const f = transformFrame(
      { origin: [10, 20], x: [0.5, 0], y: [0, 0.5] },
      (p) => [p[0] * 2 + 1, p[1] * 2],
      2,
    );
    expect(f).toEqual({ origin: [21, 40], x: [1, 0], y: [0, 1] });
  });

  it("places handles up and right of the origin regardless of scale", () => {
    const h = rotationHandles({ origin: [100, 100], x: [0.5, 0], y: [0, 0.5] }, 40);
    expect(h.origin).toEqual([100, 100]);
    expect(h.rotate).toEqual([100, 60]);
    expect(h.scale).toEqual([140, 100]);
  });

  it("follows the rotation (clockwise 90 degrees)", () => {
    // R(90): x → (0, 1), y → (-1, 0)。上 (-y) は右 (1, 0) を向く。
    const h = rotationHandles({ origin: [0, 0], x: [0, 1], y: [-1, 0] }, 10);
    expect(h.rotate[0]).toBeCloseTo(10);
    expect(h.rotate[1]).toBeCloseTo(0);
    expect(h.scale[0]).toBeCloseTo(0);
    expect(h.scale[1]).toBeCloseTo(10);
  });
});

describe("angles and scale", () => {
  it("measures clockwise angles on a y-down screen", () => {
    expect(angleBetween([0, 0], [1, 0], [0, 1])).toBeCloseTo(90);
    expect(angleBetween([0, 0], [0, -1], [-1, 0])).toBeCloseTo(-90);
    expect(angleBetween([0, 0], [-1, 0.01], [-1, -0.01])).toBeCloseTo(1.146, 2);
  });

  it("flips the sense when the frame is mirrored", () => {
    const plain = {
      origin: [0, 0] as [number, number],
      x: [1, 0] as [number, number],
      y: [0, 1] as [number, number],
    };
    const mirrored = { ...plain, x: [-1, 0] as [number, number] };
    expect(rotationSense(plain, false, false)).toBe(1);
    // 自分の反転は angle の向きを変えない (R(angle) は反転の後に掛かる)。
    expect(rotationSense(mirrored, true, false)).toBe(1);
    // 親が反転していると、angle を増やすと画面上では反時計回りに回る。
    expect(rotationSense(mirrored, false, false)).toBe(-1);
    expect(rotationSense(plain, true, true)).toBe(1);
  });

  it("measures the scale ratio from the origin", () => {
    expect(scaleFactor([0, 0], [10, 0], [0, 20])).toBeCloseTo(2);
    expect(scaleFactor([5, 5], [5, 5], [6, 6])).toBeNull();
  });

  it("snaps angles", () => {
    expect(snapAngle(22, 15)).toBe(15);
    expect(snapAngle(-23, 15)).toBe(-30);
  });
});
