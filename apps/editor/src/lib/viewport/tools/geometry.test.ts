import { describe, expect, it } from "vitest";
import {
  boundaryEdges,
  cycleHit,
  meshEdges,
  nearestVertex,
  updateVertexSelection,
  verticesInBox,
} from "./geometry";

// (0,0) (10,0) (10,10) (0,10) の四角形と、中心 (5,5)
const square = [0, 0, 10, 0, 10, 10, 0, 10, 5, 5];
// 中心で 4 枚に分けた三角形
const fan = [0, 1, 4, 1, 2, 4, 2, 3, 4, 3, 0, 4];

describe("nearestVertex", () => {
  it("picks the closest vertex within the radius", () => {
    expect(nearestVertex(square, 9, 1, 3)).toBe(1);
    expect(nearestVertex(square, 6, 6, 3)).toBe(4);
    expect(nearestVertex(square, 2.5, 2.5, 3)).toBe(null);
  });

  it("prefers the lower index on ties", () => {
    expect(nearestVertex([0, 0, 2, 0], 1, 0, 5)).toBe(0);
  });
});

describe("verticesInBox", () => {
  it("includes vertices on the edges and ignores the drag direction", () => {
    expect(verticesInBox(square, { x0: -1, y0: -1, x1: 5, y1: 5 })).toEqual([0, 4]);
    expect(verticesInBox(square, { x0: 10, y0: 10, x1: 0, y1: 4 })).toEqual([2, 3, 4]);
    expect(verticesInBox(square, { x0: 1, y0: 1, x1: 2, y1: 2 })).toEqual([]);
  });
});

describe("edges", () => {
  it("lists every edge once", () => {
    expect(meshEdges(fan)).toHaveLength(8);
    expect(meshEdges([0, 1, 2, 0, 2, 3])).toEqual([
      [0, 1],
      [1, 2],
      [0, 2],
      [2, 3],
      [0, 3],
    ]);
  });

  it("finds the outline", () => {
    expect(boundaryEdges(fan)).toEqual([
      [0, 1],
      [1, 2],
      [2, 3],
      [3, 0],
    ]);
  });
});

describe("cycleHit", () => {
  it("walks from front to back and wraps around", () => {
    const hits = ["Front", "Middle", "Back"];
    expect(cycleHit(hits, null)).toBe("Front");
    expect(cycleHit(hits, "Front")).toBe("Middle");
    expect(cycleHit(hits, "Back")).toBe("Front");
    expect(cycleHit(hits, "Elsewhere")).toBe("Front");
    expect(cycleHit([], "Front")).toBe(null);
  });
});

describe("updateVertexSelection", () => {
  it("replaces, adds and toggles", () => {
    expect(updateVertexSelection([1, 2], [4, 0], false)).toEqual([0, 4]);
    expect(updateVertexSelection([1, 2], [4, 0], true)).toEqual([0, 1, 2, 4]);
    expect(updateVertexSelection([1, 2], [2], true)).toEqual([1]);
    expect(updateVertexSelection([1, 2], [3], true)).toEqual([1, 2, 3]);
  });
});
