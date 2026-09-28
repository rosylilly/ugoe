import { describe, expect, it } from "vitest";
import type { Model } from "../../../generated/types";
import {
  type DepthElement,
  depthColor,
  depthElement,
  depthRange,
  depthScale,
  describeDepth,
  finiteValue,
  pointsCommand,
  presetCommand,
  setDepthCommand,
} from "./depth";

// 深度に関わるフィールドだけを持つモデル (型検査のためにキャストする)。
const model = {
  meshes: [
    {
      id: "Nose",
      uvs: [
        [0, 0],
        [1, 0],
        [0, 1],
      ],
      depth: [1, 2, 3],
    },
    {
      id: "Flat",
      uvs: [
        [0, 0],
        [1, 0],
        [0, 1],
        [1, 1],
      ],
    },
  ],
  deformers: [
    { id: "Face", kind: { type: "warp", rows: 2, cols: 1, depth: null } },
    { id: "Head", kind: { type: "rotation" } },
  ],
} as unknown as Model;

const nose: DepthElement = {
  target: { kind: "mesh", id: "Nose" },
  kind: "mesh",
  points: 3,
  depth: [1, 2, 3],
};

describe("depthElement", () => {
  it("reads meshes and warps", () => {
    expect(depthElement(model, { kind: "mesh", id: "Nose" })).toEqual(nose);
    expect(depthElement(model, { kind: "mesh", id: "Flat" })?.depth).toBeNull();
    expect(depthElement(model, { kind: "deformer", id: "Face" })).toEqual({
      target: { kind: "deformer", id: "Face" },
      kind: "warp",
      points: 6,
      depth: null,
    });
  });

  it("ignores other elements", () => {
    expect(depthElement(model, { kind: "deformer", id: "Head" })).toBeNull();
    expect(depthElement(model, { kind: "part", id: "Root" })).toBeNull();
    expect(depthElement(model, { kind: "mesh", id: "Missing" })).toBeNull();
    expect(depthElement(model, null)).toBeNull();
  });
});

describe("depth summary", () => {
  it("describes the range", () => {
    expect(depthRange([3, -1, 2])).toEqual({ min: -1, max: 3 });
    expect(depthRange(null)).toBeNull();
    expect(describeDepth(null)).toBe("none");
    expect(describeDepth([4, 4])).toBe("4 px");
    expect(describeDepth([0, 12.345])).toBe("0 – 12.35 px");
  });
});

describe("commands", () => {
  it("builds set_depth and presets", () => {
    const target = nose.target;
    expect(setDepthCommand(target, [1, 2])).toEqual({ op: "set_depth", target, depth: [1, 2] });
    expect(setDepthCommand(target, null)).toEqual({ op: "set_depth", target });
    expect(presetCommand(target, { kind: "dome", peak: 20 })).toEqual({
      op: "apply_depth_preset",
      target,
      preset: { kind: "dome", peak: 20 },
    });
  });

  it("sends only valid, unique point indices", () => {
    expect(pointsCommand(nose, [2, 0, 2, 7, -1, 1.5], 5)).toEqual({
      op: "apply_depth_preset",
      target: nose.target,
      preset: { kind: "points", indices: [0, 2], value: 5 },
    });
    expect(pointsCommand(nose, [9], 5)).toBeNull();
    expect(pointsCommand(nose, [], 5)).toBeNull();
  });

  it("accepts finite input values only", () => {
    expect(finiteValue(12.5)).toBe(12.5);
    expect(finiteValue(-3)).toBe(-3);
    expect(finiteValue(null)).toBeNull();
    expect(finiteValue(undefined)).toBeNull();
    expect(finiteValue(Number.NaN)).toBeNull();
    expect(finiteValue(Number.POSITIVE_INFINITY)).toBeNull();
  });
});

describe("depth colors", () => {
  it("scales by the largest magnitude", () => {
    expect(depthScale([1, -4, 2])).toBe(4);
    expect(depthScale([0, 0])).toBe(1);
    expect(depthScale([])).toBe(1);
  });

  it("is gray at zero, red in front and blue behind", () => {
    expect(depthColor(0, 10)).toBe("rgb(160, 160, 160)");
    expect(depthColor(10, 10)).toBe("rgb(235, 64, 52)");
    expect(depthColor(-10, 10)).toBe("rgb(52, 110, 235)");
    // 尺度を超える値は飽和する。
    expect(depthColor(50, 10)).toBe(depthColor(10, 10));
    expect(depthColor(5, 0)).toBe("rgb(160, 160, 160)");
  });
});
