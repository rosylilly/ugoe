#!/usr/bin/env node
// 大文字と小文字だけが違う名前が同じディレクトリに無いことを検査する (T0088)。
//
// macOS と Windows のファイルシステムは大文字と小文字を区別しないので、Linux では別のファイルが同じものになる。
// ファイル名どうし (`Foo.md` と `foo.md`) のほか、import の解決でぶつかるもの (`FileMenu.svelte` と、`./fileMenu.svelte` で
// import する `fileMenu.svelte.ts`) も見る。スクリプトの拡張子を取った名前を import の名前として比べる。
// 対象は Git が追う (と、無視されていない新しい) ファイルとそのディレクトリ。

import { execFileSync } from "node:child_process";
import { pathToFileURL } from "node:url";

// import で書かないことがある拡張子 (TypeScript と Vite の解決)
const SCRIPT_EXT = /\.(?:[cm]?[jt]s|[jt]sx)$/;

/**
 * パスの一覧から、同じディレクトリで大文字と小文字だけが違ってぶつかる名前の組を返す。
 * @param {string[]} paths `/` 区切りの相対パス
 * @returns {{ dir: string, names: string[] }[]}
 */
export function findCaseCollisions(paths) {
  // ディレクトリ → その直下の名前 (ファイルとディレクトリ)
  const dirs = new Map();
  for (const path of paths) {
    const parts = path.split("/");
    for (let i = 0; i < parts.length; i++) {
      const dir = parts.slice(0, i).join("/");
      if (!dirs.has(dir)) dirs.set(dir, new Set());
      dirs.get(dir).add(parts[i]);
    }
  }
  const collisions = [];
  for (const [dir, names] of dirs) {
    // 小文字にした形 → [元の形, 名前]
    const seen = new Map();
    const found = new Set();
    for (const name of names) {
      const forms = new Set([name]);
      if (SCRIPT_EXT.test(name)) forms.add(name.replace(SCRIPT_EXT, ""));
      for (const form of forms) {
        const key = form.toLowerCase();
        const prev = seen.get(key);
        if (!prev) {
          seen.set(key, [form, name]);
        } else if (prev[0] !== form && prev[1] !== name) {
          // 形が大文字と小文字だけ違う (`foo.ts` と `foo.js` のように同じ形になるものは、ここでは見ない)
          found.add(JSON.stringify([prev[1], name].sort()));
        }
      }
    }
    for (const pair of found) collisions.push({ dir, names: JSON.parse(pair) });
  }
  return collisions.sort((a, b) => a.dir.localeCompare(b.dir));
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  const out = execFileSync(
    "git",
    ["ls-files", "-z", "--cached", "--others", "--exclude-standard"],
    {
      encoding: "utf8",
      maxBuffer: 256 * 1024 * 1024,
    },
  );
  const collisions = findCaseCollisions(out.split("\0").filter(Boolean));
  for (const { dir, names } of collisions) {
    console.error(
      `error: ${dir || "."}: ${names.join(" and ")} differ only in case ` +
        "(the same file on macOS and Windows)",
    );
  }
  if (collisions.length > 0) process.exit(1);
}
