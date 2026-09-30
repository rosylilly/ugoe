// assets/compat/ の互換性テストの資料を、この実装で読む (docs/testing.md#互換性テスト)。
// 資料は参照実装が書き出したバイナリで、中身は黒箱として扱う: 期待するのは docs/testing.md の表の結果
// (読める・警告付きで読める・拒否される) と、劣化させて読んだ結果が元のファイルと同じになることだけ。

import assert from "node:assert/strict";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { test } from "node:test";
import {
  type Bundle,
  type ModuleFile,
  ReadError,
  readBundle,
  readModuleFile,
} from "../src/bundle.ts";
import { compose } from "../src/compose.ts";
import { type EvaluationResult, evaluate, parameterValues } from "../src/evaluate.ts";
import type * as M from "../src/types.ts";
import { COMPAT } from "./paths.ts";

const read = (path: string) => new Uint8Array(readFileSync(join(COMPAT, path)));
const bundle = (path: string): Bundle => readBundle(read(path));
const moduleFile = (path: string): ModuleFile => readModuleFile(read(path));

/** 各パラメータの最小・既定・最大と、全部を最大・最小にした組。 */
function parameterCases(model: M.Model): Record<string, number>[] {
  const cases: Record<string, number>[] = [{}];
  for (const p of model.parameters) cases.push({ [p.id]: p.min }, { [p.id]: p.max });
  cases.push(Object.fromEntries(model.parameters.map((p) => [p.id, p.max])));
  cases.push(Object.fromEntries(model.parameters.map((p) => [p.id, p.min])));
  cases.push(Object.fromEntries(model.parameters.map((p) => [p.id, (p.min + p.max) / 3])));
  return cases;
}

function evaluateAll(model: M.Model): EvaluationResult[] {
  return parameterCases(model).map((c) => evaluate(model, parameterValues(model, c)));
}

function assertFinite(results: EvaluationResult[]): void {
  for (const r of results) {
    for (const m of r.meshes) {
      for (const [x, y] of m.positions) assert.ok(Number.isFinite(x) && Number.isFinite(y), m.id);
      assert.ok(Number.isFinite(m.opacity) && Number.isFinite(m.draw_order), m.id);
    }
  }
}

const base = () => bundle("dev/dress-up-base.ugoe");

for (const file of readdirSync(join(COMPAT, "dev")).sort()) {
  test(`dev/${file} loads without warnings and evaluates`, () => {
    if (file.endsWith(".ugoe")) {
      const b = bundle(`dev/${file}`);
      assert.deepEqual(b.warnings, []);
      assertFinite(evaluateAll(b.model));
    } else {
      const m = moduleFile(`dev/${file}`);
      assert.deepEqual(m.warnings, []);
      const b = base();
      const composed = compose(b.model, b.physics, [m.module]);
      assertFinite(evaluateAll(composed.model));
    }
  });
}

test("dev modules compose together", () => {
  const b = base();
  const modules = ["dev/long-hair.ugoemod", "dev/dress.ugoemod"].map((f) => moduleFile(f).module);
  const composed = compose(b.model, b.physics, modules);
  const hidden = composed.model.parts.filter((p) => p.id.startsWith("#replaced/"));
  assert.ok(hidden.length > 0, "slots with replaces hide the replaced parts");
  assertFinite(evaluateAll(composed.model));
});

test("requires.avatar_level: プロファイルの無い本体は満たさない (level 0 でも)", () => {
  const b = base();
  const m = moduleFile("dev/long-hair.ugoemod").module;
  const withoutProfile = { ...b.model, avatar: null };
  const requiring = (level: number) => ({ ...m, requires: { ...m.requires, avatar_level: level } });
  assert.throws(() => compose(withoutProfile, b.physics, [requiring(0)]));
  const profile = readBundle(read("dev/avatar-profile.ugoe")).model.avatar;
  assert.ok(profile !== null);
  const withProfile = { ...b.model, avatar: { ...profile, level: 2 } };
  compose(withProfile, b.physics, [requiring(2)]);
  assert.throws(() => compose(withProfile, b.physics, [requiring(3)]));
});

/** 2 つのモデルの評価の結果が、同じ ID のメッシュで一致する。 */
function assertSameEvaluation(actual: M.Model, expected: M.Model): void {
  assert.deepEqual(
    actual.meshes.map((m) => [m.id, m.blend, m.mask_mode]),
    expected.meshes.map((m) => [m.id, m.blend, m.mask_mode]),
  );
  for (const c of parameterCases(expected)) {
    const a = evaluate(actual, parameterValues(actual, c));
    const e = evaluate(expected, parameterValues(expected, c));
    assert.deepEqual(a.render_order, e.render_order);
    a.meshes.forEach((m, i) => {
      const em = e.meshes[i];
      m.positions.forEach(([x, y], k) => {
        assert.ok(Math.abs(x - em.positions[k][0]) <= 0.01, `${m.id}[${k}].x ${JSON.stringify(c)}`);
        assert.ok(Math.abs(y - em.positions[k][1]) <= 0.01, `${m.id}[${k}].y ${JSON.stringify(c)}`);
      });
      assert.ok(Math.abs(m.opacity - em.opacity) <= 1e-4, `${m.id}.opacity`);
      assert.ok(Math.abs(m.draw_order - em.draw_order) <= 1e-3, `${m.id}.draw_order`);
    });
  }
}

test("future/optional-feature.ugoe degrades to dev/dress-up-base.ugoe", () => {
  const b = bundle("future/optional-feature.ugoe");
  assert.ok(b.warnings.includes('unknown optional feature "test.future_optional"'));
  assert.ok(b.warnings.length > 1, "unknown values are reported");
  assertSameEvaluation(b.model, base().model);
  // 知らない対象・セグメントの曲線と、知らない演算の項目は捨てられ、元にあったものだけが残る
  // (資料は元の曲線や項目を書き換えて知らない値を入れているので、数は元より少なくなりうる)
  const original = base();
  const within = <T>(xs: T[], ys: T[]) =>
    xs.every((x) => ys.some((y) => JSON.stringify(y) === JSON.stringify(x)));
  assert.ok(within(b.motions[0].curves, original.motions[0].curves));
  assert.ok(within(b.expressions[0].parameters, original.expressions[0].parameters));
});

test("future/optional-feature.ugoemod degrades to dev/long-hair.ugoemod", () => {
  const m = moduleFile("future/optional-feature.ugoemod");
  assert.ok(m.warnings.includes('unknown optional feature "test.future_optional"'));
  const b = base();
  const degraded = compose(b.model, b.physics, [m.module]);
  const original = compose(b.model, b.physics, [moduleFile("dev/long-hair.ugoemod").module]);
  assertSameEvaluation(degraded.model, original.model);
});

for (const ext of ["ugoe", "ugoemod"]) {
  const load = ext === "ugoe" ? bundle : moduleFile;

  test(`future/required-feature.${ext} is refused before decoding`, () => {
    assert.throws(
      () => load(`future/required-feature.${ext}`),
      (e) =>
        e instanceof ReadError &&
        e.kind === "unsupported_features" &&
        e.details.join() === "test.future_required",
    );
  });

  test(`future/newer-schema.${ext} is refused by schema_version`, () => {
    assert.throws(
      () => load(`future/newer-schema.${ext}`),
      (e) => e instanceof ReadError && e.kind === "schema_version",
    );
  });
}
