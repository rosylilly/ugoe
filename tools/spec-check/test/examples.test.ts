// spec/examples/ の例のファイルを評価して、期待値と許容誤差 (spec/examples/README.md#tolerances) で比べる。
// 例のファイルは spec/ (CC BY 4.0) から実行時に読み、このディレクトリには写さない。

import assert from "node:assert/strict";
import { readdirSync } from "node:fs";
import { join } from "node:path";
import { test } from "node:test";
import { compareCase, loadExample } from "../src/examples.ts";
import { SPEC_EXAMPLES } from "./paths.ts";

const names = readdirSync(SPEC_EXAMPLES, { withFileTypes: true })
  .filter((e) => e.isDirectory())
  .map((e) => e.name)
  .sort();

test("spec/examples に例がある", () => {
  assert.ok(names.length > 0);
});

for (const name of names) {
  test(`example ${name}`, () => {
    const example = loadExample(join(SPEC_EXAMPLES, name));
    assert.deepEqual(example.warnings, []);
    const failures = example.expected.cases.flatMap((c, i) =>
      compareCase(example.model, c).map((f) => `case ${i} ${JSON.stringify(c.parameters)}: ${f}`),
    );
    assert.deepEqual(failures, []);
  });
}
