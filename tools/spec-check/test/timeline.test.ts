// spec/examples/ の時間の例 (timeline.json のあるもの) をはじめから走らせ、フレームごとの期待値と
// 許容誤差 (spec/examples/README.md#timeline-tolerances) で比べる。
// ファイルの例 (spec/examples/bundles/) のうち時間の例と同じ中身のものは、バンドルから読んだデータでも走らせる
// (f32 に丸めて書いた値でも、期待値が許容誤差の内に収まること)。
// 例のファイルは spec/ (CC BY 4.0) から実行時に読み、このディレクトリには写さない。

import assert from "node:assert/strict";
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { test } from "node:test";
import { readBundle } from "../src/bundle.ts";
import { loadManifest } from "../src/bundles.ts";
import { loadTimeline, runTimeline } from "../src/timeline.ts";
import { SPEC_EXAMPLES } from "./paths.ts";

const names = readdirSync(SPEC_EXAMPLES, { withFileTypes: true })
  .filter((e) => e.isDirectory() && existsSync(join(SPEC_EXAMPLES, e.name, "timeline.json")))
  .map((e) => e.name)
  .sort();

test("spec/examples に時間の例がある", () => {
  assert.ok(names.length > 0);
});

for (const name of names) {
  test(`timeline ${name}`, () => {
    const example = loadTimeline(join(SPEC_EXAMPLES, name));
    assert.deepEqual(example.warnings, []);
    assert.deepEqual(runTimeline(example), []);
  });
}

const bundlesDir = join(SPEC_EXAMPLES, "bundles");
const bundleTimelines = loadManifest(bundlesDir).files.filter(
  (e) =>
    e.result === "ok" &&
    e.model !== undefined &&
    existsSync(join(dirname(resolve(bundlesDir, e.model)), "timeline.json")),
);

test("時間の例として走らせるファイルの例がある", () => {
  assert.ok(bundleTimelines.length > 0);
});

for (const e of bundleTimelines) {
  test(`timeline from file example ${e.file}`, () => {
    const exampleDir = dirname(resolve(bundlesDir, e.model ?? ""));
    const example = loadTimeline(exampleDir);
    const bundle = readBundle(new Uint8Array(readFileSync(join(bundlesDir, e.file))));
    assert.deepEqual(bundle.warnings, []);
    // 時間の例のキー → ファイル を、目録のファイルの順 (= バンドルのチャンクの順) で引き直す。
    const byFile = <T>(
      files: string[] | undefined,
      items: T[],
      keys: Record<string, string> = {},
    ) => {
      const paths = (files ?? []).map((f) => resolve(bundlesDir, f));
      assert.equal(items.length, paths.length);
      return new Map(
        Object.entries(keys).map(([k, f]) => {
          const i = paths.indexOf(resolve(exampleDir, f));
          assert.ok(i >= 0, `${f} is not in ${e.file}`);
          return [k, items[i] as T];
        }),
      );
    };
    const ran = runTimeline({
      ...example,
      model: bundle.model,
      physics: example.timeline.physics ? bundle.physics : null,
      motions: byFile(e.motions, bundle.motions, example.timeline.motions),
      expressions: byFile(e.expressions, bundle.expressions, example.timeline.expressions),
    });
    assert.deepEqual(ran, []);
  });
}
