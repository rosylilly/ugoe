// パラメータのパネルの数値の入力 (docs/editor.md#キーフォームの編集)。DOM に依存しない (parameterValue.test.ts)。
import { t } from "../i18n";
import { keyAt, keyTolerance } from "./inspector/keyforms/keyforms";

interface Range {
  min: number;
  max: number;
}

export type ParsedValue = { ok: true; value: number } | { ok: false; error: string };

/** 入力欄の表示: 小数第 3 位までに丸め、末尾の 0 を落とす (-0 は 0)。 */
export function formatValue(value: number): string {
  const rounded = Number(value.toFixed(3));
  return String(rounded === 0 ? 0 : rounded);
}

/** 入力された値を読む。数でないか、値域 (`min`〜`max`) の外なら理由を返す。 */
export function parseValue(text: string, range: Range): ParsedValue {
  const trimmed = text.trim();
  const value = trimmed === "" ? Number.NaN : Number(trimmed);
  if (!Number.isFinite(value))
    return { ok: false, error: t("parameters.value.notNumber", { text: trimmed }) };
  const lo = Math.min(range.min, range.max);
  const hi = Math.max(range.min, range.max);
  if (value < lo || value > hi) {
    return {
      ok: false,
      error: t("parameters.value.outOfRange", { value, min: range.min, max: range.max }),
    };
  }
  return { ok: true, value };
}

/** 値 `value` をキーとして足せるか。同じとみなせるキーがすでにあれば理由を返す (なければ null)。 */
export function duplicateKeyError(value: number, keys: number[], range: Range): string | null {
  const existing = keyAt(keys, value, keyTolerance(range));
  return existing === undefined ? null : t("parameters.key.duplicate", { key: existing });
}

/** スライダーのつまみの幅の半分 (CSS px)。キーの印は、つまみの中心が動く範囲 (両端から 8px 内側) に置く。 */
const THUMB_HALF = 8;

/** キー `key` の、値域の中での割合 (0〜1。値域が空なら真ん中)。 */
function keyFraction(range: Range, key: number): number {
  const span = range.max - range.min;
  return span > 0 ? (key - range.min) / span : 0.5;
}

/** キーの印の `left` (スライダーの幅に対する CSS の式)。 */
export function markerLeft(range: Range, key: number): string {
  return `calc(${THUMB_HALF}px + (100% - ${2 * THUMB_HALF}px) * ${keyFraction(range, key)})`;
}

/**
 * 印の帯の左端から `x` (CSS px) の所を押したときに移るキー: 中心が最も近い印のキー (帯の幅は `width`)。
 * 印の押せる範囲 (24px) は、近いキーどうしで重なる。重なった所では、上に描かれた印ではなく近い方のキーを選ぶ。
 */
export function nearestMarkerKey(
  range: Range,
  keys: readonly number[],
  x: number,
  width: number,
): number | undefined {
  let best: number | undefined;
  let bestDistance = Number.POSITIVE_INFINITY;
  for (const key of keys) {
    const center = THUMB_HALF + (width - 2 * THUMB_HALF) * keyFraction(range, key);
    const distance = Math.abs(center - x);
    if (distance < bestDistance) {
      best = key;
      bestDistance = distance;
    }
  }
  return best;
}
