// 着せ替えのスロットの編集 (docs/editor.md#スロットの編集) の補助。DOM や IPC に依存しない
// 純粋な関数だけを置く (slots.test.ts で検査する)。値の検査はコマンド (ugoe_editor_core::slot_edit) と
// 適用後の検証が行う。
import type { Command, DrawOrderRange, Slot } from "../../generated/types";
import { frontToBack, type ModelIndex, meshDrawOrder } from "../tree/build";

/** モジュールの描画順の範囲 (0〜1000。docs/spec/modules.md#合成)。スロットの範囲もふつうはこの中に取る。 */
export const DRAW_ORDER_MIN = 0;
export const DRAW_ORDER_MAX = 1000;
/** 新しいスロットの範囲の幅の既定値。 */
export const DEFAULT_WIDTH = 50;

/** 省略できるフィールドを埋めたスロット (画面で扱いやすく、`set_slot` にそのまま送れる)。 */
export type FullSlot = Required<Slot>;

export function fullSlot(slot: Slot): FullSlot {
  return {
    id: slot.id,
    anchor: slot.anchor ?? null,
    draw_order: { ...slot.draw_order },
    replaces: slot.replaces ?? [],
    parameters: slot.parameters ?? [],
    mask_sources: slot.mask_sources ?? [],
    part: slot.part ?? null,
    colliders: slot.colliders ?? [],
    pose_parts: slot.pose_parts ?? [],
  };
}

/** 描画順の範囲が重なるか (ugoe_editor_core::slot_edit::ranges_overlap と同じ)。端が接するだけは重ならない。 */
export function rangesOverlap(a: DrawOrderRange, b: DrawOrderRange): boolean {
  const lo = Math.max(a.min, b.min);
  const hi = Math.min(a.max, b.max);
  return lo < hi || (lo === hi && (a.min === a.max || b.min === b.max));
}

/** `slot` と描画順の範囲が重なるほかのスロットの ID。 */
export function overlapping(slot: Slot, slots: readonly Slot[]): string[] {
  return slots
    .filter((s) => s.id !== slot.id && rangesOverlap(slot.draw_order, s.draw_order))
    .map((s) => s.id);
}

/** ID の形式 (`[A-Za-z0-9_]+`、docs/spec/model.md#ID)。 */
export function isValidSlotId(id: string): boolean {
  return /^[A-Za-z0-9_]+$/.test(id);
}

/** 既存のスロットと重ならない ID (`Slot`、`Slot_2`、… ugoe_editor_core::ids::unique_id と同じ規則)。 */
export function newSlotId(slots: readonly Slot[], base = "Slot"): string {
  const taken = new Set(slots.map((s) => s.id));
  if (!taken.has(base)) return base;
  for (let n = 2; ; n++) {
    const id = `${base}_${n}`;
    if (!taken.has(id)) return id;
  }
}

/**
 * ほかのスロットと重ならない描画順の範囲。0〜1000 の中の空いている区間のうち、最も手前 (大きい値) の区間から
 * 幅 `width` (入りきらなければ区間の幅) を取る。空きがなければ null。
 */
export function freeRange(slots: readonly Slot[], width = DEFAULT_WIDTH): DrawOrderRange | null {
  const taken = slots
    .map((s) => s.draw_order)
    .filter((r) => r.max > DRAW_ORDER_MIN && r.min < DRAW_ORDER_MAX)
    .sort((a, b) => b.max - a.max);
  let top = DRAW_ORDER_MAX;
  const gaps: DrawOrderRange[] = [];
  for (const r of taken) {
    if (r.max < top) gaps.push({ min: r.max, max: top });
    top = Math.min(top, r.min);
  }
  if (top > DRAW_ORDER_MIN) gaps.push({ min: DRAW_ORDER_MIN, max: top });
  const gap = gaps.find((g) => g.max - g.min >= width) ?? gaps[0];
  if (!gap) return null;
  const w = Math.min(width, gap.max - gap.min);
  return { min: gap.max - w, max: gap.max };
}

/** 新しいスロット (取り付け先なし・空いている範囲)。空きがなければ null。 */
export function newSlot(slots: readonly Slot[]): FullSlot | null {
  const range = freeRange(slots);
  if (!range) return null;
  return fullSlot({ id: newSlotId(slots), draw_order: range });
}

/** スロットの一部を変える `set_slot`。変わらなければ null。 */
export function setSlotCommand(slot: Slot, patch: Partial<Slot>): Command | null {
  const before = fullSlot(slot);
  const after = fullSlot({ ...before, ...patch });
  if (JSON.stringify(before) === JSON.stringify(after)) return null;
  return { op: "set_slot", id: slot.id, slot: after };
}

/** `list` に `id` を入れる・外す (並びは保つ。入れるときは末尾)。 */
export function toggled(list: readonly string[], id: string, on: boolean): string[] {
  const without = list.filter((x) => x !== id);
  return on ? [...without, id] : without;
}

/**
 * アバタープロファイルの `slots` (アプリに見せるスロット) にスロットを入れる・外す `update_avatar_profile`。
 * プロファイルがなければ (`exposed` が null) 作らないように null を返す。
 */
export function exposeCommand(
  exposed: readonly string[] | null,
  id: string,
  on: boolean,
): Command | null {
  if (exposed === null || exposed.includes(id) === on) return null;
  return { op: "update_avatar_profile", slots: toggled(exposed, id, on) };
}

/** 数の欄の値 (有限でなければ null)。 */
export function finite(raw: string): number | null {
  const value = Number(raw);
  return raw.trim() !== "" && Number.isFinite(value) ? value : null;
}

/** 描画順の並びの 1 行: 本体のメッシュか、スロットの範囲の端。 */
export type DrawOrderRow =
  | { kind: "mesh"; id: string; drawOrder: number; slots: string[] }
  | { kind: "slot"; id: string; edge: "top" | "bottom"; drawOrder: number };

/**
 * 本体のメッシュを手前から奥へ (ツリーと同じ順) 並べ、スロットの範囲の上端と下端の行を差し込む。
 * メッシュの行の `slots` は、そのメッシュの描画順を範囲に含むスロット (モジュールのメッシュと前後が入り組む)。
 * 端の行は、同じ描画順のメッシュより外側 (上端は手前、下端は奥) に置く。
 */
export function drawOrderRows(index: ModelIndex, slots: readonly Slot[]): DrawOrderRow[] {
  const meshes = frontToBack(index, [...index.meshes.keys()]).map((id): DrawOrderRow => {
    const drawOrder = meshDrawOrder(index, id);
    const inside = slots
      .filter((s) => s.draw_order.min <= drawOrder && drawOrder <= s.draw_order.max)
      .map((s) => s.id);
    return { kind: "mesh", id, drawOrder, slots: inside };
  });
  const edges = slots.flatMap((s): DrawOrderRow[] => [
    { kind: "slot", id: s.id, edge: "top", drawOrder: s.draw_order.max },
    { kind: "slot", id: s.id, edge: "bottom", drawOrder: s.draw_order.min },
  ]);
  // 並べ替えの鍵: 描画順の降順。同じ描画順では 上端 → メッシュ → 下端。
  const rank = (r: DrawOrderRow) => (r.kind === "mesh" ? 1 : r.edge === "top" ? 0 : 2);
  const order = new Map<DrawOrderRow, number>(meshes.map((r, i) => [r, i]));
  return [...meshes, ...edges].sort(
    (a, b) =>
      b.drawOrder - a.drawOrder || rank(a) - rank(b) || (order.get(a) ?? 0) - (order.get(b) ?? 0),
  );
}

/** スロットの範囲に描画順が入る本体のメッシュ (「Select meshes in range」)。 */
export function meshesInRange(index: ModelIndex, range: DrawOrderRange): string[] {
  return frontToBack(index, [...index.meshes.keys()]).filter((id) => {
    const d = meshDrawOrder(index, id);
    return range.min <= d && d <= range.max;
  });
}
