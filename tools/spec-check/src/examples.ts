// 例のファイル (spec/examples/README.md) の読み込みと、期待値との比較。

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { readModelJson } from "./bundle.ts";
import { type EvaluationResult, evaluate, parameterValues } from "./evaluate.ts";
import type * as M from "./types.ts";

export interface ExpectedMesh {
  positions?: M.Vec2[];
  opacity?: number;
  draw_order?: number;
  multiply_color?: M.Rgb;
  screen_color?: M.Rgb;
  mask_strength?: number;
}

export interface ExpectedCase {
  parameters: Record<string, number>;
  meshes: Record<string, ExpectedMesh>;
  render_order?: string[];
}

export interface Expected {
  model: string;
  summary: string;
  cases: ExpectedCase[];
}

export interface Example {
  model: M.Model;
  expected: Expected;
  warnings: string[];
}

/** 例のディレクトリを読む。モデルは JSON の文書として読み、検証する。 */
export function loadExample(dir: string): Example {
  const expected = JSON.parse(readFileSync(join(dir, "expected.json"), "utf8")) as Expected;
  const path = join(dir, expected.model);
  const { model, warnings } = readModelJson(readFileSync(path, "utf8"));
  return { model, expected, warnings };
}

/** 許容誤差 (spec/evaluation.md#conformance)。 */
export const TOLERANCE = { position: 0.01, value: 1e-4, draw_order: 1e-3 };

/** 1 つのケースを評価して比べ、食い違いの説明の一覧を返す (空なら一致)。 */
export function compareCase(model: M.Model, c: ExpectedCase): string[] {
  const result = evaluate(model, parameterValues(model, c.parameters));
  const failures = compareMeshes(result, c.meshes, TOLERANCE.position);
  if (c.render_order) {
    const actual = result.render_order.map((i) => result.meshes[i].id);
    if (!sameRenderOrder(actual, c.render_order, result.meshes)) {
      failures.push(`render_order: expected ${c.render_order.join(",")}, got ${actual.join(",")}`);
    }
  }
  return failures;
}

/** 評価の結果をメッシュごとの期待値と比べる。位置の許容誤差は `positionTolerance`。 */
export function compareMeshes(
  result: EvaluationResult,
  expected: Record<string, ExpectedMesh>,
  positionTolerance: number,
): string[] {
  const failures: string[] = [];
  const byId = new Map(result.meshes.map((m) => [m.id, m]));
  const near = (what: string, actual: number, expected: number, tol: number) => {
    if (!(Math.abs(actual - expected) <= tol)) {
      failures.push(`${what}: expected ${expected}, got ${actual}`);
    }
  };
  for (const [id, exp] of Object.entries(expected)) {
    const m = byId.get(id);
    if (!m) {
      failures.push(`mesh ${id} not found`);
      continue;
    }
    if (exp.positions) {
      if (exp.positions.length !== m.positions.length) {
        failures.push(`${id}.positions: expected ${exp.positions.length} vertices`);
      } else {
        exp.positions.forEach(([x, y], i) => {
          near(`${id}.positions[${i}].x`, m.positions[i][0], x, positionTolerance);
          near(`${id}.positions[${i}].y`, m.positions[i][1], y, positionTolerance);
        });
      }
    }
    if (exp.opacity !== undefined) near(`${id}.opacity`, m.opacity, exp.opacity, TOLERANCE.value);
    if (exp.draw_order !== undefined) {
      near(`${id}.draw_order`, m.draw_order, exp.draw_order, TOLERANCE.draw_order);
    }
    for (const key of ["multiply_color", "screen_color"] as const) {
      const e = exp[key];
      if (!e) continue;
      for (const ch of ["r", "g", "b"] as const) {
        near(`${id}.${key}.${ch}`, m[key][ch], e[ch], TOLERANCE.value);
      }
    }
    if (exp.mask_strength !== undefined) {
      near(`${id}.mask_strength`, m.mask_strength, exp.mask_strength, TOLERANCE.value);
    }
  }
  return failures;
}

/**
 * 描画順の比較。`draw_order` の差が 1e-3 以下のメッシュどうしは、どちらの順でもよい。
 * 期待する並びの中で、順が入れ替わっている組がすべてそのような組なら一致とする。
 */
function sameRenderOrder(
  actual: string[],
  expected: string[],
  meshes: { id: string; draw_order: number }[],
): boolean {
  if (actual.length !== expected.length) return false;
  if ([...actual].sort().join("\0") !== [...expected].sort().join("\0")) return false;
  const order = new Map(meshes.map((m) => [m.id, m.draw_order]));
  const pos = new Map(actual.map((id, i) => [id, i]));
  for (let i = 0; i < expected.length; i++) {
    for (let j = i + 1; j < expected.length; j++) {
      const a = expected[i];
      const b = expected[j];
      if ((pos.get(a) ?? 0) > (pos.get(b) ?? 0)) {
        const da = order.get(a) ?? 0;
        const db = order.get(b) ?? 0;
        if (Math.abs(da - db) > TOLERANCE.draw_order) return false;
      }
    }
  }
  return true;
}
