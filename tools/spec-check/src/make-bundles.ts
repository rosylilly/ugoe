// 使い方: node tools/spec-check/src/make-bundles.ts
//
// ファイルの例 (spec/examples/bundles/) を、目録 bundles.json と JSON の文書から、この実装の書き出しで作り直す。
// 作ったファイルは参照実装でも読めることを確かめてからコミットする (tools/spec-check/README.md)。

import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildFile, checkFile, loadManifest } from "./bundles.ts";

const dir = join(import.meta.dirname, "../../../spec/examples/bundles");
let failed = false;
for (const e of loadManifest(dir).files) {
  const bytes = buildFile(dir, e);
  writeFileSync(join(dir, e.file), bytes);
  const failures = checkFile(dir, e, bytes);
  for (const f of failures) console.log(f);
  failed ||= failures.length > 0;
  console.log(`${e.file}: ${bytes.length} bytes`);
}
process.exit(failed ? 1 : 0);
