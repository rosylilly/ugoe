#!/usr/bin/env node
// クレートのライセンスと、依存しているクレートのライセンスを検査する。
// 方針の正は docs/adr/0018-licensing.md。
//
// - ワークスペースのクレートは MIT。エディタ (ugoe-editor) だけ GPL-3.0-only。
// - MIT のクレートが (dev 以外で) 依存するクレートは、寛容なライセンスで使えること。
// - エディタが依存するクレートは、GPL-3.0 と両立するライセンスで使えること。
// ライセンスが SPDX の式で書かれていない依存を追加する場合は、EXCEPTIONS に理由付きで登録する。

import { execFileSync } from "node:child_process";

const EDITOR_CRATES = new Set(["ugoe-editor"]);
const EXPECTED = { mit: "MIT", editor: "GPL-3.0-only" };

// 寛容なライセンス (MIT のクレートの依存に使ってよい)
const PERMISSIVE = new Set([
  "MIT",
  "MIT-0",
  "Apache-2.0",
  "BSD-2-Clause",
  "BSD-3-Clause",
  "ISC",
  "Zlib",
  "Unicode-3.0",
  "Unicode-DFS-2016",
  "Unlicense",
  "0BSD",
  "CC0-1.0",
  "BSL-1.0",
]);
// GPL-3.0 と両立するライセンス (エディタの依存に使ってよい)
const GPL_COMPATIBLE = new Set([
  ...PERMISSIVE,
  "MPL-2.0",
  "LGPL-2.1-or-later",
  "LGPL-3.0-only",
  "LGPL-3.0-or-later",
  "GPL-2.0-or-later",
  "GPL-3.0-only",
  "GPL-3.0-or-later",
]);
// WITH で付く例外のうち、元のライセンスの性質を変えないもの
const EXCEPTIONS_OK = new Set(["LLVM-exception"]);
// SPDX の式を持たない依存 (名前 -> 理由)
const EXCEPTIONS = {};

/** SPDX のライセンス式が、許可された集合の範囲で満たせるか。 */
function satisfies(expr, allowed) {
  const tokens = expr
    .replaceAll("/", " OR ")
    .replaceAll("(", " ( ")
    .replaceAll(")", " ) ")
    .split(/\s+/)
    .filter(Boolean);
  let i = 0;
  const primary = () => {
    const t = tokens[i++];
    if (t === "(") {
      const v = or();
      i++; // ")"
      return v;
    }
    if (tokens[i] === "WITH") {
      const exception = tokens[i + 1];
      i += 2;
      return allowed.has(t) && EXCEPTIONS_OK.has(exception);
    }
    return allowed.has(t);
  };
  const and = () => {
    let v = primary();
    while (tokens[i] === "AND") {
      i++;
      v = primary() && v;
    }
    return v;
  };
  const or = () => {
    let v = and();
    while (tokens[i] === "OR") {
      i++;
      v = and() || v;
    }
    return v;
  };
  return or();
}

const meta = JSON.parse(
  execFileSync("cargo", ["metadata", "--format-version", "1", "--locked"], {
    encoding: "utf8",
    maxBuffer: 256 * 1024 * 1024,
  }),
);
const byId = new Map(meta.packages.map((p) => [p.id, p]));
const nodes = new Map(meta.resolve.nodes.map((n) => [n.id, n]));
const members = new Set(meta.workspace_members);
const errors = [];

for (const id of members) {
  const pkg = byId.get(id);
  const editor = EDITOR_CRATES.has(pkg.name);
  const expected = editor ? EXPECTED.editor : EXPECTED.mit;
  if (pkg.license !== expected) {
    errors.push(`${pkg.name}: license は "${expected}" であるべき (現在: ${pkg.license})`);
  }
  const allowed = editor ? GPL_COMPATIBLE : PERMISSIVE;

  // dev 以外の依存をたどる
  const seen = new Set();
  const stack = [id];
  while (stack.length > 0) {
    const cur = stack.pop();
    if (seen.has(cur)) continue;
    seen.add(cur);
    for (const dep of nodes.get(cur).deps) {
      if (dep.dep_kinds.some((k) => k.kind !== "dev")) stack.push(dep.pkg);
    }
  }
  for (const depId of seen) {
    if (members.has(depId)) continue;
    const dep = byId.get(depId);
    if (!dep.license) {
      if (!(dep.name in EXCEPTIONS)) {
        errors.push(
          `${pkg.name} -> ${dep.name}: ライセンスの式がない (EXCEPTIONS に登録するか依存をやめる)`,
        );
      }
      continue;
    }
    if (!satisfies(dep.license, allowed)) {
      const kind = editor ? "GPL-3.0 と両立しない" : "MIT のクレートで使えない";
      errors.push(`${pkg.name} -> ${dep.name} ${dep.version}: ${dep.license} は${kind}`);
    }
  }
}

for (const e of [...new Set(errors)]) console.error(`error: ${e}`);
if (errors.length === 0) console.log(`licenses ok (${members.size} crates)`);
process.exitCode = errors.length > 0 ? 1 : 0;
