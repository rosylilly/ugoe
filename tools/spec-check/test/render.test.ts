// spec/examples/ の描画の例 (render.json のあるもの) をこの実装のラスタライザで描いて、調べる画素を
// 期待値と許容誤差 (spec/examples/README.md#rendering-tolerances) で比べる。ラスタライザの小さな性質も確かめる。

import assert from "node:assert/strict";
import { existsSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { test } from "node:test";
import { rasterize, sample } from "../src/render.ts";
import { compareRenderCase, loadRenderExample, pixelsNearEdges } from "../src/render-examples.ts";
import { SPEC_EXAMPLES } from "./paths.ts";

const names = readdirSync(SPEC_EXAMPLES, { withFileTypes: true })
  .filter((e) => e.isDirectory() && existsSync(join(SPEC_EXAMPLES, e.name, "render.json")))
  .map((e) => e.name)
  .sort();

test("spec/examples に描画の例がある", () => {
  assert.ok(names.length > 0);
});

for (const name of names) {
  test(`render example ${name}`, () => {
    const example = loadRenderExample(join(SPEC_EXAMPLES, name));
    assert.deepEqual(example.warnings, []);
    assert.equal(example.doc.textures.length, example.model.textures.length);
    example.model.textures.forEach((t, i) => {
      const spec = example.doc.textures[i];
      assert.equal(spec.width, t.width);
      assert.equal(spec.height, t.height);
      assert.equal(spec.rgba.length, 4 * t.width * t.height);
      assert.equal(spec.normal_map !== undefined, t.normal_map !== null);
    });
    const failures = example.doc.cases.flatMap((c, i) =>
      [...pixelsNearEdges(example, c), ...compareRenderCase(example, c)].map(
        (f) => `case ${i}: ${f}`,
      ),
    );
    assert.deepEqual(failures, []);
  });
}

test("隣り合う三角形は共有する辺の上の画素を 1 度だけ塗る (左上の規則)", () => {
  const count = new Map<string, number>();
  const visit = (x: number, y: number) => {
    const k = `${x},${y}`;
    count.set(k, (count.get(k) ?? 0) + 1);
  };
  // 対角線が画素の中心を通る 8 × 8 の正方形と、その下で水平の辺を共有する四角形。向きを混ぜる
  const quad = [
    [0, 0],
    [8, 0],
    [8, 8],
    [0, 8],
  ] as [number, number][];
  rasterize([quad[0], quad[1], quad[2]], 8, 12, visit);
  rasterize([quad[0], quad[3], quad[2]], 8, 12, visit);
  rasterize(
    [
      [0, 8],
      [8, 8],
      [8, 12],
    ],
    8,
    12,
    visit,
  );
  rasterize(
    [
      [0, 8],
      [8, 12],
      [0, 12],
    ],
    8,
    12,
    visit,
  );
  assert.equal(count.size, 96);
  assert.ok([...count.values()].every((n) => n === 1));
});

test("テクスチャは乗算済みの値で補間する", () => {
  const image = { width: 2, height: 1, rgba: [255, 0, 0, 0, 0, 0, 255, 255] };
  const mid = sample(image, 0.5, 0.5, true);
  assert.deepEqual(mid, [0, 0, 0.5, 0.5]);
  // clamp-to-edge
  assert.deepEqual(sample(image, -3, 0.5, true), [0, 0, 0, 0]);
  assert.deepEqual(sample(image, 3, 0.5, true), [0, 0, 1, 1]);
});
