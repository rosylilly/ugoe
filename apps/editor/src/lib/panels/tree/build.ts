// モデルからツリーの節を組み立てる (Svelte に依存しない純粋な処理。テストは build.test.ts)。
//
// - パーツのツリー: パーツの階層。各パーツの下に子パーツ、そのパーツに属するデフォーマ (平らに)、
//   メッシュ (描画順で手前から) を並べる。
// - デフォーマのツリー: デフォーマの階層。各デフォーマの下に子デフォーマ、そのデフォーマを親に持つメッシュを並べる。
//   親のないデフォーマとメッシュは最上位に置く。
import type { ElementRef } from "../../selection.svelte";
import { keyformAtDefaults, type TreeModel } from "./model";

export type TreeView = "parts" | "deformers";

export type NodeIcon = "part" | "warp" | "rotation" | "deformer" | "mesh" | "mask";

export interface TreeNode {
  /** `kind:id` (1 つのツリーの中で一意)。 */
  key: string;
  ref: ElementRef;
  name: string;
  icon: NodeIcon;
  /** パラメータが既定値のとき、祖先 (パーツ・デフォーマ) を含めた不透明度が 0。 */
  hidden: boolean;
  /** マスクとしてだけ働くメッシュ。 */
  maskOnly: boolean;
  /** メッシュの描画順 (パラメータが既定値のとき)。 */
  drawOrder?: number;
  children: TreeNode[];
}

export const refKey = (r: ElementRef) => `${r.kind}:${r.id}`;

/** モデルを引きやすくした索引。ツリーの組み立てとドロップの検査が使う。 */
export interface ModelIndex {
  model: TreeModel;
  defaults: Map<string, number>;
  parts: Map<string, TreeModel["parts"][number]>;
  deformers: Map<string, TreeModel["deformers"][number]>;
  meshes: Map<string, TreeModel["meshes"][number]>;
  /** メッシュの `meshes` 配列での位置 (描画順が同じときの前後関係)。 */
  meshIndex: Map<string, number>;
}

export function indexModel(model: TreeModel): ModelIndex {
  return {
    model,
    defaults: new Map(model.parameters.map((p) => [p.id, p.default])),
    parts: new Map(model.parts.map((p) => [p.id, p])),
    deformers: new Map(model.deformers.map((d) => [d.id, d])),
    meshes: new Map(model.meshes.map((m) => [m.id, m])),
    meshIndex: new Map(model.meshes.map((m, i) => [m.id, i])),
  };
}

/** 要素がモデルにあるか (再読み込み後の選択の検査に使う)。 */
export function exists(index: ModelIndex, ref: ElementRef): boolean {
  switch (ref.kind) {
    case "part":
      return index.parts.has(ref.id);
    case "deformer":
      return index.deformers.has(ref.id);
    case "mesh":
      return index.meshes.has(ref.id);
    case "parameter":
      return index.defaults.has(ref.id);
  }
}

export function meshDrawOrder(index: ModelIndex, id: string): number {
  const mesh = index.meshes.get(id);
  return keyformAtDefaults(mesh?.keyforms, index.defaults)?.draw_order ?? 500;
}

/** 祖先のたどり。循環していても止まる。 */
export function ancestors<T extends { parent: string | null }>(
  items: ReadonlyMap<string, T>,
  id: string | null,
): string[] {
  const out: string[] = [];
  const seen = new Set<string>();
  let cur = id;
  while (cur !== null && !seen.has(cur)) {
    seen.add(cur);
    out.push(cur);
    cur = items.get(cur)?.parent ?? null;
  }
  return out;
}

function partOpacity(index: ModelIndex, id: string | null): number {
  let o = 1;
  for (const p of ancestors(index.parts, id)) {
    o *= keyformAtDefaults(index.parts.get(p)?.keyforms, index.defaults)?.opacity ?? 1;
  }
  return o;
}

function deformerOpacity(index: ModelIndex, id: string | null): number {
  let o = 1;
  for (const d of ancestors(index.deformers, id)) {
    o *= keyformAtDefaults(index.deformers.get(d)?.keyforms, index.defaults)?.opacity ?? 1;
  }
  return o;
}

/** メッシュを手前から奥へ並べる (描画順の降順、同じなら `meshes` で後ろのものが手前)。 */
export function frontToBack(index: ModelIndex, ids: string[]): string[] {
  return [...ids].sort(
    (a, b) =>
      meshDrawOrder(index, b) - meshDrawOrder(index, a) ||
      (index.meshIndex.get(b) ?? 0) - (index.meshIndex.get(a) ?? 0),
  );
}

function meshNode(index: ModelIndex, id: string): TreeNode {
  const m = index.meshes.get(id);
  const kf = keyformAtDefaults(m?.keyforms, index.defaults);
  const opacity =
    (kf?.opacity ?? 1) *
    partOpacity(index, m?.part ?? null) *
    deformerOpacity(index, m?.parent ?? null);
  const maskOnly = m?.mask_only ?? false;
  return {
    key: `mesh:${id}`,
    ref: { kind: "mesh", id },
    name: m?.name ?? id,
    icon: maskOnly ? "mask" : "mesh",
    hidden: opacity === 0,
    maskOnly,
    drawOrder: kf?.draw_order,
    children: [],
  };
}

function deformerIcon(type: string | undefined): NodeIcon {
  return type === "warp" || type === "rotation" ? type : "deformer";
}

function deformerNode(index: ModelIndex, id: string, children: TreeNode[]): TreeNode {
  const d = index.deformers.get(id);
  return {
    key: `deformer:${id}`,
    ref: { kind: "deformer", id },
    name: d?.name ?? id,
    icon: deformerIcon(d?.kind?.type),
    hidden: deformerOpacity(index, id) === 0,
    maskOnly: false,
    children,
  };
}

/** 親が見つからない要素は最上位に置く (壊れたデータでも全要素を表示する)。 */
function groupByParent<T extends { id: string }>(
  items: T[],
  parentOf: (item: T) => string | null,
  known: (id: string) => boolean,
): Map<string | null, string[]> {
  const out = new Map<string | null, string[]>();
  for (const item of items) {
    const p = parentOf(item);
    const key = p !== null && known(p) ? p : null;
    const list = out.get(key) ?? [];
    list.push(item.id);
    out.set(key, list);
  }
  return out;
}

export function buildPartTree(index: ModelIndex): TreeNode[] {
  const { model } = index;
  const childParts = groupByParent(
    model.parts,
    (p) => p.parent,
    (id) => index.parts.has(id),
  );
  const deformersOf = groupByParent(
    model.deformers,
    (d) => d.part,
    (id) => index.parts.has(id),
  );
  const meshesOf = groupByParent(
    model.meshes,
    (m) => m.part,
    (id) => index.parts.has(id),
  );
  const visited = new Set<string>();

  const partNode = (id: string): TreeNode => {
    visited.add(id);
    const p = index.parts.get(id);
    const children = [
      ...(childParts.get(id) ?? []).filter((c) => !visited.has(c)).map(partNode),
      ...(deformersOf.get(id) ?? []).map((d) => deformerNode(index, d, [])),
      ...frontToBack(index, meshesOf.get(id) ?? []).map((m) => meshNode(index, m)),
    ];
    return {
      key: `part:${id}`,
      ref: { kind: "part", id },
      name: p?.name ?? id,
      icon: "part",
      hidden: partOpacity(index, id) === 0,
      maskOnly: false,
      children,
    };
  };

  const roots = (childParts.get(null) ?? []).map(partNode);
  // 循環したパーツ (検証で弾かれるはずだが) も表示する。
  for (const p of model.parts) if (!visited.has(p.id)) roots.push(partNode(p.id));
  // 所属パーツが見つからない要素。
  roots.push(...(deformersOf.get(null) ?? []).map((d) => deformerNode(index, d, [])));
  roots.push(...frontToBack(index, meshesOf.get(null) ?? []).map((m) => meshNode(index, m)));
  return roots;
}

export function buildDeformerTree(index: ModelIndex): TreeNode[] {
  const { model } = index;
  const known = (id: string) => index.deformers.has(id);
  const childDeformers = groupByParent(model.deformers, (d) => d.parent, known);
  const meshesOf = groupByParent(model.meshes, (m) => m.parent, known);
  const visited = new Set<string>();

  const node = (id: string): TreeNode => {
    visited.add(id);
    return deformerNode(index, id, [
      ...(childDeformers.get(id) ?? []).filter((c) => !visited.has(c)).map(node),
      ...frontToBack(index, meshesOf.get(id) ?? []).map((m) => meshNode(index, m)),
    ]);
  };

  const roots = (childDeformers.get(null) ?? []).map(node);
  for (const d of model.deformers) if (!visited.has(d.id)) roots.push(node(d.id));
  roots.push(...frontToBack(index, meshesOf.get(null) ?? []).map((m) => meshNode(index, m)));
  return roots;
}

export function buildTree(index: ModelIndex, view: TreeView): TreeNode[] {
  return view === "parts" ? buildPartTree(index) : buildDeformerTree(index);
}

/** 画面に並ぶ 1 行。 */
export interface TreeRow {
  node: TreeNode;
  depth: number;
  /** 親の節の key (最上位なら null)。 */
  parent: string | null;
}

/** 折りたたまれていない節を上から順に並べる。 */
export function flatten(roots: TreeNode[], collapsed: ReadonlySet<string>): TreeRow[] {
  const rows: TreeRow[] = [];
  const walk = (nodes: TreeNode[], depth: number, parent: string | null) => {
    for (const node of nodes) {
      rows.push({ node, depth, parent });
      if (node.children.length > 0 && !collapsed.has(node.key)) {
        walk(node.children, depth + 1, node.key);
      }
    }
  };
  walk(roots, 0, null);
  return rows;
}

/** `key` の節の祖先の key (近い順)。見つからなければ null。 */
export function pathTo(roots: TreeNode[], key: string): string[] | null {
  for (const node of roots) {
    if (node.key === key) return [];
    const sub = pathTo(node.children, key);
    if (sub) return [...sub, node.key];
  }
  return null;
}

/**
 * 要素の中のメッシュ (手前から奥へ)。メッシュはそれ自身、パーツは子孫のパーツを含めて属するメッシュ、
 * デフォーマは子孫のデフォーマを含めて親に持つメッシュ (どちらの表示の種類でも同じ)。
 * ツリーの行のホバーでビューポートに輪郭を出すのに使う (docs/editor.md#選択とホバーの見せ方)。
 */
export function meshesWithin(index: ModelIndex, ref: ElementRef): string[] {
  switch (ref.kind) {
    case "mesh":
      return index.meshes.has(ref.id) ? [ref.id] : [];
    case "part":
      return frontToBack(
        index,
        index.model.meshes
          .filter((m) => ancestors(index.parts, m.part).includes(ref.id))
          .map((m) => m.id),
      );
    case "deformer":
      return frontToBack(
        index,
        index.model.meshes
          .filter((m) => ancestors(index.deformers, m.parent).includes(ref.id))
          .map((m) => m.id),
      );
    case "parameter":
      return [];
  }
}

/**
 * `key` の節が見えていればその key、畳んだ祖先の下にあればいちばん近い見えている祖先の key。
 * ツリーに無ければ null。`visible` は今並んでいる行の key (`flatten` の結果)。
 */
export function nearestVisible(
  roots: TreeNode[],
  visible: ReadonlySet<string> | ReadonlyMap<string, unknown>,
  key: string,
): string | null {
  if (visible.has(key)) return key;
  return pathTo(roots, key)?.find((k) => visible.has(k)) ?? null;
}
