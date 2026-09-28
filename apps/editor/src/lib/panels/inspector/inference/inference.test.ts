import { describe, expect, it } from "vitest";
import type { Command, KeyformTarget, RederiveWarning } from "../../../generated/types";
import type { ElementGrid, KeyformBinding, ModelData } from "../keyforms/keyforms";
import {
  cellKey,
  describeMirrorReport,
  describeMirrorSource,
  describeWarning,
  formError,
  inferCommand,
  inferenceForm,
  inferenceFromForm,
  LEGEND_ORIGINS,
  markCommand,
  mergeWarnings,
  mirrorChoices,
  mirrorCommand,
  originStyle,
  originTitle,
  partnerCandidates,
  sameInference,
  setInferenceCommand,
  setMirrorPairCommand,
  targetName,
  toggleCell,
} from "./inference";

const eyeL: KeyformTarget = { kind: "mesh", id: "EyeL" };
const eyeR: KeyformTarget = { kind: "mesh", id: "EyeR" };
const bindings: KeyformBinding[] = [
  { parameter: "AngleX", keys: [-30, 0, 30] },
  { parameter: "AngleY", keys: [-30, 0, 30] },
];

const grid = { bindings, keyforms: [] };
const model: ModelData = {
  parameters: [],
  parts: [{ id: "Root", keyforms: grid }],
  deformers: [{ id: "Face", kind: { type: "warp", keyforms: grid } }],
  meshes: [
    { id: "EyeL", indices: [], keyforms: grid, mirror: "EyeR", mirror_axis: 0.5 },
    { id: "EyeR", indices: [], keyforms: grid, mirror: "EyeL" },
    { id: "Nose", indices: [], keyforms: grid },
  ],
};
const element = (target: KeyformTarget): ElementGrid => ({
  target,
  kind: target.kind === "deformer" ? "warp" : (target.kind as "mesh" | "part"),
  grid,
});

describe("origin styles", () => {
  it("gives every origin a distinct symbol and color", () => {
    const styles = LEGEND_ORIGINS.map(originStyle);
    expect(new Set(styles.map((s) => s.symbol)).size).toBe(LEGEND_ORIGINS.length);
    expect(new Set(styles.map((s) => s.color)).size).toBe(LEGEND_ORIGINS.length);
    expect(originStyle("mirror").symbol).toBe("M");
    expect(originStyle("authored").symbol).toBe("A");
  });

  it("explains derived and mirrored cells", () => {
    expect(originTitle("derived", null)).toContain("additive blending");
    expect(
      originTitle("derived", {
        method: "depth_rotation",
        angle_scale: 1,
        pivot: [0, 0],
        pivot_depth: 0,
      }),
    ).toContain("depth rotation");
    expect(originTitle("mirror", null, { id: "EyeL" })).toBe("Mirror: mirrored from EyeL");
    expect(originTitle("mirror", null, null)).toContain("not re-derived");
  });
});

describe("inference form", () => {
  it("defaults the rotation axes to AngleX / AngleY when they are grid axes", () => {
    const form = inferenceForm(null, bindings, [10, 20]);
    expect(form).toMatchObject({ method: "additive", yaw: "AngleX", pitch: "AngleY" });
    expect([form.pivotX, form.pivotY]).toEqual([10, 20]);
    expect(inferenceForm(undefined, bindings.slice(0, 1)).pitch).toBe("");
  });

  it("round-trips depth rotation settings", () => {
    const inference = {
      method: "depth_rotation" as const,
      angle_scale: 0.8,
      pivot: [1, 2] as [number, number],
      pivot_depth: -4,
      yaw: "AngleX",
    };
    const form = inferenceForm(inference, bindings);
    expect(form.pitch).toBe("");
    expect(inferenceFromForm(form)).toEqual(inference);
    expect(sameInference(inferenceFromForm(form), inference)).toBe(true);
    expect(sameInference(inferenceFromForm({ ...form, angleScale: 1 }), inference)).toBe(false);
  });

  it("treats additive as no setting", () => {
    expect(inferenceFromForm(inferenceForm(null, bindings))).toBeNull();
    expect(sameInference(null, { method: "additive" })).toBe(true);
  });

  it("rejects non-finite numbers and missing axes", () => {
    const form = { ...inferenceForm(null, bindings), method: "depth_rotation" as const };
    expect(formError(form)).toBeNull();
    expect(formError({ ...form, pivotX: Number.NaN })).toBe("Pivot x must be a number");
    expect(formError({ ...form, yaw: "", pitch: "" })).toMatch(/yaw or pitch/);
    expect(formError({ ...form, method: "additive", pivotX: Number.NaN })).toBeNull();
  });
});

describe("commands", () => {
  it("builds inference commands", () => {
    expect(inferCommand(eyeL, true)).toEqual({
      op: "infer_keyforms",
      target: eyeL,
      method: "from_grid",
      overwrite_derived: true,
    } satisfies Command);
    expect(setInferenceCommand(eyeL, null)).toEqual({
      op: "set_keyform_inference",
      target: eyeL,
      inference: null,
    });
    const cells = [{ AngleX: 30, AngleY: 0 }];
    expect(markCommand(eyeL, cells, "derived")).toEqual({
      op: "mark_keyform_derived",
      target: eyeL,
      cells,
    });
    expect(markCommand(eyeL, cells, "authored").op).toBe("mark_keyform_authored");
  });

  it("omits empty mirror options", () => {
    expect(
      mirrorCommand(eyeL, eyeR, { axis: null, tolerance: Number.NaN, overwriteAuthored: false }),
    ).toEqual({ op: "mirror_keyforms", source: eyeL, target: eyeR, overwrite_authored: false });
    expect(mirrorCommand(eyeL, eyeR, { axis: 0, tolerance: 0.5, overwriteAuthored: true })).toEqual(
      {
        op: "mirror_keyforms",
        source: eyeL,
        target: eyeR,
        axis: 0,
        tolerance: 0.5,
        overwrite_authored: true,
      },
    );
  });

  it("builds mirror pair commands", () => {
    expect(setMirrorPairCommand(eyeL, "Nose", 0.25)).toEqual({
      op: "set_mirror_pair",
      target: eyeL,
      mirror: "Nose",
      axis: 0.25,
    });
    expect(setMirrorPairCommand(eyeL, null, null)).toEqual({ op: "set_mirror_pair", target: eyeL });
  });
});

describe("mirror sources", () => {
  it("offers the partner first, then the element itself", () => {
    const choices = mirrorChoices(model, element(eyeL));
    expect(choices.map((c) => c.source)).toEqual([eyeR, eyeL]);
    expect(
      mirrorChoices(model, element({ kind: "mesh", id: "Nose" })).map((c) => c.source),
    ).toEqual([{ kind: "mesh", id: "Nose" }]);
    // パーツは対もミラーも持たない。
    expect(mirrorChoices(model, element({ kind: "part", id: "Root" }))).toEqual([]);
  });

  it("lists partner candidates of the same kind", () => {
    expect(partnerCandidates(model, eyeL).map((c) => c.id)).toEqual(["EyeR", "Nose"]);
    expect(partnerCandidates(model, { kind: "deformer", id: "Face" })).toEqual([]);
    expect(partnerCandidates(model, { kind: "part", id: "Root" })).toEqual([]);
  });

  it("describes the recorded mirror source", () => {
    expect(describeMirrorSource({ id: "EyeL" }, "EyeR")).toBe("Mirrored cells come from EyeL.");
    expect(describeMirrorSource({ id: "Nose", axis: 0.5, tolerance: 0.01 }, "Nose")).toBe(
      "Mirrored cells come from Nose itself (axis 0.5, tolerance 0.01).",
    );
    expect(describeMirrorSource({ id: "Warp_2" }, "Warp_3", (id) => `左目 (${id})`)).toBe(
      "Mirrored cells come from 左目 (Warp_2).",
    );
  });

  it("labels elements with their display names", () => {
    const named = {
      ...model,
      meshes: model.meshes.map((m) => (m.id === "EyeR" ? { ...m, name: "右目" } : m)),
    };
    expect(mirrorChoices(named, element(eyeL))[0]?.label).toBe("右目 (EyeR) (mirror partner)");
    expect(partnerCandidates(named, eyeL)[0]).toEqual({ id: "EyeR", label: "右目 (EyeR)" });
    expect(targetName(named, eyeR)).toBe("右目 (EyeR)");
  });

  it("explains the mirror result", () => {
    const label = (t: KeyformTarget) => t.id;
    const report = { source: eyeL, target: eyeR, written: 3, unedited: 1, kept_authored: 1 };
    expect(describeMirrorReport(report, label)).toEqual({
      text: "Mirrored 3 cells from EyeL. Kept 1 cell authored (not overwritten).",
      overwrite: false,
    });
    const none = { ...report, written: 0, unedited: 0, kept_authored: 2 };
    expect(describeMirrorReport(none, label).overwrite).toBe(true);
    expect(describeMirrorReport(none, label).text).toContain("all 2 cells that EyeL maps to");
    const nothing = { ...none, kept_authored: 0 };
    expect(describeMirrorReport(nothing, label).overwrite).toBe(false);
  });
});

describe("cell selection", () => {
  it("compares cells regardless of axis order and toggles them", () => {
    expect(cellKey({ AngleX: 0, AngleY: 30 })).toBe(cellKey({ AngleY: 30, AngleX: 0 }));
    const a = { AngleX: 0, AngleY: 30 };
    const b = { AngleX: 30, AngleY: 30 };
    expect(toggleCell([a], b)).toEqual([a, b]);
    expect(toggleCell([a, b], { AngleY: 30, AngleX: 0 })).toEqual([b]);
  });
});

describe("re-derivation warnings", () => {
  const warning = (message: string): RederiveWarning => ({
    target: eyeR,
    source: eyeL,
    message,
  });

  it("describes the failure and that the cells were kept", () => {
    expect(describeWarning(warning("vertices [0] unmatched"))).toBe(
      "Could not re-mirror mesh EyeR from mesh EyeL: vertices [0] unmatched. " +
        "Its mirrored cells keep their previous values.",
    );
  });

  it("keeps one warning per grid and source, the newest", () => {
    const other: RederiveWarning = { target: eyeL, source: eyeR, message: "x" };
    expect(mergeWarnings([warning("old"), other], [warning("new")])).toEqual([
      other,
      warning("new"),
    ]);
  });
});
