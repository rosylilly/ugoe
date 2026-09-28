import { describe, expect, it } from "vitest";
import type { Glue, Model } from "../../../generated/types";
import {
  clonesOf,
  createCommand,
  drawCloneInfo,
  glueId,
  isInSync,
  newClone,
  originalOf,
} from "./drawClone";

const quad = (
  id: string,
  uvs: [number, number][] = [
    [0, 0],
    [1, 0],
    [0, 1],
  ],
) => ({
  id,
  name: id,
  part: "Root",
  parent: "Head",
  texture: 0,
  uvs,
  indices: [0, 1, 2],
  keyforms: { bindings: [], keyforms: [{ positions: uvs, opacity: 1, draw_order: 500 }] },
});

const link = (original: string, clone: string, n = 3): Glue =>
  ({
    id: glueId(clone),
    mesh_a: original,
    mesh_b: clone,
    pairs: Array.from({ length: n }, (_, i) => ({ a: i, b: i, weight: 0 })),
    keyforms: { bindings: [], keyforms: [{ intensity: 1 }] },
  }) as unknown as Glue;

// 複製に関わるフィールドだけを持つモデル (型検査のためにキャストする)。
function model(meshes: ReturnType<typeof quad>[], glues: Glue[]): Model {
  return { meshes, glues } as unknown as Model;
}

const synced = () =>
  model(
    [quad("Ear"), quad("Ear_Clone"), quad("Face")],
    [{ ...link("Face", "Ear"), id: "Seam" }, link("Ear", "Ear_Clone")],
  );

describe("links", () => {
  it("finds the original and the clones by the glue ID", () => {
    const m = synced();
    expect(originalOf(m, "Ear_Clone")).toBe("Ear");
    expect(originalOf(m, "Ear")).toBeNull();
    expect(clonesOf(m, "Ear")).toEqual(["Ear_Clone"]);
    // ID が規則に合わないグルー (Seam) は複製のつながりではない。
    expect(clonesOf(m, "Face")).toEqual([]);
  });

  it("describes clones and originals for the inspector", () => {
    const m = synced();
    const clone = drawCloneInfo(m, "Ear_Clone");
    expect(clone.role).toBe("clone");
    expect(clone.role === "clone" && clone.original?.id).toBe("Ear");
    expect(clone.role === "clone" && clone.inSync).toBe(true);
    const original = drawCloneInfo(m, "Ear");
    expect(original.role === "original" && original.clones.map((c) => c.id)).toEqual(["Ear_Clone"]);
  });
});

describe("isInSync", () => {
  it("accepts a clone that matches its original", () => {
    expect(isInSync(synced(), "Ear_Clone")).toBe(true);
  });

  it("detects drift of the topology, the glue and the glue order", () => {
    const uv = synced();
    (uv.meshes[1] as { uvs: [number, number][] }).uvs = [
      [0, 0],
      [1, 0],
      [0.5, 1],
    ];
    expect(isInSync(uv, "Ear_Clone")).toBe(false);

    const pairs = synced();
    (pairs.glues[1] as Glue).pairs.pop();
    expect(isInSync(pairs, "Ear_Clone")).toBe(false);

    const intensity = synced();
    (intensity.glues[1] as Glue).keyforms.keyforms[0] = { intensity: 0.5 };
    expect(isInSync(intensity, "Ear_Clone")).toBe(false);

    // 元を使う他のグルーが複製のグルーより後ろにある。
    const order = synced();
    order.glues.reverse();
    expect(isInSync(order, "Ear_Clone")).toBe(false);

    expect(isInSync(synced(), "Ear")).toBe(false);
  });
});

describe("commands", () => {
  it("creates a clone and finds its ID afterwards", () => {
    expect(createCommand("Ear")).toEqual({ op: "create_draw_clone", source: "Ear", name: "" });
    const before = synced();
    const after = model([...before.meshes, quad("Ear_Clone_2")] as ReturnType<typeof quad>[], [
      ...before.glues,
      link("Ear", "Ear_Clone_2"),
    ]);
    expect(newClone(before, after, "Ear")).toBe("Ear_Clone_2");
    // 複製から作っても元の複製になる。
    expect(newClone(before, after, "Ear_Clone")).toBe("Ear_Clone_2");
  });
});
