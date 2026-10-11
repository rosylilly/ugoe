#!/usr/bin/env node
// このリポジトリを指す URL が、ルートの Cargo.toml の `workspace.package.repository` と食い違わないことを検査する
// (T1003。docs/release.md#公開の前の点検)。
//
// 公開する場所 (GitHub の owner や repo の名前) を変えたとき、SECURITY.md・Issue のテンプレート・README・各パッケージの
// 設定 (npm と Unity の package.json・Godot の Cargo.toml・Tauri の capabilities・MCPB の manifest など) に古い URL が残らないようにする。
//
// 見分け方: GitHub の owner と repo の組 (下の `REFERENCE_PATTERNS` の形) を拾い、正の owner と repo を比べる。
// - 両方が同じ (大文字と小文字は区別しない): このリポジトリを正しく指している。
// - どちらか片方だけが同じ: このリポジトリを古い場所で指している (owner の改名・repo の改名) とみなし、誤りにする。
// - 両方が違う: ほかのリポジトリ (依存・出典) なので見ない。
// 正の owner か repo と同じ名前を持つ、本当にほかのリポジトリへのリンクを置くときは、`OTHER_REPOS` に足す。
// owner と repo の両方を一度に変えると、古い URL は「ほかのリポジトリ」に見えて拾えない。そのときは、変える前に
// 古い URL を `git grep` で探して直すか、この検査を先に流して片方ずつ変える。
//
// 引数にファイルを渡すとそれだけ、無ければ Git が追う (と、無視されていない新しい) ファイルすべてを見る。
// バイナリ (先頭に NUL がある) と、無くなったファイルは飛ばす。

import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, statSync } from "node:fs";
import { basename } from "node:path";
import { pathToFileURL } from "node:url";

/** 正の owner か repo と同じ名前を持つが、このリポジトリではないもの (`owner/repo` を小文字で)。 */
export const OTHER_REPOS = new Set([]);

// GitHub の owner は英数字とハイフン、repo はそれに `_` と `.` を足したもの
const OWNER = "[A-Za-z0-9-]+";
const REPO = "[A-Za-z0-9_.-]+";

/**
 * このリポジトリを指しうる書き方。どれも 1 つ目の組が owner、2 つ目が repo。
 * - `https://github.com/o/r` (`http`・`www.`・`git+https://`・`ssh://git@github.com/` も)
 * - `git@github.com:o/r` (SSH の短い形)
 * - `https://raw.githubusercontent.com/o/r/…` (JSON Schema の `$id` など)
 * - `github:o/r` (npm の package.json の `repository` の短い形)
 * - `--repo o/r` (`gh` のコマンドの例)
 */
const REFERENCE_PATTERNS = [
  new RegExp(`(?:raw\\.githubusercontent\\.com|github\\.com)[/:](${OWNER})/(${REPO})`, "g"),
  new RegExp(`\\bgithub:(${OWNER})/(${REPO})`, "g"),
  new RegExp(`--repo[ =](${OWNER})/(${REPO})`, "g"),
];

/**
 * URL などから拾った repo の名前から、`.git` と文の終わりの `.` を除く。
 * @param {string} repo
 */
function trimRepo(repo) {
  return repo.replace(/\.+$/, "").replace(/\.git$/i, "");
}

/**
 * `https://github.com/<owner>/<repo>` の形の URL から owner と repo を取り出す。
 * @param {string} url
 * @returns {{ owner: string, repo: string }}
 */
export function parseRepositoryUrl(url) {
  const m = new RegExp(`^(?:git\\+)?https://github\\.com/(${OWNER})/(${REPO})/?$`).exec(url);
  if (!m) throw new Error(`repository is not a GitHub URL: ${JSON.stringify(url)}`);
  return { owner: m[1], repo: trimRepo(m[2]) };
}

/**
 * ルートの Cargo.toml の `[workspace.package]` の `repository` を読む。
 * @param {string} toml
 * @returns {string}
 */
export function workspaceRepository(toml) {
  let inSection = false;
  for (const raw of toml.split("\n")) {
    const line = raw.trim();
    if (line.startsWith("[")) {
      inSection = line === "[workspace.package]";
      continue;
    }
    if (!inSection) continue;
    const m = /^repository\s*=\s*"([^"]*)"/.exec(line);
    if (m) return m[1];
  }
  throw new Error("Cargo.toml has no repository in [workspace.package]");
}

/**
 * 1 つのファイルの中身から、このリポジトリを古い場所で指している参照を探す。
 * @param {string} text
 * @param {{ owner: string, repo: string }} expected
 * @returns {{ line: number, match: string }[]}
 */
export function findStaleRepoUrls(text, expected) {
  const owner = expected.owner.toLowerCase();
  const repo = expected.repo.toLowerCase();
  const found = [];
  const lines = text.split("\n");
  for (let i = 0; i < lines.length; i++) {
    for (const pattern of REFERENCE_PATTERNS) {
      for (const m of lines[i].matchAll(pattern)) {
        const o = m[1].toLowerCase();
        const r = trimRepo(m[2]).toLowerCase();
        const sameOwner = o === owner;
        const sameRepo = r === repo;
        if (sameOwner === sameRepo) continue;
        if (OTHER_REPOS.has(`${o}/${r}`)) continue;
        found.push({ line: i + 1, match: m[0] });
      }
    }
  }
  return found;
}

/**
 * パッケージの設定の `repository` (npm・Unity の package.json の `repository`、Cargo.toml の `repository = "…"`) を確かめる。
 * これはそのパッケージ自身の置き場なので、両方が違う (ほかのリポジトリに見える) ときも誤りにする。`"o/r"` の短い形も見る。
 * 片方だけが違うものは本文の検査 (`findStaleRepoUrls`) が拾うので、二重に数えないよう飛ばす。
 * @param {string} file ファイルの名前 (package.json か Cargo.toml だけを見る)
 * @param {string} text
 * @param {{ owner: string, repo: string }} expected
 * @returns {{ line: number, match: string }[]}
 */
export function findStaleManifestRepository(file, text, expected) {
  const values = [];
  if (basename(file) === "package.json") {
    let json;
    try {
      json = JSON.parse(text);
    } catch {
      return [];
    }
    const repository = json?.repository;
    const value = typeof repository === "string" ? repository : repository?.url;
    if (typeof value === "string") {
      const line = text.split("\n").findIndex((l) => l.includes(JSON.stringify(value)));
      values.push({ line: line + 1 || 1, value });
    }
  } else if (basename(file) === "Cargo.toml") {
    text.split("\n").forEach((l, i) => {
      const m = /^\s*repository\s*=\s*"([^"]*)"/.exec(l);
      if (m) values.push({ line: i + 1, value: m[1] });
    });
  }
  const found = [];
  for (const { line, value } of values) {
    const m =
      new RegExp(`^(?:github:)?(${OWNER})/(${REPO})$`).exec(value) ??
      new RegExp(`github\\.com[/:](${OWNER})/(${REPO})/?$`).exec(value);
    const same =
      m !== null &&
      m[1].toLowerCase() === expected.owner.toLowerCase() &&
      trimRepo(m[2]).toLowerCase() === expected.repo.toLowerCase();
    if (same || findStaleRepoUrls(value, expected).length > 0) continue;
    found.push({ line, match: value });
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
  const url = workspaceRepository(readFileSync("Cargo.toml", "utf8"));
  const expected = parseRepositoryUrl(url);
  const files = process.argv.length > 2 ? process.argv.slice(2) : trackedFiles();
  let count = 0;
  const report = (file, line, message) => {
    console.error(
      `error: ${file}:${line}: ${message} (Cargo.toml workspace.package.repository is ${url})`,
    );
    count++;
  };
  for (const file of files) {
    if (!existsSync(file) || !statSync(file).isFile()) continue;
    const bytes = readFileSync(file);
    if (bytes.subarray(0, 8000).includes(0)) continue;
    const text = bytes.toString("utf8");
    for (const { line, match } of findStaleRepoUrls(text, expected)) {
      report(
        file,
        line,
        `${JSON.stringify(match)} points to this repository at another place ` +
          "(if it is really another repository, add it to OTHER_REPOS in scripts/check-repo-url.mjs)",
      );
    }
    for (const { line, match } of findStaleManifestRepository(file, text, expected)) {
      report(file, line, `the package repository ${JSON.stringify(match)} is not this repository`);
    }
  }
  if (count > 0) process.exit(1);
}
