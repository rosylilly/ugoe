import { describe, expect, it } from "vitest";
import type { Parameter } from "../../ipc";
import { applyExpression, changedParameters, expressionFromValues, valueFor } from "./expressions";

function param(id: string, min: number, def: number, max: number): Parameter {
  return {
    id,
    name: id,
    min,
    default: def,
    max,
    kind: "normal",
    mirror: null,
    group: null,
  } as Parameter;
}

const params = [
  param("EyeLOpen", 0, 1, 1),
  param("MouthForm", -1, 0, 1),
  param("AngleX", -30, 0, 30),
];

describe("expressions", () => {
  it("computes values that reproduce the current sliders", () => {
    expect(valueFor("add", 0.4, 1)).toBeCloseTo(-0.6);
    expect(valueFor("multiply", 0.4, 1)).toBeCloseTo(0.4);
    expect(valueFor("multiply", 0.4, 0)).toBeNull();
    expect(valueFor("overwrite", 0.4, 1)).toBe(0.4);
  });

  it("finds the parameters moved away from their defaults", () => {
    expect(changedParameters(params, { EyeLOpen: 0.4, MouthForm: 0, AngleX: 0.001 })).toEqual([
      "EyeLOpen",
    ]);
  });

  it("creates an expression and falls back to add when multiply cannot", () => {
    const values = { EyeLOpen: 0.4, MouthForm: 0.8, AngleX: 0 };
    const e = expressionFromValues("Smile", params, values, ["EyeLOpen", "MouthForm"], "multiply");
    expect(e.parameters).toEqual([
      { parameter: "EyeLOpen", op: "multiply", value: 0.4 },
      { parameter: "MouthForm", op: "add", value: 0.8 },
    ]);
    // 既定値に重ねると今の値になる。
    const applied = applyExpression(e, { EyeLOpen: 1, MouthForm: 0, AngleX: 0 });
    expect(applied.EyeLOpen).toBeCloseTo(0.4);
    expect(applied.MouthForm).toBeCloseTo(0.8);
  });

  it("applies ops in order and ignores unknown parameters", () => {
    const applied = applyExpression(
      {
        name: "e",
        fade_in: 0,
        fade_out: 0,
        parameters: [
          { parameter: "AngleX", op: "add", value: 5 },
          { parameter: "EyeLOpen", op: "overwrite", value: 0 },
          { parameter: "Missing", op: "add", value: 1 },
        ],
      },
      { AngleX: 10, EyeLOpen: 1 },
    );
    expect(applied).toEqual({ AngleX: 15, EyeLOpen: 0 });
  });
});
