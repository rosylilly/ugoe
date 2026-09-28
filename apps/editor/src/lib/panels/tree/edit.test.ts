import { describe, expect, it } from "vitest";
import type { ElementRef } from "../../selection.svelte";
import { indexModel } from "./build";
import {
  addPartCommand,
  deleteCommand,
  dropCommand,
  parentPartFor,
  renameCommand,
  uniqueId,
} from "./edit";
import { at, testModel } from "./testModel";

const part = (id: string): ElementRef => ({ kind: "part", id });
const deformer = (id: string): ElementRef => ({ kind: "deformer", id });
const mesh = (id: string): ElementRef => ({ kind: "mesh", id });
const index = () => indexModel(testModel());

describe("dropCommand into", () => {
  it("reparents parts and rejects cycles", () => {
    expect(
      dropCommand(index(), "parts", [part("Eyes")], { target: part("Hidden"), position: "into" }),
    ).toEqual({
      ok: true,
      command: { op: "set_part_parent", id: "Eyes", parent: "Hidden" },
    });
    const cycle = dropCommand(index(), "parts", [part("Root")], {
      target: part("Eyes"),
      position: "into",
    });
    expect(cycle).toMatchObject({ ok: false, reason: expect.stringContaining("descendant") });
    const self = dropCommand(index(), "parts", [part("Face")], {
      target: part("Face"),
      position: "into",
    });
    expect(self.ok).toBe(false);
  });

  it("moves parts to the top level", () => {
    expect(
      dropCommand(index(), "parts", [part("Face")], { target: null, position: "into" }),
    ).toEqual({
      ok: true,
      command: { op: "set_part_parent", id: "Face", parent: null },
    });
    expect(
      dropCommand(index(), "parts", [part("Root")], { target: null, position: "into" }),
    ).toMatchObject({
      ok: false,
      noop: true,
    });
  });

  it("reparents deformers and rejects cycles", () => {
    expect(
      dropCommand(index(), "deformers", [deformer("EyeRot")], {
        target: deformer("FaceWarp"),
        position: "into",
      }),
    ).toEqual({
      ok: true,
      command: {
        op: "reparent_preserving_look",
        element: { kind: "deformer", id: "EyeRot" },
        new_parent: "FaceWarp",
      },
    });
    expect(
      dropCommand(index(), "deformers", [deformer("FaceWarp")], {
        target: deformer("EyeRot"),
        position: "into",
      }),
    ).toMatchObject({ ok: false, reason: expect.stringContaining("descendant") });
    expect(
      dropCommand(index(), "deformers", [deformer("HeadRot")], { target: null, position: "into" }),
    ).toEqual({
      ok: true,
      command: {
        op: "reparent_preserving_look",
        element: { kind: "deformer", id: "HeadRot" },
        new_parent: null,
      },
    });
  });

  it("moves deformers between parts in the parts view", () => {
    expect(
      dropCommand(index(), "parts", [deformer("EyeRot")], {
        target: part("Face"),
        position: "into",
      }),
    ).toEqual({
      ok: true,
      command: { op: "set_deformer_part", id: "EyeRot", part: "Face" },
    });
    expect(
      dropCommand(index(), "parts", [deformer("EyeRot")], { target: null, position: "into" }).ok,
    ).toBe(false);
    expect(
      dropCommand(index(), "parts", [part("Face")], {
        target: deformer("EyeRot"),
        position: "into",
      }).ok,
    ).toBe(false);
  });

  it("moves meshes to parts and deformers", () => {
    expect(
      dropCommand(index(), "parts", [mesh("Skin")], { target: part("Eyes"), position: "into" }),
    ).toEqual({
      ok: true,
      command: { op: "set_mesh_properties", id: "Skin", part: "Eyes" },
    });
    expect(
      dropCommand(index(), "deformers", [mesh("Skin")], {
        target: deformer("EyeRot"),
        position: "into",
      }),
    ).toEqual({
      ok: true,
      command: {
        op: "reparent_preserving_look",
        element: { kind: "mesh", id: "Skin" },
        new_parent: "EyeRot",
      },
    });
    expect(
      dropCommand(index(), "deformers", [mesh("Skin")], { target: null, position: "into" }),
    ).toEqual({
      ok: true,
      command: {
        op: "reparent_preserving_look",
        element: { kind: "mesh", id: "Skin" },
        new_parent: null,
      },
    });
    expect(
      dropCommand(index(), "parts", [mesh("Skin")], { target: null, position: "into" }).ok,
    ).toBe(false);
    expect(
      dropCommand(index(), "parts", [mesh("Skin")], { target: mesh("EyeL"), position: "into" }).ok,
    ).toBe(false);
  });

  it("batches several elements and skips those already there", () => {
    const r = dropCommand(index(), "parts", [mesh("Skin"), mesh("EyeL"), deformer("FaceWarp")], {
      target: part("Eyes"),
      position: "into",
    });
    expect(r).toEqual({
      ok: true,
      command: {
        op: "batch",
        label: "Move 2 elements",
        commands: [
          { op: "set_mesh_properties", id: "Skin", part: "Eyes" },
          { op: "set_deformer_part", id: "FaceWarp", part: "Eyes" },
        ],
      },
    });
    // 1 つでも移せない要素があれば全体を拒否する。
    const bad = dropCommand(index(), "parts", [mesh("Skin"), part("Root")], {
      target: part("Eyes"),
      position: "into",
    });
    expect(bad.ok).toBe(false);
  });
});

describe("dropCommand reorder", () => {
  // meshes の順: Skin, EyeL, EyeMask, EyeR, Ghost。ツリーでは手前から EyeR, EyeMask, EyeL。
  const eyes = { container: part("Eyes") };

  it("moves a mesh in front of the target (above in the tree)", () => {
    expect(
      dropCommand(index(), "parts", [mesh("EyeL")], {
        target: mesh("EyeR"),
        position: "above",
        ...eyes,
      }),
    ).toEqual({ ok: true, command: { op: "reorder_mesh", id: "EyeL", before: "Ghost" } });
  });

  it("moves a mesh behind the target (below in the tree)", () => {
    expect(
      dropCommand(index(), "parts", [mesh("EyeR")], {
        target: mesh("EyeL"),
        position: "below",
        ...eyes,
      }),
    ).toEqual({ ok: true, command: { op: "reorder_mesh", id: "EyeR", before: "EyeL" } });
  });

  it("detects no-op moves", () => {
    // EyeL は EyeMask の直前 (= すぐ奥) にある。
    expect(
      dropCommand(index(), "parts", [mesh("EyeL")], {
        target: mesh("EyeMask"),
        position: "below",
        ...eyes,
      }),
    ).toMatchObject({ ok: false, noop: true });
    expect(
      dropCommand(index(), "parts", [mesh("EyeR")], {
        target: mesh("EyeMask"),
        position: "above",
        ...eyes,
      }),
    ).toMatchObject({ ok: false, noop: true });
  });

  it("uses null when moving to the very end", () => {
    const model = testModel();
    model.meshes.pop(); // Ghost を消すと EyeR が末尾。
    expect(
      dropCommand(indexModel(model), "parts", [mesh("EyeL")], {
        target: mesh("EyeR"),
        position: "above",
        ...eyes,
      }),
    ).toEqual({ ok: true, command: { op: "reorder_mesh", id: "EyeL", before: null } });
  });

  it("rejects different draw orders, other containers and multiple items", () => {
    const model = testModel();
    at(model.meshes, 0).part = "Eyes"; // Skin (描画順 500) を Eyes に。
    expect(
      dropCommand(indexModel(model), "parts", [mesh("Skin")], {
        target: mesh("EyeL"),
        position: "above",
        ...eyes,
      }),
    ).toMatchObject({ ok: false, reason: expect.stringContaining("draw order") });
    expect(
      dropCommand(index(), "parts", [mesh("Skin")], {
        target: mesh("EyeL"),
        position: "above",
        ...eyes,
      }).ok,
    ).toBe(false);
    expect(
      dropCommand(index(), "parts", [mesh("EyeL"), mesh("EyeR")], {
        target: mesh("EyeMask"),
        position: "above",
        ...eyes,
      }).ok,
    ).toBe(false);
  });

  it("reorders within the parent deformer in the deformers view", () => {
    expect(
      dropCommand(index(), "deformers", [mesh("EyeR")], {
        target: mesh("EyeL"),
        position: "below",
        container: deformer("EyeRot"),
      }),
    ).toEqual({ ok: true, command: { op: "reorder_mesh", id: "EyeR", before: "EyeL" } });
  });
});

describe("other commands", () => {
  it("renames by kind", () => {
    expect(renameCommand(deformer("A"), "B")).toEqual({
      op: "rename_deformer",
      id: "A",
      name: "B",
    });
    expect(renameCommand({ kind: "parameter", id: "A" }, "B")).toBeNull();
  });

  it("deletes children before parents in one batch", () => {
    expect(
      deleteCommand(index(), [part("Face"), mesh("Skin"), part("Eyes"), deformer("FaceWarp")]),
    ).toEqual({
      op: "batch",
      label: "Delete 4 elements",
      commands: [
        { op: "remove_mesh", id: "Skin" },
        { op: "remove_deformer", id: "FaceWarp" },
        { op: "remove_part", id: "Eyes" },
        { op: "remove_part", id: "Face" },
      ],
    });
    expect(deleteCommand(index(), [])).toBeNull();
  });

  it("adds a part with a unique id under the selection's part", () => {
    const i = index();
    expect(parentPartFor(i, mesh("EyeL"))).toBe("Eyes");
    expect(parentPartFor(i, null)).toBeNull();
    expect(addPartCommand(i, "Eyes")).toEqual({
      op: "add_part",
      part: {
        id: "Part",
        name: "Part",
        parent: "Eyes",
        keyforms: { bindings: [], keyforms: [{ opacity: 1 }] },
      },
    });
    expect(uniqueId((id) => id === "Part" || id === "Part2", "Part")).toBe("Part3");
  });
});
