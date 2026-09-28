// 推定の操作と由来の表示の補助 (docs/editor.md#推定とミラー)。DOM や IPC に依存しない純粋な関数だけを置く
// (inference.test.ts で検査する)。コマンドは ugoe_editor_core::Command の JSON を組み立てるだけで、送るのは呼び出し側。
import type {
  Command,
  KeyformInference,
  KeyformTarget,
  MirrorReport,
  MirrorSource,
  RederiveWarning,
} from "../../../generated/types";
import { labelOf } from "../../../labels";
import type {
  Cell,
  ElementGrid,
  KeyformBinding,
  KeyformOrigin,
  ModelData,
} from "../keyforms/keyforms";

/** セルの由来ごとの表示 (格子の記号・名前・色)。色は明暗どちらのテーマでも読める中間の明るさ。 */
export interface OriginStyle {
  symbol: string;
  name: string;
  description: string;
  /** CSS の色。`authored` は既定の文字色。 */
  color: string;
}

export const ORIGIN_STYLES: Record<KeyformOrigin, OriginStyle> = {
  authored: {
    symbol: "A",
    name: "Authored",
    description: "made or fixed by hand; inference never overwrites it",
    color: "var(--text)",
  },
  additive: {
    symbol: "+",
    name: "Additive",
    description: "inferred by additive blending",
    color: "#0d9488",
  },
  depth_rotation: {
    symbol: "R",
    name: "Depth rotation",
    description: "inferred by rotating the depth",
    color: "#d97706",
  },
  mirror: {
    symbol: "M",
    name: "Mirror",
    description: "mirrored from the mirror source",
    color: "#2563eb",
  },
  derived: {
    symbol: "D",
    name: "Derived",
    description: "inferred without a recorded method; re-derived with the grid's method",
    color: "#a855f7",
  },
};

/** 凡例の並び。 */
export const LEGEND_ORIGINS: readonly KeyformOrigin[] = [
  "authored",
  "additive",
  "depth_rotation",
  "mirror",
  "derived",
];

export function originStyle(origin: KeyformOrigin): OriginStyle {
  return ORIGIN_STYLES[origin];
}

/** セルの説明 (ツールチップ用)。方法を記録していない `derived` は、作り直す方法も書く。 */
export function originTitle(
  origin: KeyformOrigin,
  inference: KeyformInference | null | undefined,
  mirrorSource?: MirrorSource | null,
): string {
  const style = originStyle(origin);
  if (origin === "derived") {
    return `${style.name}: re-derived by ${methodName(inference)}`;
  }
  if (origin === "mirror") {
    return mirrorSource
      ? `${style.name}: mirrored from ${mirrorSource.id}`
      : `${style.name}: no mirror source recorded (not re-derived)`;
  }
  return `${style.name}: ${style.description}`;
}

/** 格子の推定の方法の名前 (`inference` が省略なら加算合成)。 */
export function methodName(inference: KeyformInference | null | undefined): string {
  if (!inference || inference.method === "additive") return "additive blending";
  if (inference.method === "depth_rotation") return "depth rotation";
  return "an unknown method";
}

// ---- 推定の設定のフォーム ----

/** 推定の設定の編集中の値。数値は入力欄の値 (空や NaN もありうる)。 */
export interface InferenceForm {
  method: "additive" | "depth_rotation";
  angleScale: number;
  pivotX: number;
  pivotY: number;
  pivotDepth: number;
  /** 軸のパラメータ ID。空文字列ならその向きには回さない。 */
  yaw: string;
  pitch: string;
}

/**
 * 格子の設定からフォームを作る。深度からの回転でなければ、回転の既定値を入れておく
 * (yaw / pitch は格子の軸に AngleX / AngleY があればそれ、pivot は `pivot`)。
 */
export function inferenceForm(
  inference: KeyformInference | null | undefined,
  bindings: KeyformBinding[],
  pivot: [number, number] = [0, 0],
): InferenceForm {
  if (inference?.method === "depth_rotation") {
    return {
      method: "depth_rotation",
      angleScale: inference.angle_scale,
      pivotX: inference.pivot[0],
      pivotY: inference.pivot[1],
      pivotDepth: inference.pivot_depth,
      yaw: inference.yaw ?? "",
      pitch: inference.pitch ?? "",
    };
  }
  const bound = (id: string) => (bindings.some((b) => b.parameter === id) ? id : "");
  return {
    method: "additive",
    angleScale: 1,
    pivotX: pivot[0],
    pivotY: pivot[1],
    pivotDepth: 0,
    yaw: bound("AngleX"),
    pitch: bound("AngleY"),
  };
}

/** フォームの値の誤り (なければ null)。 */
export function formError(form: InferenceForm): string | null {
  if (form.method === "additive") return null;
  const numbers: [string, number][] = [
    ["Angle scale", form.angleScale],
    ["Pivot x", form.pivotX],
    ["Pivot y", form.pivotY],
    ["Pivot depth", form.pivotDepth],
  ];
  const bad = numbers.find(([, v]) => typeof v !== "number" || !Number.isFinite(v));
  if (bad) return `${bad[0]} must be a number`;
  if (!form.yaw && !form.pitch) return "Choose a yaw or pitch parameter";
  return null;
}

/** フォームの値の `KeyformInference`。加算合成は null (省略と同じ)。 */
export function inferenceFromForm(form: InferenceForm): KeyformInference | null {
  if (form.method === "additive") return null;
  return {
    method: "depth_rotation",
    angle_scale: form.angleScale,
    pivot: [form.pivotX, form.pivotY],
    pivot_depth: form.pivotDepth,
    ...(form.yaw ? { yaw: form.yaw } : {}),
    ...(form.pitch ? { pitch: form.pitch } : {}),
  };
}

/** 2 つの設定が同じか (加算合成は省略と同じ)。 */
export function sameInference(
  a: KeyformInference | null | undefined,
  b: KeyformInference | null | undefined,
): boolean {
  const norm = (i: KeyformInference | null | undefined) =>
    !i || i.method === "additive" ? null : i;
  return JSON.stringify(norm(a)) === JSON.stringify(norm(b));
}

// ---- コマンド ----

export function setInferenceCommand(
  target: KeyformTarget,
  inference: KeyformInference | null,
): Command {
  return { op: "set_keyform_inference", target, inference };
}

/** 格子の推定の設定で `derived` のセルを作り直す。`overwriteDerived` なら別の方法のセルも書き換える。 */
export function inferCommand(target: KeyformTarget, overwriteDerived: boolean): Command {
  return {
    op: "infer_keyforms",
    target,
    method: "from_grid",
    overwrite_derived: overwriteDerived,
  };
}

export interface MirrorForm {
  /** 対称軸 (空欄なら null: 元の要素の `mirror_axis`)。 */
  axis: number | null;
  /** 許容距離 (空欄ならキャンバスの 1px)。 */
  tolerance: number | null;
  overwriteAuthored: boolean;
}

/** 有限の数だけを残す (空欄は省略)。 */
function optionalNumber(value: number | null | undefined): number | undefined {
  return typeof value === "number" && Number.isFinite(value) ? value : undefined;
}

export function mirrorCommand(
  source: KeyformTarget,
  target: KeyformTarget,
  form: MirrorForm,
): Command {
  const axis = optionalNumber(form.axis);
  const tolerance = optionalNumber(form.tolerance);
  return {
    op: "mirror_keyforms",
    source,
    target,
    ...(axis !== undefined ? { axis } : {}),
    ...(tolerance !== undefined ? { tolerance } : {}),
    overwrite_authored: form.overwriteAuthored,
  };
}

export function markCommand(
  target: KeyformTarget,
  cells: Cell[],
  origin: "authored" | "derived",
): Command {
  return {
    op: origin === "authored" ? "mark_keyform_authored" : "mark_keyform_derived",
    target,
    cells,
  };
}

/** 左右の対を張る (`partner` が null なら外す)。`axis` が空欄なら親で決まる既定値。 */
export function setMirrorPairCommand(
  target: KeyformTarget,
  partner: string | null,
  axis: number | null,
): Command {
  const a = optionalNumber(axis);
  return {
    op: "set_mirror_pair",
    target,
    ...(partner ? { mirror: partner } : {}),
    ...(a !== undefined ? { axis: a } : {}),
  };
}

// ---- ミラーの元と対 ----

/** 要素の左右の対と対称軸 (メッシュ・デフォーマだけ)。対を持てない要素なら null。 */
export function mirrorPair(
  model: ModelData,
  target: KeyformTarget,
): { mirror: string | null; axis: number | null } | null {
  const item =
    target.kind === "mesh"
      ? model.meshes.find((m) => m.id === target.id)
      : target.kind === "deformer"
        ? model.deformers.find((d) => d.id === target.id)
        : undefined;
  if (!item) return null;
  return { mirror: item.mirror ?? null, axis: item.mirror_axis ?? null };
}

export interface MirrorChoice {
  source: KeyformTarget;
  label: string;
}

/** 要素と同じ種類の要素の一覧 (メッシュかデフォーマ。それ以外は空)。 */
function sameKind(model: ModelData, target: KeyformTarget): readonly { id: string }[] {
  if (target.kind === "mesh") return model.meshes;
  if (target.kind === "deformer") return model.deformers;
  return [];
}

/** 要素の表記 (表示名と ID)。 */
export function targetName(model: ModelData, target: KeyformTarget): string {
  return labelOf(sameKind(model, target), target.id);
}

/**
 * 「Mirror from…」の元の候補: 対の相手 (あれば) と自分自身 (左右対称な 1 つの要素)。
 * ミラーできない要素 (パーツ・グルー) なら空。表示は名前と ID。
 */
export function mirrorChoices(model: ModelData, element: ElementGrid): MirrorChoice[] {
  const pair = mirrorPair(model, element.target);
  if (!pair) return [];
  const { kind, id } = element.target;
  const items = sameKind(model, element.target);
  const self: MirrorChoice = {
    source: element.target,
    label: `${labelOf(items, id)} (itself, left ↔ right)`,
  };
  if (!pair.mirror || pair.mirror === id) return [self];
  const partner = { kind, id: pair.mirror } as KeyformTarget;
  return [{ source: partner, label: `${labelOf(items, pair.mirror)} (mirror partner)` }, self];
}

/** 対の相手の候補 (同じ種類の、自分以外の要素)。表示は名前と ID。 */
export function partnerCandidates(
  model: ModelData,
  target: KeyformTarget,
): { id: string; label: string }[] {
  const items = sameKind(model, target);
  return items
    .filter((item) => item.id !== target.id)
    .map((item) => ({ id: item.id, label: labelOf(items, item.id) }));
}

/** ミラー元の記録の説明。`label` は要素の ID から表記を作る (省略なら ID)。 */
export function describeMirrorSource(
  source: MirrorSource,
  self: string,
  label: (id: string) => string = (id) => id,
): string {
  const options = [
    source.axis != null ? `axis ${source.axis}` : null,
    source.tolerance != null ? `tolerance ${source.tolerance}` : null,
  ].filter((s) => s !== null);
  const from = source.id === self ? `${label(source.id)} itself` : label(source.id);
  return `Mirrored cells come from ${from}${options.length ? ` (${options.join(", ")})` : ""}.`;
}

function cells(n: number): string {
  return `${n} cell${n === 1 ? "" : "s"}`;
}

/**
 * 「Mirror」の結果の説明 (docs/editor.md#推定とミラー)。1 つも書かなかったときは理由を書き、
 * `overwrite` を真にする (authored のセルを上書きしてやり直す操作を出す)。
 */
export function describeMirrorReport(
  report: MirrorReport,
  label: (target: KeyformTarget) => string,
): { text: string; overwrite: boolean } {
  const from = label(report.source);
  if (report.written > 0) {
    const kept =
      report.kept_authored > 0
        ? ` Kept ${cells(report.kept_authored)} authored (not overwritten).`
        : "";
    return { text: `Mirrored ${cells(report.written)} from ${from}.${kept}`, overwrite: false };
  }
  if (report.kept_authored > 0) {
    return {
      text:
        `Nothing was mirrored: all ${cells(report.kept_authored)} that ${from} maps to are ` +
        "authored, and authored cells are kept unless overwritten.",
      overwrite: true,
    };
  }
  return {
    text: `Nothing was mirrored: no cell of ${from} maps to a cell of this grid that can be written.`,
    overwrite: false,
  };
}

// ---- セルの選択 ----

/** セルを比べるための文字列 (軸の順によらない)。 */
export function cellKey(cell: Cell): string {
  return JSON.stringify(Object.entries(cell).sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0)));
}

/** `cells` に `cell` がなければ加え、あれば除いた一覧。 */
export function toggleCell(cells: Cell[], cell: Cell): Cell[] {
  const key = cellKey(cell);
  return cells.some((c) => cellKey(c) === key)
    ? cells.filter((c) => cellKey(c) !== key)
    : [...cells, cell];
}

export function targetKey(target: KeyformTarget): string {
  return `${target.kind}:${target.id}`;
}

// ---- 自動の作り直しの警告 ----

function targetLabel(target: KeyformTarget): string {
  return `${target.kind} ${target.id}`;
}

/** 警告の文 (通知に出す)。 */
export function describeWarning(warning: RederiveWarning): string {
  return (
    `Could not re-mirror ${targetLabel(warning.target)} from ${targetLabel(warning.source)}: ` +
    `${warning.message}. Its mirrored cells keep their previous values.`
  );
}

/** 通知に積む警告 (同じ格子とミラー元の組は新しいもので置き換える)。 */
export function mergeWarnings(
  current: RederiveWarning[],
  incoming: RederiveWarning[],
): RederiveWarning[] {
  const key = (w: RederiveWarning) => `${targetKey(w.target)}<-${targetKey(w.source)}`;
  const replaced = new Set(incoming.map(key));
  return [...current.filter((w) => !replaced.has(key(w))), ...incoming];
}
