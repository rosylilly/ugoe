import { describe, expect, it } from "vitest";
import type { Glue, ModuleStatus } from "../../ipc";
import {
  createGlueCommand,
  distanceValue,
  glueableMeshes,
  glueWeight,
  isBaseGlue,
  isDrawCloneGlue,
  keyformLabels,
  selectedPair,
  setGlueCommand,
  swapped,
  uniformWeight,
  withIntensity,
  withWeight,
} from "./glues";

const glue = (fields: Partial<Glue> = {}): Glue => ({
  id: "Seam",
  mesh_a: "Left",
  mesh_b: "Right",
  pairs: [
    { a: 1, b: 0, weight: 0.5 },
    { a: 2, b: 3, weight: 0.5 },
  ],
  keyforms: { bindings: [], keyforms: [{ intensity: 1 }] },
  ...fields,
});

const module = { base_meshes: ["Face", "Body"] } as unknown as ModuleStatus;

describe("glue editing (T0659)", () => {
  it("treats glues of base meshes as read-only in module projects", () => {
    expect(isBaseGlue(glue(), null)).toBe(false);
    expect(isBaseGlue(glue(), module)).toBe(false);
    expect(isBaseGlue(glue({ mesh_a: "Face" }), module)).toBe(true);
    expect(glueableMeshes([{ id: "Face" }, { id: "Hat/Brim" }], module)).toEqual([
      { id: "Hat/Brim" },
    ]);
    expect(glueableMeshes([{ id: "Face" }], null)).toEqual([{ id: "Face" }]);
  });

  it("recognizes the glues of draw clones", () => {
    expect(isDrawCloneGlue({ id: "DrawClone_Face_Clone" })).toBe(true);
    expect(isDrawCloneGlue({ id: "Hat/DrawClone_Brim_Clone" })).toBe(true);
    expect(isDrawCloneGlue({ id: "Seam" })).toBe(false);
  });

  it("sets the weight of every pair and the intensity of a keyform", () => {
    expect(glueWeight(glue())).toBe(0.5);
    expect(glueWeight(glue({ pairs: [] }))).toBe(0.5);
    const mixed = glue({
      pairs: [
        { a: 0, b: 0, weight: 0.2 },
        { a: 1, b: 1, weight: 0.8 },
      ],
    });
    expect(uniformWeight(mixed)).toBe(false);
    const set = withWeight(mixed, 0.3);
    expect(set.pairs.map((p) => p.weight)).toEqual([0.3, 0.3]);
    expect(uniformWeight(set)).toBe(true);
    expect(withIntensity(glue(), 0, 0.25).keyforms.keyforms).toEqual([{ intensity: 0.25 }]);
  });

  it("swaps the meshes without changing the look", () => {
    const s = swapped(glue({ pairs: [{ a: 1, b: 0, weight: 0.25 }] }));
    expect([s.mesh_a, s.mesh_b]).toEqual(["Right", "Left"]);
    expect(s.pairs).toEqual([{ a: 0, b: 1, weight: 0.75 }]);
  });

  it("labels keyforms by their cells", () => {
    expect(keyformLabels(glue())).toEqual(["Intensity"]);
    const keyed = glue({
      keyforms: {
        bindings: [{ parameter: "Glue", keys: [0, 1] }],
        keyforms: [{ intensity: 0 }, { intensity: 1 }],
      },
    });
    expect(keyformLabels(keyed)).toEqual(["Glue = 0", "Glue = 1"]);
  });

  it("builds the commands and reads the inputs", () => {
    expect(createGlueCommand("Left", "Right", 8)).toEqual({
      op: "create_glue",
      mesh_a: "Left",
      mesh_b: "Right",
      distance: 8,
    });
    expect(setGlueCommand(glue())).toEqual({ op: "set_glue", glue: glue() });
    expect(setGlueCommand(glue(), 4)).toEqual({ op: "set_glue", glue: glue(), distance: 4 });
    expect(selectedPair(["Left", "Face", "Right"], ["Left", "Right"])).toEqual(["Left", "Right"]);
    expect(selectedPair(["Left"], ["Left", "Right"])).toBeNull();
    expect(distanceValue("8")).toBe(8);
    expect(distanceValue("")).toBeNull();
    expect(distanceValue("-1")).toBeNull();
  });
});
