#!/usr/bin/env node
// クレートのライセンスと、依存しているクレートのライセンスを検査する。
// 方針の正は docs/adr/0018-licensing.md。
//
// - ワークスペースのクレートは MIT。アプリ (エディタ ugoe-editor・ビューア ugoe-viewer) だけ GPL-3.0-only
//   (docs/adr/0019-viewer-license.md)。
// - MIT のクレートが (dev 以外で) 依存するクレートは、寛容なライセンスで使えること。
// - アプリが依存するクレートは、GPL-3.0 と両立するライセンスで使えること。
// - npm のパッケージ bindings/web (@ugoe/runtime) は MIT で、実行時の依存は寛容なライセンスに限る
//   (docs/adr/0025-web-runtime-package.md)。
// - 公開仕様 (spec/) は CC BY 4.0 (docs/adr/0021-public-specification.md)。spec/LICENSE が
//   CC BY 4.0 の条文であることを確かめる。
// - Godot のバインディング (bindings/godot) はルートのワークスペースの外にあり、`--godot` を付けたときだけ
//   それを検査する (docs/adr/0026-godot-binding.md)。MIT で、依存は寛容なライセンスに限る。例外として
//   godot-rust のクレートだけ MPL-2.0 を認める。`node bindings/godot/build.mjs --check` が実行する。
// ライセンスが SPDX の式で書かれていない依存を追加する場合は、EXCEPTIONS に理由付きで登録する。

import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const EDITOR_CRATES = new Set(["ugoe-editor", "ugoe-viewer"]);
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
  // webpki-roots (TLS のルート証明書) のデータ用ライセンス。コピーレフトはなく、条件は再配布時に条文を示すことだけ
  "CDLA-Permissive-2.0",
  "BSL-1.0",
]);
// GPL-3.0 と両立するライセンス (アプリの依存に使ってよい)
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
// MPL-2.0 を認める godot-rust のクレート (--godot のときだけ。gdextension-api は Godot の API の定義を収めたクレート)
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

/** `cargo metadata` の結果 (`manifest` を省くとルートのワークスペース)。 */
function cargoMetadata(manifest) {
  const args = ["metadata", "--format-version", "1", "--locked"];
  if (manifest) args.push("--manifest-path", manifest);
  return JSON.parse(
    execFileSync("cargo", args, { encoding: "utf8", maxBuffer: 256 * 1024 * 1024 }),
  );
}

/** `id` から dev 以外の依存をたどったパッケージの ID の集合 (`id` 自身を含む)。 */
function runtimeDeps(nodes, id) {
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
  return seen;
}

if (process.argv.includes("--godot")) {
  const meta = cargoMetadata(
    fileURLToPath(new URL("../bindings/godot/Cargo.toml", import.meta.url)),
  );
  const byId = new Map(meta.packages.map((p) => [p.id, p]));
  const nodes = new Map(meta.resolve.nodes.map((n) => [n.id, n]));
  const errors = [];
  const [rootId] = meta.workspace_members;
  const rootPkg = byId.get(rootId);
  if (rootPkg.license !== EXPECTED.mit) {
    errors.push(
      `${rootPkg.name}: license は "${EXPECTED.mit}" であるべき (現在: ${rootPkg.license})`,
    );
  }
  const deps = runtimeDeps(nodes, rootId);
  for (const id of deps) {
    if (id === rootId) continue;
    const dep = byId.get(id);
    const allowed = GODOT_RUST.has(dep.name) ? new Set([...PERMISSIVE, "MPL-2.0"]) : PERMISSIVE;
    if (!dep.license || !satisfies(dep.license, allowed)) {
      errors.push(`${rootPkg.name} -> ${dep.name} ${dep.version}: ${dep.license} は使えない`);
    }
  }
  for (const e of errors) console.error(`error: ${e}`);
  if (errors.length === 0)
    console.log(`licenses ok (${rootPkg.name}, ${deps.size - 1} dependencies)`);
  process.exit(errors.length > 0 ? 1 : 0);
}

const meta = cargoMetadata();
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

  for (const depId of runtimeDeps(nodes, id)) {
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

// MIT の npm パッケージ (bindings/。docs/adr/0025-web-runtime-package.md): パッケージのライセンスが MIT で、
// 実行時の依存 (dependencies) が寛容なライセンスで使えること。依存のライセンスは node_modules の package.json で読む。
const BINDINGS = ["bindings/web"];
for (const dir of BINDINGS) {
  const base = new URL(`../${dir}/`, import.meta.url);
  const pkg = JSON.parse(readFileSync(new URL("package.json", base), "utf8"));
  if (pkg.license !== EXPECTED.mit) {
    errors.push(`${pkg.name}: license は "${EXPECTED.mit}" であるべき (現在: ${pkg.license})`);
  }
  for (const dep of Object.keys(pkg.dependencies ?? {})) {
    const manifest = new URL(`node_modules/${dep}/package.json`, base);
    if (!existsSync(manifest)) {
      errors.push(`${pkg.name} -> ${dep}: node_modules に無い (pnpm install を実行する)`);
      continue;
    }
    const license = JSON.parse(readFileSync(manifest, "utf8")).license;
    if (typeof license !== "string" || !satisfies(license, PERMISSIVE)) {
      errors.push(`${pkg.name} -> ${dep}: ${license} は MIT のパッケージで使えない`);
    }
  }
}

// 公開仕様のライセンス (CC BY 4.0 の条文) が置かれていること。
const SPEC_LICENSE = new URL("../spec/LICENSE", import.meta.url);
const specLicense = existsSync(SPEC_LICENSE) ? readFileSync(SPEC_LICENSE, "utf8") : "";
if (
  !specLicense.startsWith("Attribution 4.0 International") ||
  !specLicense.includes("Creative Commons Attribution 4.0 International Public License")
) {
  errors.push("spec/LICENSE: CC BY 4.0 の条文であるべき");
}

for (const e of [...new Set(errors)]) console.error(`error: ${e}`);
if (errors.length === 0)
  console.log(`licenses ok (${members.size} crates, ${BINDINGS.length} npm packages)`);
process.exitCode = errors.length > 0 ? 1 : 0;
