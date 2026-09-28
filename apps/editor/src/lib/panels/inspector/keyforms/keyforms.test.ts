import { describe, expect, it } from "vitest";
import {
  cellAt,
  cellIndex,
  cellOrigin,
  currentCell,
  elementGrid,
  gridInterpolation,
  isDerived,
  type KeyformBinding,
  type ModelData,
  nearestKey,
  type ParameterData,
  snapToKeys,
  supportsSmooth,
  withPoint,
} from "./keyforms";

const params: ParameterData[] = [
  { id: "AngleX", name: "Angle X", min: -30, max: 30, default: 0 },
  { id: "Open", name: "Open", min: 0, max: 1, default: 1 },
];
const bindings: KeyformBinding[] = [
  { parameter: "AngleX", keys: [-30, 0, 30] },
  { parameter: "Open", keys: [0, 0.1, 1] },
];

describe("nearestKey", () => {
  it("picks the closest key, the smaller one on ties", () => {
    expect(nearestKey([-30, 0, 30], 14)).toBe(0);
    expect(nearestKey([-30, 0, 30], 16)).toBe(30);
    expect(nearestKey([-30, 0, 30], 15)).toBe(0);
    expect(nearestKey([-30, 0, 30], -99)).toBe(-30);
    expect(nearestKey([], 1)).toBeUndefined();
  });
});

describe("currentCell", () => {
  it("returns the cell when every axis is on a key", () => {
    expect(currentCell(bindings, { AngleX: 30, Open: 0 }, params)).toEqual({ AngleX: 30, Open: 0 });
  });

  it("uses defaults for values that are not set", () => {
    expect(currentCell(bindings, { AngleX: -30 }, params)).toEqual({ AngleX: -30, Open: 1 });
  });

  it("accepts values that went through f32", () => {
    // 0.1 を f32 に丸めた値 (wasm の parameter() が返す値)。
    const f32 = Math.fround(0.1);
    expect(f32).not.toBe(0.1);
    expect(currentCell(bindings, { AngleX: 0, Open: f32 }, params)).toEqual({
      AngleX: 0,
      Open: 0.1,
    });
  });

  it("returns null when an axis is between keys", () => {
    expect(currentCell(bindings, { AngleX: 12, Open: 1 }, params)).toBeNull();
  });

  it("returns the only cell of a grid without axes", () => {
    expect(currentCell([], {}, params)).toEqual({});
  });
});

describe("snapToKeys", () => {
  it("moves every axis to its nearest key", () => {
    expect(snapToKeys(bindings, { AngleX: 20, Open: 0.3 }, params)).toEqual({
      AngleX: 30,
      Open: 0.1,
    });
  });
});

describe("cellIndex / cellAt", () => {
  it("uses the axis-0-fastest order", () => {
    expect(cellIndex(bindings, { AngleX: 30, Open: 0.1 })).toBe(2 + 3 * 1);
    expect(cellAt(bindings, 5)).toEqual({ AngleX: 30, Open: 0.1 });
    expect(cellIndex(bindings, { AngleX: 5, Open: 0 })).toBe(-1);
    expect(cellIndex([], {})).toBe(0);
    for (let i = 0; i < 9; i++) expect(cellIndex(bindings, cellAt(bindings, i))).toBe(i);
  });
});

describe("cellOrigin", () => {
  it("treats empty or unknown origins as authored", () => {
    expect(cellOrigin({ keyforms: [{}] }, 0)).toBe("authored");
    expect(cellOrigin({ keyforms: [{}, {}], origins: ["authored", "derived"] }, 1)).toBe("derived");
    expect(cellOrigin({ keyforms: [{}], origins: ["future"] }, 0)).toBe("authored");
  });

  it("keeps the recorded inference method", () => {
    const grid = { keyforms: [{}, {}, {}], origins: ["additive", "mirror", "authored"] };
    expect(cellOrigin(grid, 0)).toBe("additive");
    expect(isDerived(cellOrigin(grid, 1))).toBe(true);
    expect(isDerived(cellOrigin(grid, 2))).toBe(false);
  });
});

describe("elementGrid", () => {
  const model: ModelData = {
    parameters: params,
    parts: [{ id: "Root", keyforms: { keyforms: [{ opacity: 1 }] } }],
    deformers: [
      {
        id: "Face",
        kind: { type: "warp", rows: 1, cols: 1, keyforms: { keyforms: [{ points: [] }] } },
      },
      { id: "Neck", kind: { type: "rotation", keyforms: { keyforms: [{}] } } },
      { id: "Future", kind: { type: "spline" } },
    ],
    meshes: [{ id: "Eye", indices: [0, 1, 2], keyforms: { keyforms: [] } }],
  };

  it("resolves the grid and its keyform kind", () => {
    expect(elementGrid(model, { kind: "deformer", id: "Face" })?.kind).toBe("warp");
    expect(elementGrid(model, { kind: "deformer", id: "Neck" })?.kind).toBe("rotation");
    expect(elementGrid(model, { kind: "mesh", id: "Eye" })?.target).toEqual({
      kind: "mesh",
      id: "Eye",
    });
    expect(elementGrid(model, { kind: "part", id: "Root" })?.kind).toBe("part");
    expect(elementGrid(model, { kind: "deformer", id: "Future" })).toBeNull();
    expect(elementGrid(model, { kind: "parameter", id: "AngleX" })).toBeNull();
    expect(elementGrid(model, null)).toBeNull();
  });
});

describe("withPoint", () => {
  it("replaces one point without touching the original", () => {
    const keyform = {
      positions: [[0, 0] as [number, number], [1, 1] as [number, number]],
      opacity: 1,
    };
    const next = withPoint("mesh", keyform, 1, [5, 6]);
    expect(next).toEqual({
      positions: [
        [0, 0],
        [5, 6],
      ],
      opacity: 1,
    });
    expect(keyform.positions[1]).toEqual([1, 1]);
    expect(withPoint("rotation", { origin: [0, 0], angle: 3 }, 0, [2, 2])).toEqual({
      origin: [2, 2],
      angle: 3,
    });
  });
});

describe("gridInterpolation / supportsSmooth", () => {
  it("reads linear by default and flags unknown values", () => {
    expect(gridInterpolation({ keyforms: [] })).toBe("linear");
    expect(gridInterpolation({ keyforms: [], interpolation: "smooth" })).toBe("smooth");
    expect(gridInterpolation({ keyforms: [], interpolation: "bezier" })).toBe("unknown");
  });

  it("offers smooth only for grids with shapes", () => {
    expect(supportsSmooth("mesh")).toBe(true);
    expect(supportsSmooth("warp")).toBe(true);
    expect(supportsSmooth("rotation")).toBe(true);
    expect(supportsSmooth("part")).toBe(false);
  });
});
