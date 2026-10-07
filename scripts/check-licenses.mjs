#!/usr/bin/env node
// クレートのライセンスと、依存しているクレートのライセンスを検査する。
// 方針の正は docs/adr/0018-licensing-and-1.0-scope.md。
//
// - ワークスペースのクレートは MIT。アプリ (エディタ ugoe-editor・ビューア ugoe-viewer・スタジオ ugoe-studio と
//   その仮想カメラ ugoe-studio-vcam) だけ GPL-3.0-only (docs/adr/0019-viewer-license.md・docs/adr/0028-studio.md)。
// - MIT のクレートが (dev 以外で) 依存するクレートは、寛容なライセンスで使えること。
// - アプリが依存するクレートは、GPL-3.0 と両立するライセンスで使えること。
// - npm のパッケージ bindings/web (@ugoe/runtime) は MIT で、実行時の依存は寛容なライセンスに限る
//   (docs/adr/0025-web-runtime-package.md)。
// - npm のアプリ (apps/*) と共有パッケージ (packages/*) は GPL-3.0-only で、実行時の依存 (推移的な依存を含む) は
//   GPL-3.0 と両立するライセンスに限る (docs/adr/0029-shared-ui-and-i18n.md)。package.json に SPDX の式が無い
//   依存は scripts/lib/npm-license-exceptions.mjs に、確かめた版と理由を付けて登録する。版が変わったら例外は外れ、確かめ直す。
// - 公開仕様 (spec/) は CC BY 4.0 (docs/adr/0021-public-specification.md)。spec/LICENSE が
//   CC BY 4.0 の条文であることを確かめる。
// - Godot のバインディング (bindings/godot) はルートのワークスペースの外にあり、`--godot` を付けたときだけ
//   それを検査する (docs/adr/0026-godot-binding.md)。MIT で、依存は寛容なライセンスに限る。例外として
//   godot-rust のクレートだけ MPL-2.0 を認める。`node bindings/godot/build.mjs --check` が実行する。
// - fuzz のクレート (fuzz/) もワークスペースの外にあり、`--fuzz` を付けたときだけ検査する
//   (docs/adr/0032-fuzzing-with-cargo-fuzz.md)。MIT で、依存は寛容なライセンスに限る。libfuzzer-sys だけ NCSA も認める。`node fuzz/run.mjs --check` と `--lint` が実行する。
// ライセンスが SPDX の式で書かれていない依存を追加する場合は、EXCEPTIONS に理由付きで登録する。

import { spawnSync } from "node:child_process";
import { existsSync, readdirSync, readFileSync, realpathSync } from "node:fs";
import { basename, dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { staleLockMessage } from "./lib/cargo-locked.mjs";
import { exportMbxPath } from "./lib/mbx.mjs";
import { NPM_LICENSE_EXCEPTIONS } from "./lib/npm-license-exceptions.mjs";

// mbx (ビルドのキャッシュ。docs/workflow/build-cache.md) の shim が入っていれば、justfile と同じく cargo を mbx に通す
exportMbxPath();

const REPO_ROOT = fileURLToPath(new URL("..", import.meta.url));
const EDITOR_CRATES = new Set(["ugoe-editor", "ugoe-viewer", "ugoe-studio", "ugoe-studio-vcam"]);
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

/**
 * `cargo metadata` の結果 (`manifest` を省くとルートのワークスペース)。lock のファイルが古いために `--locked` で
 * 失敗したら、古いファイルと直し方だけを出して終了コード 1 で終わる (scripts/lib/cargo-locked.mjs)。
 * ほかの失敗は cargo の出力をそのまま出して投げる。
 */
function cargoMetadata(manifest) {
  const args = ["metadata", "--format-version", "1", "--locked"];
  if (manifest) args.push("--manifest-path", manifest);
  const r = spawnSync("cargo", args, { encoding: "utf8", maxBuffer: 256 * 1024 * 1024 });
  if (r.error) throw r.error;
  const stale = r.status !== 0 ? staleLockMessage(r.stderr, REPO_ROOT) : undefined;
  if (stale !== undefined) {
    console.error(stale);
    process.exit(1);
  }
  process.stderr.write(r.stderr);
  if (r.status !== 0)
    throw new Error(`cargo ${args.join(" ")} が失敗した (終了コード ${r.status})`);
  return JSON.parse(r.stdout);
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

/**
 * ルートのワークスペースの外にある 1 つのクレート (`manifest`) を検査して終わる: クレートは MIT で、
 * dev 以外の依存は寛容なライセンスか、`extra(name)` が返すライセンス。
 */
function checkOutsideCrate(manifest, extra = () => []) {
  const meta = cargoMetadata(fileURLToPath(new URL(manifest, import.meta.url)));
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
    const allowed = new Set([...PERMISSIVE, ...extra(dep.name)]);
    if (!dep.license || !satisfies(dep.license, allowed)) {
      errors.push(`${rootPkg.name} -> ${dep.name} ${dep.version}: ${dep.license} は使えない`);
    }
  }
  for (const e of errors) console.error(`error: ${e}`);
  if (errors.length === 0)
    console.log(`licenses ok (${rootPkg.name}, ${deps.size - 1} dependencies)`);
  process.exit(errors.length > 0 ? 1 : 0);
}

if (process.argv.includes("--godot")) {
  checkOutsideCrate("../bindings/godot/Cargo.toml", (name) =>
    GODOT_RUST.has(name) ? ["MPL-2.0"] : [],
  );
}
if (process.argv.includes("--fuzz")) {
  // libfuzzer-sys の同梱の libFuzzer (LLVM) は NCSA (寛容なライセンス。ソースの見出しは Apache-2.0 WITH LLVM-exception)
  checkOutsideCrate("../fuzz/Cargo.toml", (name) => (name === "libfuzzer-sys" ? ["NCSA"] : []));
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

/** npm の package.json の license を SPDX の式として読む (古い `licenses: [{ type }]` の形も受ける)。 */
function npmLicense(manifest) {
  const { license, licenses } = manifest;
  if (typeof license === "string") return license;
  if (license && typeof license.type === "string") return license.type;
  if (Array.isArray(licenses) && licenses.length > 0) {
    return licenses.map((l) => (typeof l === "string" ? l : l.type)).join(" OR ");
  }
  return undefined;
}

/** Node の解決のしかたで、`fromDir` (パッケージの実体のディレクトリ) から依存 `name` のディレクトリを探す。 */
function resolveNpm(fromDir, name) {
  let dir = fromDir;
  for (;;) {
    if (basename(dir) !== "node_modules") {
      const candidate = join(dir, "node_modules", name, "package.json");
      if (existsSync(candidate)) return realpathSync(dirname(candidate));
    }
    const parent = dirname(dir);
    if (parent === dir) return undefined;
    dir = parent;
  }
}

/**
 * npm のパッケージ `dir` の実行時の依存 (dependencies・optionalDependencies・入っている peerDependencies) を
 * 推移的にたどり、ライセンスを `allowed` で検査する。ワークスペースのパッケージ (node_modules の外) はたどらない
 * (それぞれを別に検査する)。たどったパッケージの数を返す。
 */
function checkNpmDeps(dir, allowed, kind) {
  const root = JSON.parse(readFileSync(join(dir, "package.json"), "utf8"));
  const seen = new Set();
  const stack = [{ from: realpathSync(dir), manifest: root, optional: false, path: [root.name] }];
  while (stack.length > 0) {
    const { from, manifest, path } = stack.pop();
    const deps = [
      ...Object.keys(manifest.dependencies ?? {}).map((n) => [n, false]),
      ...Object.keys(manifest.optionalDependencies ?? {}).map((n) => [n, true]),
      ...Object.keys(manifest.peerDependencies ?? {}).map((n) => [n, true]),
    ];
    for (const [name, optional] of deps) {
      const depDir = resolveNpm(from, name);
      if (!depDir) {
        if (!optional)
          errors.push(
            `${path.join(" -> ")} -> ${name}: node_modules に無い (pnpm install を実行する)`,
          );
        continue;
      }
      if (!depDir.split(/[\\/]/).includes("node_modules")) continue; // ワークスペースのパッケージ
      if (seen.has(depDir)) continue;
      seen.add(depDir);
      const dep = JSON.parse(readFileSync(join(depDir, "package.json"), "utf8"));
      const where = `${[...path, `${dep.name} ${dep.version}`].join(" -> ")}`;
      const license = npmLicense(dep);
      const exception = NPM_LICENSE_EXCEPTIONS[dep.name];
      if (exception && !license) {
        if (exception.version !== dep.version) {
          errors.push(
            `${where}: scripts/lib/npm-license-exceptions.mjs の例外は ${exception.version} で確かめたもの。この版のライセンスを確かめ直して版を書き換える`,
          );
        } else if (!satisfies(exception.license, allowed)) {
          errors.push(`${where}: ${exception.license} は${kind}`);
        }
      } else if (!license) {
        errors.push(
          `${where}: ライセンスの式がない (確かめて scripts/lib/npm-license-exceptions.mjs に登録するか依存をやめる)`,
        );
      } else if (!satisfies(license, allowed)) {
        errors.push(`${where}: ${license} は${kind}`);
      }
      stack.push({ from: depDir, manifest: dep, path: [...path, dep.name] });
    }
  }
  return seen.size;
}

const npmPackages = [];
let npmDeps = 0;

// MIT の npm パッケージ (bindings/。docs/adr/0025-web-runtime-package.md): パッケージのライセンスが MIT で、
// 実行時の依存が推移的に寛容なライセンスで使えること。
const BINDINGS = ["bindings/web"];
for (const dir of BINDINGS) {
  const path = fileURLToPath(new URL(`../${dir}/`, import.meta.url));
  const pkg = JSON.parse(readFileSync(join(path, "package.json"), "utf8"));
  if (pkg.license !== EXPECTED.mit) {
    errors.push(`${pkg.name}: license は "${EXPECTED.mit}" であるべき (現在: ${pkg.license})`);
  }
  npmDeps += checkNpmDeps(path, PERMISSIVE, "MIT のパッケージで使えない");
  npmPackages.push(pkg.name);
}

// GPL-3.0-only の npm パッケージ (apps/*・packages/*。docs/adr/0029-shared-ui-and-i18n.md)
for (const group of ["apps", "packages"]) {
  const groupDir = fileURLToPath(new URL(`../${group}/`, import.meta.url));
  for (const entry of readdirSync(groupDir, { withFileTypes: true })) {
    const path = join(groupDir, entry.name);
    if (!entry.isDirectory() || !existsSync(join(path, "package.json"))) continue;
    const pkg = JSON.parse(readFileSync(join(path, "package.json"), "utf8"));
    if (pkg.license !== EXPECTED.editor) {
      errors.push(`${pkg.name}: license は "${EXPECTED.editor}" であるべき (現在: ${pkg.license})`);
    }
    npmDeps += checkNpmDeps(path, GPL_COMPATIBLE, "GPL-3.0 と両立しない");
    npmPackages.push(pkg.name);
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
  console.log(
    `licenses ok (${members.size} crates, ${npmPackages.length} npm packages, ${npmDeps} npm dependencies)`,
  );
process.exitCode = errors.length > 0 ? 1 : 0;
