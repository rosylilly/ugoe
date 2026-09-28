// キーフォーム格子を読む補助 (docs/editor.md#キーフォームの編集)。DOM や IPC に依存しない純粋な関数だけを置く
// (keyforms.test.ts で検査する)。モデルの JSON (ugoe_model::Model) のうち、ここで使う部分だけを型にする。
import type { KeyformInference, KeyformTarget, MirrorSource } from "../../../generated/types";
import type { ElementRef } from "../../../selection.svelte";

export type { KeyformTarget };

export type Point = [number, number];

export interface KeyformBinding {
  parameter: string;
  keys: number[];
}

/**
 * セルの由来の表示用の分類。`derived` は方法の記録のない推定、`additive` / `depth_rotation` は方法を記録した推定、
 * `mirror` はミラーで作ったセル (ugoe_model::KeyformOrigin、docs/spec/keyform-inference.md#セルの由来)。
 */
export type KeyformOrigin = "authored" | "derived" | "additive" | "depth_rotation" | "mirror";

const DERIVED_ORIGINS: readonly string[] = ["derived", "additive", "depth_rotation", "mirror"];

export interface KeyformGrid<T = Record<string, unknown>> {
  bindings?: KeyformBinding[];
  keyforms: T[];
  /** 空または省略ならすべて authored。 */
  origins?: string[];
  /** 方法を記録していない derived のセルを作り直す設定。省略なら加算合成。 */
  inference?: KeyformInference | null;
  /** mirror のセルのミラー元。 */
  mirror_source?: MirrorSource | null;
}

export interface MeshKeyformData {
  positions: Point[];
  [field: string]: unknown;
}

export interface WarpKeyformData {
  points: Point[];
  [field: string]: unknown;
}

export interface RotationKeyformData {
  origin: Point;
  angle: number;
  [field: string]: unknown;
}

export interface ParameterData {
  id: string;
  name: string;
  min: number;
  max: number;
  default: number;
}

/** 左右の対 (メッシュ・デフォーマ)。 */
interface MirrorFields {
  mirror?: string | null;
  mirror_axis?: number | null;
}

interface DeformerData extends MirrorFields {
  id: string;
  parent?: string | null;
  kind: {
    type: string;
    rows?: number;
    cols?: number;
    keyforms?: KeyformGrid;
  };
}

export interface ModelData {
  parameters: ParameterData[];
  parts: { id: string; keyforms: KeyformGrid }[];
  deformers: DeformerData[];
  meshes: ({
    id: string;
    parent?: string | null;
    indices: number[];
    keyforms: KeyformGrid;
  } & MirrorFields)[];
}

/** 格子のキーフォームの種類 (ugoe_editor_core::Keyform の kind)。 */
export type KeyformKind = "part" | "warp" | "rotation" | "mesh";

/** 要素の格子と、その種類。 */
export interface ElementGrid {
  target: KeyformTarget;
  kind: KeyformKind;
  grid: KeyformGrid;
  /** ワープの行・列の数 (ワープだけ)。 */
  rows?: number;
  cols?: number;
  /** メッシュの三角形 (メッシュだけ)。 */
  indices?: number[];
}

/** 軸のパラメータ ID → キーの値。ugoe_editor_core::KeyformCell。 */
export type Cell = Record<string, number>;

const parsed = new Map<string, ModelData>();

/** モデルの JSON を読む。同じ文字列は 1 度だけ解析する (直近の 1 つだけを覚える)。 */
export function parseModel(json: string): ModelData {
  const hit = parsed.get(json);
  if (hit) return hit;
  const model = JSON.parse(json) as ModelData;
  parsed.clear();
  parsed.set(json, model);
  return model;
}

/** 選択した要素のキーフォーム格子。格子を持たない要素 (パラメータ・未知のデフォーマ) なら null。 */
export function elementGrid(model: ModelData, ref: ElementRef | null): ElementGrid | null {
  if (!ref) return null;
  switch (ref.kind) {
    case "part": {
      const part = model.parts.find((p) => p.id === ref.id);
      return part
        ? { target: { kind: "part", id: ref.id }, kind: "part", grid: part.keyforms }
        : null;
    }
    case "mesh": {
      const mesh = model.meshes.find((m) => m.id === ref.id);
      return mesh
        ? {
            target: { kind: "mesh", id: ref.id },
            kind: "mesh",
            grid: mesh.keyforms,
            indices: mesh.indices,
          }
        : null;
    }
    case "deformer": {
      const d = model.deformers.find((d) => d.id === ref.id);
      if (!d?.kind.keyforms) return null;
      const target: KeyformTarget = { kind: "deformer", id: ref.id };
      if (d.kind.type === "warp") {
        return {
          target,
          kind: "warp",
          grid: d.kind.keyforms,
          rows: d.kind.rows,
          cols: d.kind.cols,
        };
      }
      if (d.kind.type === "rotation") return { target, kind: "rotation", grid: d.kind.keyforms };
      return null;
    }
    default:
      return null;
  }
}

/** 値がキーと同じとみなす許容差。スライダーの値は f32 を経由するので、完全一致では比べない。 */
export function keyTolerance(param: Pick<ParameterData, "min" | "max"> | undefined): number {
  const span = param ? Math.abs(param.max - param.min) : 1;
  return 1e-6 * Math.max(1, span);
}

/** `value` と同じとみなせるキー。なければ undefined。 */
export function keyAt(keys: number[], value: number, tolerance: number): number | undefined {
  return keys.find((k) => Math.abs(k - value) <= tolerance);
}

/** `value` に最も近いキー (等距離なら小さい方)。キーがなければ undefined。 */
export function nearestKey(keys: number[], value: number): number | undefined {
  let best: number | undefined;
  for (const k of keys) {
    if (best === undefined || Math.abs(k - value) < Math.abs(best - value)) best = k;
  }
  return best;
}

/** パラメータの現在値 (表示の状態に無ければ既定値)。 */
export function parameterValue(
  id: string,
  values: Record<string, number>,
  params: ParameterData[],
): number {
  return values[id] ?? params.find((p) => p.id === id)?.default ?? 0;
}

/**
 * 今のパラメータ値がすべての軸でキーの上にあれば、そのセル。どれかの軸がキーの上になければ null。
 * 軸のない格子は常に唯一のセル `{}`。
 */
export function currentCell(
  bindings: KeyformBinding[],
  values: Record<string, number>,
  params: ParameterData[],
): Cell | null {
  const cell: Cell = {};
  for (const b of bindings) {
    const param = params.find((p) => p.id === b.parameter);
    const key = keyAt(b.keys, parameterValue(b.parameter, values, params), keyTolerance(param));
    if (key === undefined) return null;
    cell[b.parameter] = key;
  }
  return cell;
}

/** すべての軸を最も近いキーに合わせた値 (軸のパラメータ ID → キー)。 */
export function snapToKeys(
  bindings: KeyformBinding[],
  values: Record<string, number>,
  params: ParameterData[],
): Cell {
  const out: Cell = {};
  for (const b of bindings) {
    const key = nearestKey(b.keys, parameterValue(b.parameter, values, params));
    if (key !== undefined) out[b.parameter] = key;
  }
  return out;
}

/** セルの線形インデックス (軸 0 が最も速く変わる)。キーが無ければ -1。 */
export function cellIndex(bindings: KeyformBinding[], cell: Cell): number {
  let index = 0;
  let stride = 1;
  for (const b of bindings) {
    const value = cell[b.parameter];
    const i = value === undefined ? -1 : b.keys.indexOf(value);
    if (i < 0) return -1;
    index += i * stride;
    stride *= b.keys.length;
  }
  return index;
}

/** 線形インデックスのセル。 */
export function cellAt(bindings: KeyformBinding[], index: number): Cell {
  const cell: Cell = {};
  let rest = index;
  for (const b of bindings) {
    const n = Math.max(1, b.keys.length);
    cell[b.parameter] = b.keys[rest % n] ?? 0;
    rest = Math.floor(rest / n);
  }
  return cell;
}

/** セルの由来。origins が空または未知の値なら authored。 */
export function cellOrigin(grid: KeyformGrid, index: number): KeyformOrigin {
  const origin = grid.origins?.[index];
  return origin !== undefined && DERIVED_ORIGINS.includes(origin)
    ? (origin as KeyformOrigin)
    : "authored";
}

/** 推定で作ったセル (手で編集していないセル) か。 */
export function isDerived(origin: KeyformOrigin): boolean {
  return origin !== "authored";
}

/** キーフォームのうち、ビューポートで動かせる点 (親の座標系)。パーツには無い。 */
export function editablePoints(kind: KeyformKind, keyform: unknown): Point[] {
  switch (kind) {
    case "mesh":
      return (keyform as MeshKeyformData).positions;
    case "warp":
      return (keyform as WarpKeyformData).points;
    case "rotation":
      return [(keyform as RotationKeyformData).origin];
    default:
      return [];
  }
}

/** `editablePoints` の `index` 番目を `point` にしたキーフォーム (元は変えない)。 */
export function withPoint(
  kind: KeyformKind,
  keyform: Record<string, unknown>,
  index: number,
  point: Point,
): Record<string, unknown> {
  switch (kind) {
    case "mesh":
    case "warp": {
      const field = kind === "mesh" ? "positions" : "points";
      const points = [...(keyform[field] as Point[])];
      points[index] = point;
      return { ...keyform, [field]: points };
    }
    case "rotation":
      return { ...keyform, origin: point };
    default:
      return keyform;
  }
}
