#!/usr/bin/env node
// Godot のバインディング (bindings/godot) が依存するクレートのライセンスを検査する。
// 方針の正は docs/adr/0026-godot-binding.md。
//
// - このクレートはルートのワークスペースの外にあるので、scripts/check-licenses.mjs (`just licenses`) の対象にならない。
// - バインディング自体は MIT。依存は寛容なライセンスで使えること。例外として godot-rust (gdext) のクレートだけ
//   MPL-2.0 を認める (MPL はファイル単位のコピーレフトなので、このクレートのコードは MIT のまま配れる)。
// - SPDX の式の読み方は scripts/check-licenses.mjs と同じ。

import { execFileSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));

// 寛容なライセンス (scripts/check-licenses.mjs の PERMISSIVE と同じ)
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
  "CDLA-Permissive-2.0",
  "BSL-1.0",
]);
const EXCEPTIONS_OK = new Set(["LLVM-exception"]);
// MPL-2.0 を認める godot-rust のクレート (gdextension-api は Godot の API の定義を収めたクレート)
const GODOT_RUST = new Set([
  "gdextension-api",
  "godot",
  "godot-bindings",
  "godot-cell",
  "godot-codegen",
  "godot-core",
  "godot-ffi",
  "godot-macros",
]);
const GODOT_RUST_ALLOWED = new Set([...PERMISSIVE, "MPL-2.0"]);

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
  execFileSync(
    "cargo",
    ["metadata", "--format-version", "1", "--locked", "--manifest-path", join(here, "Cargo.toml")],
    { encoding: "utf8", maxBuffer: 256 * 1024 * 1024 },
  ),
);
const byId = new Map(meta.packages.map((p) => [p.id, p]));
const nodes = new Map(meta.resolve.nodes.map((n) => [n.id, n]));
const errors = [];

const [rootId] = meta.workspace_members;
const rootPkg = byId.get(rootId);
if (rootPkg.license !== "MIT") {
  errors.push(`${rootPkg.name}: license は "MIT" であるべき (現在: ${rootPkg.license})`);
}
// dev 以外の依存をたどる
const seen = new Set();
const stack = [rootId];
while (stack.length > 0) {
  const cur = stack.pop();
  if (seen.has(cur)) continue;
  seen.add(cur);
  for (const dep of nodes.get(cur).deps) {
    if (dep.dep_kinds.some((k) => k.kind !== "dev")) stack.push(dep.pkg);
  }
}
for (const id of seen) {
  if (id === rootId) continue;
  const dep = byId.get(id);
  if (!dep.license) {
    errors.push(`${dep.name}: ライセンスの式がない`);
    continue;
  }
  const allowed = GODOT_RUST.has(dep.name) ? GODOT_RUST_ALLOWED : PERMISSIVE;
  if (!satisfies(dep.license, allowed)) {
    errors.push(`${rootPkg.name} -> ${dep.name} ${dep.version}: ${dep.license} は使えない`);
  }
}

for (const e of errors) console.error(`error: ${e}`);
if (errors.length === 0) console.log(`licenses ok (${seen.size - 1} dependencies)`);
process.exitCode = errors.length > 0 ? 1 : 0;
