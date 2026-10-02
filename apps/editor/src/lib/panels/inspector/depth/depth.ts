// 深度の編集の補助 (docs/editor.md#深度の編集)。DOM や IPC に依存しない純粋な関数だけを置く
// (depth.test.ts で検査する)。深度の値を作るのはバックエンドのプリセット (apply_depth_preset) で、
// ここではコマンドを組み立て、表示用に深度を読む。
import type { ChildRef, Command, DepthPreset, Model } from "../../../generated/types";
import { t } from "../../../i18n";
import type { ElementRef } from "../../../selection.svelte";

/** 深度を持てる要素 (メッシュかワープデフォーマ)。 */
export interface DepthElement {
  target: ChildRef;
  kind: "mesh" | "warp";
  /** 点 (頂点・制御点) の数。 */
  points: number;
  /** 今の深度 (なければ null)。 */
  depth: number[] | null;
}

/** 選択した要素の深度。メッシュとワープデフォーマ以外なら null。 */
export function depthElement(model: Model, ref: ElementRef | null): DepthElement | null {
  if (ref?.kind === "mesh") {
    const mesh = model.meshes.find((m) => m.id === ref.id);
    if (!mesh) return null;
    return {
      target: { kind: "mesh", id: ref.id },
      kind: "mesh",
      points: mesh.uvs.length,
      depth: mesh.depth ?? null,
    };
  }
  if (ref?.kind === "deformer") {
    const kind = model.deformers.find((d) => d.id === ref.id)?.kind;
    if (kind?.type !== "warp") return null;
    return {
      target: { kind: "deformer", id: ref.id },
      kind: "warp",
      points: (kind.rows + 1) * (kind.cols + 1),
      depth: kind.depth ?? null,
    };
  }
  return null;
}

/** 深度の範囲 (表示用)。深度がなければ null。 */
export function depthRange(depth: readonly number[] | null): { min: number; max: number } | null {
  if (!depth || depth.length === 0) return null;
  return { min: Math.min(...depth), max: Math.max(...depth) };
}

/** 深度の範囲の説明 ("none"、"0 – 12 px" など)。 */
export function describeDepth(depth: readonly number[] | null): string {
  const range = depthRange(depth);
  if (!range) return t("inspector.depth.none");
  const f = (v: number) => String(Math.round(v * 100) / 100);
  return range.min === range.max
    ? t("inspector.depth.value", { value: f(range.min) })
    : t("inspector.depth.range", { min: f(range.min), max: f(range.max) });
}

export function setDepthCommand(target: ChildRef, depth: number[] | null): Command {
  return depth ? { op: "set_depth", target, depth } : { op: "set_depth", target };
}

export function presetCommand(target: ChildRef, preset: DepthPreset): Command {
  return { op: "apply_depth_preset", target, preset };
}

/** 選んだ点の深度を `value` にするコマンド。点の番号は範囲内のものだけを重複なく昇順で送る。 */
export function pointsCommand(
  element: DepthElement,
  indices: readonly number[],
  value: number,
): Command | null {
  const valid = [...new Set(indices)]
    .filter((i) => Number.isInteger(i) && i >= 0 && i < element.points)
    .sort((a, b) => a - b);
  if (valid.length === 0) return null;
  return presetCommand(element.target, { kind: "points", indices: valid, value });
}

/** 数の入力欄の値 (bind:value は空なら null) を有限の数にする。そうでなければ null。 */
export function finiteValue(value: number | null | undefined): number | null {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

/** 色分けの尺度: 深度の絶対値の最大 (すべて 0 なら 1)。 */
export function depthScale(depth: readonly number[]): number {
  const max = depth.reduce((m, d) => Math.max(m, Math.abs(d)), 0);
  return max > 0 ? max : 1;
}

/**
 * 深度の色 (ビューポートの点)。0 は灰色、手前 (正) ほど赤く、奥 (負) ほど青くする。
 * `scale` は色が飽和する深度の絶対値 (depthScale)。
 */
export function depthColor(depth: number, scale: number): string {
  const t = Math.max(-1, Math.min(1, scale > 0 ? depth / scale : 0));
  const gray = 160;
  const mix = (to: number) => Math.round(gray + (to - gray) * Math.abs(t));
  return t >= 0
    ? `rgb(${mix(235)}, ${mix(64)}, ${mix(52)})`
    : `rgb(${mix(52)}, ${mix(110)}, ${mix(235)})`;
}
