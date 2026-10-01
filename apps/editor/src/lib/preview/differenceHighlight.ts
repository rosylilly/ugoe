// 着せ替えの違いの行の要素を、選ぶ先とプレビューのビューポートでの強調 (T0657) の、DOM と wasm に依存しない部分
// (differenceHighlight.test.ts)。描くのは DifferenceOverlay.svelte。
import type { DifferenceItem } from "../ipc";
import type { ElementRef } from "../selection.svelte";
import type { Point } from "../viewport/tools/geometry";
import { editingName } from "./dressUp";

/**
 * 違いの要素を選ぶ先 (プレビューを出て選ぶ)。パーツ・デフォーマ・メッシュとグルー (両端のメッシュ) はツリー、
 * 物理のリグと衝突体は Physics タブ。
 */
export type DifferenceTarget =
  | { panel: "tree"; refs: ElementRef[] }
  | { panel: "physics"; rig: string | null; collider: string | null };

/**
 * 要素 `item` を選ぶ先。今の編集に無い要素 (除いたもの) は選べない (`null`)。グルーの両端は、作業用のモデルの ID
 * (`<モジュールの名前>/<ID>`) で `meshExists` なものだけを選ぶ。
 */
export function differenceTarget(
  item: DifferenceItem,
  moduleName: string,
  meshExists: (id: string) => boolean,
): DifferenceTarget | null {
  const element = item.element;
  if (element === null) return null;
  switch (item.kind) {
    case "part":
    case "deformer":
    case "mesh":
      return { panel: "tree", refs: [{ kind: item.kind, id: element }] };
    case "glue": {
      const refs = item.meshes
        .map((m) => `${moduleName}/${m}`)
        .filter(meshExists)
        .map((id): ElementRef => ({ kind: "mesh", id }));
      return refs.length > 0 ? { panel: "tree", refs } : null;
    }
    case "physics_rig":
      return { panel: "physics", rig: element, collider: null };
    case "collider":
      return { panel: "physics", rig: null, collider: element };
  }
}

/** 違いを強調するときにビューポートに描いている版: 今の編集 (`own`) かファイル。 */
export interface ShownVersion {
  /** WebPuppet の中のモジュールの名前 (ID の接頭辞 `<name>/`)。 */
  name: string;
  own: boolean;
}

/**
 * モジュール `label` の今の編集とファイルのうち、ビューポートに出ている方。比べていない (`comparing` でない) ときは
 * 作業用のモデルを描いているので今の編集 (接頭辞はモジュールの名前)。どちらも付いていなければ `null`。
 */
export function shownVersion(
  label: string,
  comparing: boolean,
  attached: readonly string[],
): ShownVersion | null {
  if (!comparing) return { name: label, own: true };
  const editing = editingName(label);
  if (attached.includes(editing)) return { name: editing, own: true };
  if (attached.includes(label)) return { name: label, own: false };
  return null;
}

/** 強調の知らせ (ビューポートの下に出す)。`found` は描けた要素があったか。 */
export function highlightNotice(
  item: Pick<DifferenceItem, "id" | "text">,
  shown: ShownVersion | null,
  found: boolean,
): string {
  if (shown === null) {
    return `Put on your current edits or the file to see ${item.id}.`;
  }
  const where = shown.own ? "your current edits" : "the file";
  if (found) return `${item.id} in ${where}`;
  if (shown.own && item.text === "removed") {
    return `${item.id} was removed in your current edits. Put on the file to see it.`;
  }
  if (!shown.own && item.text === "added") {
    return `${item.id} is not in the file. Put on your current edits to see it.`;
  }
  return `${item.id} can't be drawn in ${where}.`;
}

/** 点の凸包 (反時計回り。メッシュの三角形が分からないときの輪郭)。 */
export function convexHull(points: readonly Point[]): Point[] {
  const sorted = [...points].sort((a, b) => a[0] - b[0] || a[1] - b[1]);
  if (sorted.length < 3) return sorted;
  const cross = (o: Point, a: Point, b: Point) =>
    (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0]);
  const half = (list: Point[]): Point[] => {
    const out: Point[] = [];
    for (const p of list) {
      while (out.length >= 2) {
        const [a, b] = [out[out.length - 2], out[out.length - 1]] as [Point, Point];
        if (cross(a, b, p) > 0) break;
        out.pop();
      }
      out.push(p);
    }
    out.pop();
    return out;
  };
  return [...half(sorted), ...half([...sorted].reverse())];
}
