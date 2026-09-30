// Markdown の検査で共有する小さな道具 (check-md-links.mjs と check-spec.mjs)。

import { readFileSync } from "node:fs";

/** コードブロックを空行に置き換える (行番号を保つ)。 */
export function stripCodeBlocks(text) {
  let inFence = false;
  return text
    .split("\n")
    .map((line) => {
      if (/^\s*(```|~~~)/.test(line)) {
        inFence = !inFence;
        return "";
      }
      return inFence ? "" : line;
    })
    .join("\n");
}

/** GitHub の見出しのアンカー: 小文字にし、英数字・空白・`-`・`_` 以外を除き、空白を `-` にする。 */
export function slug(heading) {
  return heading
    .replace(/`/g, "")
    .replace(/\[([^\]]*)\]\([^)]*\)/g, "$1")
    .trim()
    .toLowerCase()
    .replace(/[^\p{L}\p{N}\p{M} _-]/gu, "")
    .replace(/ /g, "-");
}

/**
 * ファイルの見出しの一覧 `{ level, text, anchor, line }`。
 * アンカーは GitHub と同じく、同じ見出しの 2 つ目から -1, -2 … を付ける。
 */
export function headings(file) {
  const out = [];
  const counts = new Map();
  const lines = stripCodeBlocks(readFileSync(file, "utf8")).split("\n");
  lines.forEach((line, index) => {
    const m = /^(#{1,6})\s+(.*?)\s*#*\s*$/.exec(line);
    if (!m) return;
    const base = slug(m[2]);
    const n = counts.get(base) ?? 0;
    counts.set(base, n + 1);
    out.push({
      level: m[1].length,
      text: m[2],
      anchor: n === 0 ? base : `${base}-${n}`,
      line: index + 1,
    });
  });
  return out;
}
