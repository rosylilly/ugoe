// 親のワープの格子からのはみ出し (docs/editor.md#はみ出しの検証) の表示用の純粋な関数。
// 検出はバックエンド (ugoe_editor_core::overflow) が行い、ここは一覧とビューポートの表示を組み立てるだけ。

import { t } from "../../i18n";
import type {
  ElementOverflow,
  KeyformTarget,
  Model,
  OverflowFinding,
  OverflowPointKind,
  OverflowReport,
} from "../../ipc";
import { labelOf } from "../../labels";
import type { ElementRef } from "../../selection.svelte";

/** 1 つの要素のはみ出し (一覧の 1 項目)。セルは量の大きい順。 */
export interface OverflowGroup {
  key: string;
  target: KeyformTarget;
  parent: string;
  kind: OverflowPointKind;
  findings: OverflowFinding[];
  /** セルの中で最大の量。 */
  maxAmount: number;
}

export function targetKey(target: KeyformTarget): string {
  return `${target.kind}:${target.id}`;
}

/** はみ出しを要素ごとにまとめる。要素は最大の量の大きい順 (同じなら報告の順)。 */
export function groupFindings(report: OverflowReport): OverflowGroup[] {
  const groups = new Map<string, OverflowGroup>();
  for (const f of report.findings) {
    const key = targetKey(f.target);
    let group = groups.get(key);
    if (!group) {
      group = {
        key,
        target: f.target,
        parent: f.parent,
        kind: f.kind,
        findings: [],
        maxAmount: 0,
      };
      groups.set(key, group);
    }
    group.findings.push(f);
    group.maxAmount = Math.max(group.maxAmount, f.max_amount);
  }
  const list = [...groups.values()];
  for (const g of list) g.findings.sort((a, b) => b.max_amount - a.max_amount);
  return list.sort((a, b) => b.maxAmount - a.maxAmount);
}

/** はみ出しの量 (格子の幅・高さを 1 とした UV の距離) を格子の大きさに対する百分率で書く。 */
export function formatAmount(amount: number): string {
  const percent = amount * 100;
  return `${percent < 10 ? percent.toFixed(1) : percent.toFixed(0)}%`;
}

/** セル (軸のパラメータの値) の表記。軸がなければ「(no axes)」。 */
export function formatCell(cell: Partial<Record<string, number>>, model: Model | null): string {
  const entries = Object.entries(cell).filter(
    (e): e is [string, number] => typeof e[1] === "number",
  );
  if (entries.length === 0) return t("verify.overflow.noAxes");
  return entries
    .map(([id, value]) => `${labelOf(model?.parameters, id)} = ${Number(value.toFixed(3))}`)
    .join(", ");
}

/** 「3 vertices」のような点の数の表記。 */
export function formatCount(kind: OverflowPointKind, count: number): string {
  switch (kind) {
    case "vertex":
      return t("verify.counts.vertex", { count });
    case "control_point":
      return t("verify.counts.controlPoint", { count });
    case "origin":
      return t("verify.counts.origin", { count });
  }
}

/** 要素の表記 (名前と ID)。 */
export function targetLabel(target: KeyformTarget, model: Model | null): string {
  const items = target.kind === "mesh" ? model?.meshes : model?.deformers;
  return labelOf(items, target.id);
}

/** はみ出した要素の選択 (ツリー・インスペクタと同じ選択)。部品・グルーは来ない。 */
export function targetRef(target: KeyformTarget): ElementRef | null {
  if (target.kind === "mesh" || target.kind === "deformer")
    return { kind: target.kind, id: target.id };
  return null;
}

/** セルへ移るときにビューポートに設定するパラメータの値 (ID と値)。 */
export function cellParameters(cell: Partial<Record<string, number>>): [string, number][] {
  return Object.entries(cell).filter((e): e is [string, number] => typeof e[1] === "number");
}

/** ビューポートに描くはみ出した点 (CSS ピクセル)。 */
export interface OverflowDot {
  key: string;
  x: number;
  y: number;
  title: string;
}

/**
 * はみ出した点の画面上の位置。`positions` は要素の点の評価済みの位置 (CSS ピクセル、[x0, y0, ...]。
 * メッシュは頂点、ワープは制御点、回転デフォーマは原点の 1 点)。読めない要素 (まだ評価していない) は飛ばす。
 */
export function overflowDots(
  current: readonly ElementOverflow[],
  positions: (target: KeyformTarget) => ArrayLike<number> | null,
  model: Model | null,
): OverflowDot[] {
  return current.flatMap((element) => {
    const flat = positions(element.target);
    if (!flat) return [];
    const label = targetLabel(element.target, model);
    return element.points.flatMap((p) => {
      const x = flat[2 * p.index];
      const y = flat[2 * p.index + 1];
      if (x === undefined || y === undefined) return [];
      const point = element.kind === "origin" ? t("verify.overflow.origin") : `#${p.index}`;
      return [
        {
          key: `${targetKey(element.target)}#${p.index}`,
          x,
          y,
          title: t("verify.overflow.dot", {
            element: label,
            point,
            amount: formatAmount(p.amount),
            parent: element.parent,
          }),
        },
      ];
    });
  });
}
