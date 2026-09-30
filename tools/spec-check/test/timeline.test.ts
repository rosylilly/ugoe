// spec/examples/ の時間の例 (timeline.json のあるもの) をはじめから走らせ、フレームごとの期待値と
// 許容誤差 (spec/examples/README.md#timeline-tolerances) で比べる。
// 例のファイルは spec/ (CC BY 4.0) から実行時に読み、このディレクトリには写さない。

import assert from "node:assert/strict";
import { existsSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { test } from "node:test";
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
