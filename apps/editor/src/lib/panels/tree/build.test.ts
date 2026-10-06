import { describe, expect, it } from "vitest";
import type { Model } from "../../generated/types";
import {
  buildDeformerTree,
  buildPartTree,
  exists,
  flatten,
  indexModel,
  meshesWithin,
  nearestVisible,
  pathTo,
  type TreeNode,
} from "./build";
import { keyformAtDefaults, treeModel } from "./model";
import { at, testModel } from "./testModel";

/** 節を `key(子, ...)` の形の文字列にする (比較しやすくするため)。 */
const shape = (nodes: TreeNode[]): string[] =>
  nodes.map((n) => (n.children.length ? `${n.key}(${shape(n.children).join(" ")})` : n.key));

describe("buildPartTree", () => {
  it("nests parts and lists deformers then meshes front to back", () => {
    const tree = buildPartTree(indexModel(testModel()));
    expect(shape(tree)).toEqual([
      "part:Root(" +
        "part:Face(part:Eyes(deformer:EyeRot mesh:EyeR mesh:EyeMask mesh:EyeL) " +
        "deformer:FaceWarp deformer:HeadRot mesh:Skin) " +
        "part:Hidden(mesh:Ghost))",
    ]);
  });

  it("marks hidden, mask-only and deformer kinds", () => {
    const rows = flatten(buildPartTree(indexModel(testModel())), new Set());
    const byKey = new Map(rows.map((r) => [r.node.key, r.node]));
    expect(byKey.get("part:Hidden")?.hidden).toBe(true);
    // 祖先パーツの不透明度が 0 なら、メッシュも非表示。
    expect(byKey.get("mesh:Ghost")?.hidden).toBe(true);
    expect(byKey.get("mesh:Skin")?.hidden).toBe(false);
    expect(byKey.get("mesh:EyeMask")?.icon).toBe("mask");
    expect(byKey.get("mesh:EyeMask")?.maskOnly).toBe(true);
    expect(byKey.get("deformer:FaceWarp")?.icon).toBe("warp");
    expect(byKey.get("deformer:HeadRot")?.icon).toBe("rotation");
  });

  it("keeps elements with a missing parent at the top", () => {
    const model = testModel();
    model.parts.push({
      id: "Orphan",
      name: "Orphan",
      parent: "Nope",
      keyforms: at(model.parts, 0).keyforms,
    });
    model.meshes.push({ ...at(model.meshes, 0), id: "Lost", part: "Nope" });
    const tree = buildPartTree(indexModel(model));
    expect(tree.map((n) => n.key)).toEqual(["part:Root", "part:Orphan", "mesh:Lost"]);
  });

  it("does not loop on cyclic parts", () => {
    const model = testModel();
    at(model.parts, 0).parent = "Eyes";
    const keys = flatten(buildPartTree(indexModel(model)), new Set()).map((r) => r.node.key);
    expect(keys.filter((k) => k.startsWith("part:")).sort()).toEqual([
      "part:Eyes",
      "part:Face",
      "part:Hidden",
      "part:Root",
    ]);
  });
});

describe("buildDeformerTree", () => {
  it("nests deformers with meshes under their parent deformer", () => {
    const tree = buildDeformerTree(indexModel(testModel()));
    expect(shape(tree)).toEqual([
      "deformer:FaceWarp(deformer:HeadRot(deformer:EyeRot(mesh:EyeR mesh:EyeMask mesh:EyeL)) mesh:Skin)",
      "mesh:Ghost",
    ]);
  });
});

describe("flatten / pathTo", () => {
  it("skips collapsed subtrees and tracks depth and parent", () => {
    const tree = buildPartTree(indexModel(testModel()));
    const rows = flatten(tree, new Set(["part:Face"]));
    expect(rows.map((r) => `${r.depth}:${r.node.key}`)).toEqual([
      "0:part:Root",
      "1:part:Face",
      "1:part:Hidden",
      "2:mesh:Ghost",
    ]);
    expect(rows[3]?.parent).toBe("part:Hidden");
  });

  it("finds the ancestors of a node", () => {
    const tree = buildPartTree(indexModel(testModel()));
    expect(pathTo(tree, "mesh:EyeL")).toEqual(["part:Eyes", "part:Face", "part:Root"]);
    expect(pathTo(tree, "part:Root")).toEqual([]);
    expect(pathTo(tree, "mesh:Nope")).toBeNull();
  });
});

describe("meshesWithin / nearestVisible", () => {
  it("collects the meshes inside a part or a deformer front to back", () => {
    const index = indexModel(testModel());
    expect(meshesWithin(index, { kind: "part", id: "Root" })).toEqual([
      "EyeR",
      "EyeMask",
      "EyeL",
      // 描画順が同じなら `meshes` で後ろのものが手前
      "Ghost",
      "Skin",
    ]);
    expect(meshesWithin(index, { kind: "part", id: "Eyes" })).toEqual(["EyeR", "EyeMask", "EyeL"]);
    // デフォーマは子孫のデフォーマを親に持つメッシュも含む (パーツの表示の種類で平らに並べていても)。
    expect(meshesWithin(index, { kind: "deformer", id: "HeadRot" })).toEqual([
      "EyeR",
      "EyeMask",
      "EyeL",
    ]);
    expect(meshesWithin(index, { kind: "deformer", id: "FaceWarp" })).toHaveLength(4);
    expect(meshesWithin(index, { kind: "mesh", id: "Skin" })).toEqual(["Skin"]);
    expect(meshesWithin(index, { kind: "mesh", id: "Nope" })).toEqual([]);
    expect(meshesWithin(index, { kind: "parameter", id: "Angle" })).toEqual([]);
  });

  it("does not loop on cyclic parts", () => {
    const model = testModel();
    model.parts = model.parts.map((p) => (p.id === "Root" ? { ...p, parent: "Eyes" } : p));
    // Root → Eyes → Face → Root の循環なので、Hidden の Ghost も Face の中になる
    expect(meshesWithin(indexModel(model), { kind: "part", id: "Face" })).toHaveLength(5);
  });

  it("finds the nearest visible ancestor of a node under a collapsed one", () => {
    const tree = buildPartTree(indexModel(testModel()));
    const visible = (collapsed: string[]) =>
      new Set(flatten(tree, new Set(collapsed)).map((r) => r.node.key));
    expect(nearestVisible(tree, visible([]), "mesh:EyeL")).toBe("mesh:EyeL");
    expect(nearestVisible(tree, visible(["part:Eyes"]), "mesh:EyeL")).toBe("part:Eyes");
    // 畳んだ祖先の中に、さらに畳んだ節があっても外側の見えている行
    expect(nearestVisible(tree, visible(["part:Eyes", "part:Face"]), "mesh:EyeL")).toBe(
      "part:Face",
    );
    expect(nearestVisible(tree, visible(["part:Root"]), "mesh:Ghost")).toBe("part:Root");
    expect(nearestVisible(tree, visible([]), "mesh:Nope")).toBeNull();
  });
});

describe("model helpers", () => {
  it("picks the keyform cell nearest to the parameter defaults", () => {
    const grid = {
      bindings: [
        { parameter: "A", keys: [-1, 0, 1] },
        { parameter: "B", keys: [0, 10] },
      ],
      keyforms: [0, 1, 2, 3, 4, 5],
    };
    // A = 0.9 → 軸 0 のキー 2、B = 7 → 軸 1 のキー 1。インデックス = 2 + 3 * 1。
    const defaults = new Map([
      ["A", 0.9],
      ["B", 7],
    ]);
    expect(keyformAtDefaults(grid, defaults)).toBe(5);
    expect(keyformAtDefaults({ keyforms: ["only"] }, new Map())).toBe("only");
  });

  it("reads missing lists as empty", () => {
    const model = treeModel({} as Model);
    expect(model).toEqual({ parameters: [], parts: [], meshes: [], deformers: [] });
    expect(indexModel(model).model.meshes).toEqual([]);
  });

  it("reads deformer keyforms from the kind", () => {
    const model = treeModel({
      parameters: [],
      parts: [],
      meshes: [],
      deformers: [
        {
          id: "W",
          name: "W",
          part: "P",
          parent: null,
          kind: { type: "warp", rows: 1, cols: 1, keyforms: { keyforms: [{ opacity: 0 }] } },
        },
      ],
    } as unknown as Model);
    expect(model.deformers[0]?.keyforms?.keyforms[0]?.opacity).toBe(0);
  });

  it("checks whether a selected element still exists", () => {
    const index = indexModel(testModel());
    expect(exists(index, { kind: "mesh", id: "Skin" })).toBe(true);
    expect(exists(index, { kind: "deformer", id: "Skin" })).toBe(false);
    expect(exists(index, { kind: "parameter", id: "Angle" })).toBe(true);
  });
});
