// 表情の作成とプレビューの、DOM に依存しない補助。Vitest で検査する (expressions.test.ts)。
// 表情の式は docs/spec/motion.md#表情。
import { t } from "../../i18n";
import type { Expression, ExpressionOp, ExpressionParameter, Parameter } from "../../ipc";

/** 表情の op の選択肢 (翻訳を通す。呼ぶたびに今の言語で作る)。 */
export function expressionOps(): { value: ExpressionOp; label: string }[] {
  return [
    { value: "add", label: t("timeline.expression.op.add") },
    { value: "multiply", label: t("timeline.expression.op.multiply") },
    { value: "overwrite", label: t("timeline.expression.op.overwrite") },
  ];
}

/**
 * 今の値から表情の値を作る。`add` は既定値との差、`multiply` は既定値との比 (既定値が 0 なら作れず null)、
 * `overwrite` は今の値そのもの。既定値に今の値を重ねると今の値になる。
 */
export function valueFor(op: ExpressionOp, current: number, base: number): number | null {
  switch (op) {
    case "add":
      return current - base;
    case "multiply":
      return Math.abs(base) < 1e-9 ? null : current / base;
    case "overwrite":
      return current;
  }
}

/** 既定値から動かしたパラメータ (値域の 1e-4 倍より大きく違うもの) の ID。 */
export function changedParameters(
  parameters: Parameter[],
  values: Record<string, number>,
): string[] {
  return parameters
    .filter((p) => {
      const v = values[p.id];
      return v !== undefined && Math.abs(v - p.default) > Math.max(1e-6, (p.max - p.min) * 1e-4);
    })
    .map((p) => p.id);
}

/**
 * 今の値から表情を作る。`ids` のパラメータを `op` で、既定値に重ねると今の値になるように。
 * `multiply` で作れない (既定値が 0 の) パラメータは `add` にする。
 */
export function expressionFromValues(
  name: string,
  parameters: Parameter[],
  values: Record<string, number>,
  ids: string[],
  op: ExpressionOp,
): Expression {
  const rows: ExpressionParameter[] = [];
  for (const p of parameters) {
    if (!ids.includes(p.id)) continue;
    const current = values[p.id] ?? p.default;
    const value = valueFor(op, current, p.default);
    rows.push(
      value === null
        ? { parameter: p.id, op: "add", value: current - p.default }
        : { parameter: p.id, op, value },
    );
  }
  return { name, fade_in: 0.5, fade_out: 0.5, parameters: rows };
}

/**
 * 表情を重み 1 で重ねた値 (プレビュー)。`base` は重ねる前の値。モデルにないパラメータは無視する。
 * 値域には収めない (ランタイムが評価で収める)。
 */
export function applyExpression(
  expression: Expression,
  base: Record<string, number>,
): Record<string, number> {
  const out: Record<string, number> = {};
  for (const p of expression.parameters) {
    const v = out[p.parameter] ?? base[p.parameter];
    if (v === undefined) continue;
    switch (p.op) {
      case "add":
        out[p.parameter] = v + p.value;
        break;
      case "multiply":
        out[p.parameter] = v * p.value;
        break;
      case "overwrite":
        out[p.parameter] = p.value;
        break;
    }
  }
  return out;
}
