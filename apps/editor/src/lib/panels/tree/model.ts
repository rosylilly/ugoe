// ツリーが読むモデルの JSON の一部 (ugoe_model の型のうち、ツリーの表示と編集に要るフィールドだけ)。
// 仕様は docs/spec/model.md。TODO(T0603): ts-rs で生成した型に置き換える。

export interface KeyformBinding {
  parameter: string;
  keys: number[];
}

export interface KeyformGrid<T> {
  bindings?: KeyformBinding[];
  keyforms: T[];
}

export interface TreeParameter {
  id: string;
  default: number;
}

export interface TreePart {
  id: string;
  name: string;
  parent: string | null;
  keyforms: KeyformGrid<{ opacity: number }>;
}

export interface TreeDeformer {
  id: string;
  name: string;
  part: string;
  parent: string | null;
  /** `warp` / `rotation` (知らない種類は読み込み時に `Unknown` になり、書き出されない)。 */
  kind: { type: string };
  keyforms?: KeyformGrid<{ opacity?: number }>;
}

export interface TreeMesh {
  id: string;
  name: string;
  part: string;
  parent: string | null;
  mask_only?: boolean;
  masks?: { mesh: string; invert?: boolean }[];
  keyforms: KeyformGrid<{ opacity: number; draw_order: number }>;
}

export interface TreeModel {
  parameters: TreeParameter[];
  parts: TreePart[];
  deformers: TreeDeformer[];
  meshes: TreeMesh[];
}

/** get_model_json の文字列を読む。デフォーマのキーフォームは `kind` の中にある。 */
export function parseTreeModel(json: string): TreeModel {
  const raw = JSON.parse(json);
  return {
    parameters: raw.parameters ?? [],
    parts: raw.parts ?? [],
    meshes: raw.meshes ?? [],
    deformers: (raw.deformers ?? []).map((d: TreeDeformer & { kind: { keyforms?: unknown } }) => ({
      ...d,
      keyforms: d.kind?.keyforms,
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
