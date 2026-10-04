// ツリーが読むモデルの JSON の一部。生成した型 (ugoe_model、lib/generated/types.ts) から、
// ツリーの表示と編集に要るフィールドだけを取り出して組み立てる。仕様は docs/spec/model.md。
import type {
  Deformer,
  DeformerKind,
  KeyformGrid as FullKeyformGrid,
  Mesh,
  MeshKeyform,
  Model,
  Parameter,
  Part,
  PartKeyform,
  RotationKeyform,
  WarpKeyform,
} from "../../generated/types";

/** キーフォーム格子のうち、ツリーが読む部分 (軸は省略できる: 軸なしの定数格子)。 */
export type KeyformGrid<T> = Pick<FullKeyformGrid<T>, "keyforms"> &
  Partial<Pick<FullKeyformGrid<T>, "bindings">>;

export type TreeParameter = Pick<Parameter, "id" | "default">;

export type TreePart = Pick<Part, "id" | "name" | "parent"> & {
  keyforms: KeyformGrid<PartKeyform>;
};

export type TreeDeformer = Pick<Deformer, "id" | "name" | "part" | "parent"> & {
  /** `warp` / `rotation` (知らない種類は読み込み時に `Unknown` になり、書き出されない)。 */
  kind: Pick<DeformerKind, "type">;
  /** `kind` の中のキーフォーム (`treeModel` が取り出す)。 */
  keyforms?: KeyformGrid<Pick<WarpKeyform | RotationKeyform, "opacity">>;
};

export type TreeMesh = Pick<Mesh, "id" | "name" | "part" | "parent"> &
  Partial<Pick<Mesh, "mask_only" | "masks">> & {
    keyforms: KeyformGrid<Pick<MeshKeyform, "opacity" | "draw_order">>;
  };

export interface TreeModel {
  parameters: TreeParameter[];
  parts: TreePart[];
  deformers: TreeDeformer[];
  meshes: TreeMesh[];
}

/**
 * 解析したモデル (`EditorDocument.parsedModel`) からツリーが読む部分を取り出す。デフォーマのキーフォームは `kind` の中にある。
 * 欠けた配列は空として読む。
 */
export function treeModel(model: Model): TreeModel {
  const raw = model as Partial<Model>;
  return {
    parameters: raw.parameters ?? [],
    parts: raw.parts ?? [],
    meshes: raw.meshes ?? [],
    deformers: (raw.deformers ?? []).map((d) => ({
      ...d,
      keyforms: d.kind && "keyforms" in d.kind ? d.kind.keyforms : undefined,
    })),
  };
}

/**
 * パラメータが既定値のときのキーフォーム。各軸で既定値に最も近いキーのセルを選ぶ (補間はしない)。
 * ツリーの表示 (描画順・非表示の印) の目安に使う。格子の並びは軸 0 が最も速く変化する順 (docs/spec/model.md)。
 */
export function keyformAtDefaults<T>(
  grid: KeyformGrid<T> | undefined,
  defaults: ReadonlyMap<string, number>,
): T | undefined {
  if (!grid) return undefined;
  let index = 0;
  let stride = 1;
  for (const b of grid.bindings ?? []) {
    const value = defaults.get(b.parameter) ?? 0;
    let best = 0;
    b.keys.forEach((k, i) => {
      if (Math.abs(k - value) < Math.abs((b.keys[best] ?? 0) - value)) best = i;
    });
    index += best * stride;
    stride *= b.keys.length;
  }
  return grid.keyforms[index];
}
