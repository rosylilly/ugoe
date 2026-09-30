#!/usr/bin/env node
// Markdown の相対リンクとアンカーのリンク切れを検査する。
// 使い方: node scripts/check-md-links.mjs [ディレクトリかファイル ...] (既定は spec/)
//
// - 相対リンクの先のファイルがあること。
// - `#anchor` が、リンク先の Markdown の見出しから GitHub と同じ規則で作ったアンカーにあること。
// - http(s) や mailto のリンクは検査しない。コードブロックとインラインコードの中は無視する。

import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, extname, join, relative, resolve } from "node:path";
import { headings, stripCodeBlocks } from "./lib/markdown.mjs";

const root = resolve(dirname(new URL(import.meta.url).pathname), "..");
const targets = process.argv.slice(2);
if (targets.length === 0) targets.push("spec");

/** ディレクトリなら中の .md を再帰的に集める。 */
function collect(path, out) {
  if (statSync(path).isDirectory()) {
    for (const name of readdirSync(path).sort()) collect(join(path, name), out);
  } else if (extname(path) === ".md") {
    out.push(path);
  }
  return out;
}

const anchorCache = new Map();
/** ファイルの見出しのアンカーの集合。 */
function anchors(file) {
  if (!anchorCache.has(file)) {
    anchorCache.set(file, new Set(headings(file).map((h) => h.anchor)));
  }
  return anchorCache.get(file);
}

const errors = [];
const files = targets.flatMap((t) => collect(resolve(root, t), []));
for (const file of files) {
  const lines = stripCodeBlocks(readFileSync(file, "utf8")).split("\n");
  lines.forEach((raw, index) => {
    const line = raw.replace(/`[^`]*`/g, "");
    for (const m of line.matchAll(/\]\(([^)\s]+)(?:\s+"[^"]*")?\)/g)) {
      const link = m[1];
      if (/^[a-z][a-z0-9+.-]*:/i.test(link)) continue;
      const [path, anchor] = link.split("#");
      const target = path === "" ? file : resolve(dirname(file), decodeURI(path));
      const where = `${relative(root, file)}:${index + 1}`;
      if (!existsSync(target)) {
        errors.push(`${where}: ${link} の先がない`);
        continue;
      }
      if (anchor !== undefined && extname(target) === ".md") {
        if (!anchors(target).has(decodeURIComponent(anchor))) {
          errors.push(`${where}: ${link} のアンカーがない`);
        }
      }
    }
  });
}

for (const e of errors) console.error(`error: ${e}`);
if (errors.length === 0) console.log(`links ok (${files.length} files)`);
process.exitCode = errors.length > 0 ? 1 : 0;
