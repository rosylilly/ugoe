// 着せ替えの違いの行の要素を、選ぶ先とプレビューのビューポートでの強調 (T0657) の、DOM と wasm に依存しない部分
// (differenceHighlight.test.ts)。描くのは DifferenceOverlay.svelte。
import { t } from "../i18n";
import type { DifferenceItem } from "../ipc";
import type { ElementRef } from "../selection.svelte";
import type { Point } from "../viewport/tools/geometry";
import { editingName } from "./dressUp";

/**
 * 違いの要素を選ぶ先 (プレビューを出て選ぶ)。パーツ・デフォーマ・メッシュはツリー、グルーは Glue タブ (両端のメッシュを
 * ツリーでも選ぶ)、物理のリグと衝突体は Physics タブ、ポーズのグループと追従は Poses タブ (パーツをツリーでも選ぶ。T0659)。
 */
export type DifferenceTarget =
  | { panel: "tree"; refs: ElementRef[] }
  | { panel: "glue"; glue: string; refs: ElementRef[] }
  | { panel: "poses"; refs: ElementRef[] }
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
      return { panel: "glue", glue: element, refs };
    }
    case "physics_rig":
      return { panel: "physics", rig: element, collider: null };
    case "collider":
      return { panel: "physics", rig: null, collider: element };
    case "pose_group":
    case "pose_follow":
      return { panel: "poses", refs: item.parts.map((id): ElementRef => ({ kind: "part", id })) };
  }
}

/** 違いの要素を選ぶボタンの説明。 */
export function selectTitle(item: Pick<DifferenceItem, "kind" | "id">): string {
  const id = item.id;
  switch (item.kind) {
    case "physics_rig":
    case "collider":
      return t("preview.difference.select.physics", { id });
    case "glue":
      return t("preview.difference.select.glue", { id });
    case "pose_group":
    case "pose_follow":
      return t("preview.difference.select.poses", { id });
    default:
      return t("preview.difference.select.tree", { id });
  }
}

/** ビューポートを要素に寄せる動き (`ViewportController.zoomAt` してから `panBy`)。 */
export interface FitMove {
  /** ズームの倍率 (今のズームに掛ける)。 */
  factor: number;
  /** ズームの中心 (CSS ピクセル。要素の範囲の中心)。 */
  center: Point;
  /** ズームの後にずらす量 (中心をビューポートの中央へ)。 */
  pan: Point;
}

/**
 * 要素を寄せたときに、要素の範囲がビューポートの幅・高さに対して占める割合の上限 (縦横のうち先に届く方で決まる)。
 * 周りも見えるように半分にする (T1671。0.8 では細長い袖が画面いっぱいになり、どこの袖か分からなかった)。
 */
const FIT_FILL = 0.5;
/** 寄せるときの範囲の最小の大きさ (CSS ピクセル。点や細い要素で寄りすぎないように)。 */
const FIT_MIN_SIZE = 24;

/**
 * 画面の点 `points` (CSS ピクセル) を、大きさ `width`×`height` のビューポートの中央に、範囲の `FIT_FILL` の大きさで
 * 映す動き。点が無ければ `null`。
 */
export function fitMove(points: readonly Point[], width: number, height: number): FitMove | null {
  if (points.length === 0 || width <= 0 || height <= 0) return null;
  // Math.min(...xs) は引数の数の上限で止まるので、1 つずつ見る (頂点の多いメッシュ)。
  let [minX, maxX, minY, maxY] = [
    Number.POSITIVE_INFINITY,
    Number.NEGATIVE_INFINITY,
    Number.POSITIVE_INFINITY,
    Number.NEGATIVE_INFINITY,
  ];
  for (const [x, y] of points) {
    minX = Math.min(minX, x);
    maxX = Math.max(maxX, x);
    minY = Math.min(minY, y);
    maxY = Math.max(maxY, y);
  }
  if (![minX, maxX, minY, maxY].every(Number.isFinite)) return null;
  const w = Math.max(maxX - minX, FIT_MIN_SIZE);
  const h = Math.max(maxY - minY, FIT_MIN_SIZE);
  const factor = Math.min((width * FIT_FILL) / w, (height * FIT_FILL) / h);
  const center: Point = [(minX + maxX) / 2, (minY + maxY) / 2];
  return { factor, center, pan: [width / 2 - center[0], height / 2 - center[1]] };
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

/**
 * 要素が、描いている版 `shown` に無いか (今の編集で除いた要素か、ファイルに無い足した要素)。メッシュがその版に
 * 残っていれば輪郭は描けるが、要素そのものではない (T0661)。
 */
export function missingIn(item: Pick<DifferenceItem, "text">, shown: ShownVersion | null): boolean {
  if (shown === null) return false;
  return shown.own ? item.text === "removed" : item.text === "added";
}

/** 強調の知らせ (ビューポートの下に出す)。`found` は描けた要素があったか。 */
export function highlightNotice(
  item: Pick<DifferenceItem, "id" | "text">,
  shown: ShownVersion | null,
  found: boolean,
): string {
  const id = item.id;
  if (shown === null) return t("preview.difference.notice.notShown", { id });
  // 除いた (足した) 要素は、メッシュがこの版にも残っていれば輪郭を描けるが、要素そのものは無い。描けても
  // 「Removed」(「Added」) と分かるようにする (T0661)。
  if (missingIn(item, shown)) {
    if (!found) {
      return shown.own
        ? t("preview.difference.notice.removedNotDrawn", { id })
        : t("preview.difference.notice.notInFile", { id });
    }
    return shown.own
      ? t("preview.difference.notice.removedOutline", { id })
      : t("preview.difference.notice.addedOutline", { id });
  }
  if (shown.own) {
    return found
      ? t("preview.difference.notice.inEdits", { id })
      : t("preview.difference.notice.notDrawnInEdits", { id });
  }
  return found
    ? t("preview.difference.notice.inFile", { id })
    : t("preview.difference.notice.notDrawnInFile", { id });
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
