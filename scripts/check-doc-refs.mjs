#!/usr/bin/env node
// コード (Rust・TypeScript・Svelte・C# など) のコメントから文書への参照の切れを検査する (`just refs`)。
//
// `docs/spec/model.md#検証` のような、リポジトリの根からのパスで書いた参照を集め、
// - ファイルがあること
// - `#` の後があれば、そのファイルの見出しにあること (GitHub と同じ規則で作ったアンカーと、書いた文字を同じ規則で崩したものを比べる。
//   `#ID` や `#1.0-の基準の機能` のように見出しの文字のまま書いてもよい)
// を確かめる。Markdown のリンクは scripts/check-md-links.mjs が見るので、ここでは Markdown のファイルは読まない。
//
// M9 のコードレビュー (T0316) で、見出しを変えたときにコードからの参照が古くなることが分かって足した
// (docs/workflow/code-review.md#m9-の結果)。

import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { headings, slug } from "./lib/markdown.mjs";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const SOURCES = [
  "*.rs",
  "*.ts",
  "*.svelte",
  "*.mjs",
  "*.js",
  "*.cs",
  "*.gd",
  "*.h",
  "*.toml",
  "*.yml",
  "*.just",
  "justfile",
];
// 検査そのもののテスト (例として存在しない見出しを書く)。
const EXCLUDE = new Set(["scripts/check-refs.test.mjs"]);

/**
 * 本文から参照 `{ path, anchor, line }` を集める。`anchor` は無ければ `undefined`。
 * 前に英数字・`/`・`.` が続く (相対パスの一部・URL) ものは除く。
 */
export function refs(text) {
  const out = [];
  text.split("\n").forEach((line, index) => {
    const pattern = /(?<![\w/.-])((?:docs|spec)\/[\w/.-]*?\.md)(?:#([^\s)\]`'",;:。、）」>]+))?/g;
    for (const m of line.matchAll(pattern)) {
      out.push({ path: m[1], anchor: m[2], line: index + 1 });
    }
  });
  return out;
}

/** `anchor` が見出しのアンカーの集合 `anchors` にあるか。書いた文字も同じ規則で崩して比べる。 */
export function hasAnchor(anchors, anchor) {
  let decoded = anchor;
  try {
    decoded = decodeURIComponent(anchor);
  } catch {
    // そのまま比べる
  }
  return anchors.has(decoded) || anchors.has(slug(decoded));
}

function main() {
  const files = execFileSync("git", ["ls-files", "-z", "--", ...SOURCES], {
    cwd: ROOT,
    encoding: "utf8",
  })
    .split("\0")
    .filter((f) => f && !EXCLUDE.has(f));
  const cache = new Map();
  const anchorsOf = (path) => {
    if (!cache.has(path)) cache.set(path, new Set(headings(join(ROOT, path)).map((h) => h.anchor)));
    return cache.get(path);
  };
  const errors = [];
  let count = 0;
  for (const file of files) {
    for (const ref of refs(readFileSync(join(ROOT, file), "utf8"))) {
      count++;
      const where = `${file}:${ref.line}`;
      if (!existsSync(join(ROOT, ref.path))) {
        errors.push(`${where}: ${ref.path} が無い`);
      } else if (ref.anchor !== undefined && !hasAnchor(anchorsOf(ref.path), ref.anchor)) {
        errors.push(`${where}: ${ref.path}#${ref.anchor} の見出しが無い`);
      }
    }
  }
  for (const e of errors) console.error(`error: ${e}`);
  if (errors.length === 0) console.log(`doc refs ok (${count} refs in ${files.length} files)`);
  return errors.length > 0 ? 1 : 0;
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  process.exitCode = main();
}
