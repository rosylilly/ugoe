import { describe, expect, it } from "vitest";
import type { Glue, ModuleStatus } from "../../ipc";
import {
  addGlueKeyCommand,
  createGlueCommand,
  distanceValue,
  glueableMeshes,
  glueWeight,
  isBaseGlue,
  isDrawCloneGlue,
  keyformLabels,
  keyValue,
  removeGlueKeyCommand,
  selectedPair,
  setGlueCommand,
  swapped,
  unboundParameters,
  uniformWeight,
  withIntensity,
  withoutPair,
  withPair,
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

describe("glue pairs and intensity axes (T0661)", () => {
  it("adds a pair with the glue's weight and keeps one pair per B vertex", () => {
    const g = glue({ pairs: [{ a: 1, b: 0, weight: 0.25 }] });
    expect(withPair(g, 4, 2)?.pairs).toEqual([
      { a: 1, b: 0, weight: 0.25 },
      { a: 4, b: 2, weight: 0.25 },
    ]);
    // 同じ B の頂点の組は置き換える。
    expect(withPair(g, 3, 0)?.pairs).toEqual([{ a: 3, b: 0, weight: 0.25 }]);
    // 同じ組は足さない。
    expect(withPair(g, 1, 0)).toBeNull();
    expect(withPair(glue({ pairs: [] }), 0, 0)?.pairs).toEqual([{ a: 0, b: 0, weight: 0.5 }]);
  });

  it("removes a pair", () => {
    expect(withoutPair(glue(), 0).pairs).toEqual([{ a: 2, b: 3, weight: 0.5 }]);
  });

  it("adds and removes keys of the intensity grid", () => {
    expect(addGlueKeyCommand("Seam", "ArmL", 1)).toEqual({
      op: "add_keyform_key",
      target: { kind: "glue", id: "Seam" },
      parameter: "ArmL",
      key: 1,
    });
    expect(removeGlueKeyCommand("Seam", "ArmL", 1)).toEqual({
      op: "remove_keyform_key",
      target: { kind: "glue", id: "Seam" },
      parameter: "ArmL",
      key: 1,
    });
    const bound = glue({
      keyforms: {
        bindings: [{ parameter: "ArmL", keys: [0, 1] }],
        keyforms: [{ intensity: 0 }, { intensity: 1 }],
      },
    });
    expect(unboundParameters(bound, [{ id: "ArmL" }, { id: "AngleX" }])).toEqual([
      { id: "AngleX" },
    ]);
  });

  it("accepts keys inside the parameter range that are not on the axis yet", () => {
    const range = { min: -1, max: 1 };
    expect(keyValue("0.5", range)).toBe(0.5);
    expect(keyValue("", range)).toBeNull();
    expect(keyValue("2", range)).toBeNull();
    expect(keyValue("x", range)).toBeNull();
    expect(keyValue("1", range, [0, 1])).toBeNull();
  });

  it("treats a key within the keyform tolerance as already on the axis", () => {
    // f32 を経由したキー (0.1 は 0.10000000149011612 になる) も同じキーとみなす。
    expect(keyValue("0.1", { min: -1, max: 1 }, [Math.fround(0.1)])).toBeNull();
    expect(keyValue("0.2", { min: -1, max: 1 }, [Math.fround(0.1)])).toBe(0.2);
  });
});
