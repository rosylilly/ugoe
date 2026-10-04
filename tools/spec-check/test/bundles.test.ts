// ファイルの例 (spec/examples/bundles/) を読み、目録 bundles.json と比べる。
// また、目録からこの実装の書き出しで作り直したものが、置いてあるファイルと同じ中身であることを確かめる
// (zstd の圧縮の結果は版で変わりうるので、展開したペイロードで比べる)。

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { test } from "node:test";
import { zstdDecompressSync } from "node:zlib";
import { buildFile, checkFile, loadManifest, storedTable } from "../src/bundles.ts";
import { SPEC_EXAMPLES } from "./paths.ts";

const dir = join(SPEC_EXAMPLES, "bundles");
const manifest = loadManifest(dir);

const KNOWN = new Set(["FEAT", "MODL", "MODU", "PHYS", "MOTN", "EXPR", "META", "TEXR", "NRML"]);

/**
 * 表の順の (タグ, flags, 展開したペイロード)。知らないチャンクは格納されたまま。
 * 表を拒むファイルもあるので、表は検査せずに読む。
 */
function contents(bytes: Uint8Array) {
  return storedTable(bytes).map((c) => {
    const raw = bytes.subarray(c.offset, c.offset + c.length);
    const data = (c.flags & 1) === 1 && KNOWN.has(c.tag) ? zstdDecompressSync(raw) : raw;
    return { tag: c.tag, flags: c.flags, data: Buffer.from(data).toString("hex") };
  });
}

test("spec/examples/bundles に例がある", () => {
  assert.ok(manifest.files.length > 0);
});

for (const e of manifest.files) {
  test(`file example ${e.file}`, () => {
    const bytes = new Uint8Array(readFileSync(join(dir, e.file)));
    assert.deepEqual(checkFile(dir, e, bytes), []);
  });

  test(`file example ${e.file} is what the writer makes from bundles.json`, () => {
    const bytes = new Uint8Array(readFileSync(join(dir, e.file)));
    assert.deepEqual(contents(buildFile(dir, e)), contents(bytes));
  });
}
