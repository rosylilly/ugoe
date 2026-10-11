#!/usr/bin/env node
// 機械に固有のパスが入っていないことを検査する (T0090。docs/release.md#公開の前の点検)。
//
// 公開するリポジトリに、作業した機械のホームのディレクトリ (`/home/<名前>`・`/Users/<名前>`・`C:\Users\<名前>`) や
// エージェントの一時ディレクトリ (`/tmp/claude-<番号>/…`) を残さない。文書やタスクのメモでは、リポジトリの中の相対パスか
// `<repo>`・`<scratchpad>`・`$HOME` のような書き方にする。
// テストや例のパスは、1 文字の名前 (`/home/u`・`C:\Users\a`) か、下の `PLACEHOLDER_USERS` の名前を使う。
//
// 引数にファイルを渡すとそれだけ (pre-commit)、無ければ Git が追う (と、無視されていない新しい) ファイルすべてを見る。
// バイナリ (先頭に NUL がある) と、無くなったファイルは飛ばす。

import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, statSync } from "node:fs";
import { pathToFileURL } from "node:url";

/** 例のパスに使ってよいユーザー名 (小文字で比べる)。1 文字の名前もよい。 */
export const PLACEHOLDER_USERS = new Set([
  "audit",
  "me",
  "name",
  "runner",
  "runneradmin",
  "ugoe",
  "user",
  "you",
]);

// ホームのディレクトリ: `/home/x`・`/Users/x` (`/mnt/c/Users/x` を含む)・`C:\Users\x` (`\\` と `/` の区切りも)・
// WSL の UNC の `\home\x`。`/var/home/x` のような途中の一致も機械の場所かもしれないので見る
const HOME =
  /(?:\/home\/|\/Users\/|\b[A-Za-z]:(?:\\+|\/)Users(?:\\+|\/)|\\+home\\+)([A-Za-z0-9_-]+)/g;
// エージェント (Claude Code) の一時ディレクトリ
const AGENT_TMP = /\/tmp\/claude-\d/g;

/**
 * 1 つのファイルの中身から、機械に固有のパスを探す。
 * @param {string} text
 * @returns {{ line: number, match: string }[]}
 */
export function findLocalPaths(text) {
  const found = [];
  const lines = text.split("\n");
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    for (const m of line.matchAll(HOME)) {
      const user = m[1];
      if (user.length === 1 || PLACEHOLDER_USERS.has(user.toLowerCase())) continue;
      found.push({ line: i + 1, match: m[0] });
    }
    for (const m of line.matchAll(AGENT_TMP)) found.push({ line: i + 1, match: m[0] });
  }
  return found;
}

function trackedFiles() {
  const out = execFileSync(
    "git",
    ["ls-files", "-z", "--cached", "--others", "--exclude-standard"],
    {
      encoding: "utf8",
      maxBuffer: 256 * 1024 * 1024,
    },
  );
  return out.split("\0").filter(Boolean);
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  const files = process.argv.length > 2 ? process.argv.slice(2) : trackedFiles();
  let count = 0;
  for (const file of files) {
    if (!existsSync(file) || !statSync(file).isFile()) continue;
    const bytes = readFileSync(file);
    if (bytes.subarray(0, 8000).includes(0)) continue;
    for (const { line, match } of findLocalPaths(bytes.toString("utf8"))) {
      console.error(
        `error: ${file}:${line}: machine-specific path ${JSON.stringify(match)} ` +
          "(use a repository-relative path, or <repo>, <scratchpad>, $HOME, or a placeholder user such as /home/u)",
      );
      count++;
    }
  }
  if (count > 0) process.exit(1);
}
