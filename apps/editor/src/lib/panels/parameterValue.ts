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
