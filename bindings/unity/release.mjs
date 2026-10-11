#!/usr/bin/env node
// UPM パッケージ com.ugoe.unity のリリースの補助 (bindings/unity/README.md#リリース)。公開 (配る場所に置く) は利用者が行う。
//
//   node bindings/unity/release.mjs check [<tag>]   package.json の版がクレートの版と揃っているか (タグ `v<版>` を渡すとタグとも比べる)
//   node bindings/unity/release.mjs sync            package.json の版をクレートの版 (Cargo.toml の workspace.package.version) にする
//   node bindings/unity/release.mjs pack <dir> [--allow-missing]
//       # com.ugoe.unity/ を UPM の tarball (<dir>/com.ugoe.unity-<版>.tgz、中は package/。Package Manager の
//       # "Add package from tarball..." で入る) にし、SHA-256 を <dir>/com.ugoe.unity-<版>.tgz.sha256 に書く。
//       # 先に build.mjs --plugins でネイティブライブラリと見本のモデルを並べておく。すべてのプラットフォームの
//       # ライブラリ (build.mjs の PLUGINS) が無ければ失敗する (--allow-missing で手元の OS の分だけでも作る)。
//
// 版の正はルートの Cargo.toml の `[workspace.package] version` (すべての配布物で揃える。docs/portability.md#配布)。

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { basename, join, resolve } from "node:path";
import { PLUGINS, pkg } from "./build.mjs";

const root = join(import.meta.dirname, "..", "..");
const manifestPath = join(pkg, "package.json");

/** ルートの Cargo.toml の `[workspace.package] version`。 */
export function crateVersion(toml = readFileSync(join(root, "Cargo.toml"), "utf8")) {
  const section = toml.match(/^\[workspace\.package\]\r?\n([\s\S]*?)(?=^\[|(?![\s\S]))/m)?.[1];
  const version = section?.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
  if (!version) throw new Error("Cargo.toml has no [workspace.package] version");
  return version;
}

/** 版の食い違いを文の配列で返す (揃っていれば空)。`tag` は Git のタグ (`v0.1.0`。省略可)。 */
export function versionProblems({ packageVersion, crate, tag }) {
  const problems = [];
  if (packageVersion !== crate) {
    problems.push(
      `com.ugoe.unity/package.json version ${packageVersion} differs from the crates' version ${crate} ` +
        "(run: node bindings/unity/release.mjs sync)",
    );
  }
  if (tag && tag !== `v${crate}`) {
    problems.push(`tag ${tag} does not match the crates' version ${crate} (expected v${crate})`);
  }
  return problems;
}

/** tarball に入っているべきファイル (package/ からのパス)。無いものを返す。 */
export function missingEntries(entries, { allowMissing }) {
  const have = new Set(entries.map((e) => e.replace(/^package\//, "")));
  const required = [
    "package.json",
    "LICENSE.md",
    "Third Party Notices.md",
    "README.md",
    "Runtime/Ugoe.Runtime.asmdef",
  ];
  if (!allowMissing) {
    for (const p of PLUGINS)
      required.push(`Runtime/Plugins/${p.path}`, `Runtime/Plugins/${p.path}.meta`);
  }
  const missing = required.filter((r) => !have.has(r));
  if (allowMissing && ![...have].some((e) => e.startsWith("Runtime/Plugins/"))) {
    missing.push("Runtime/Plugins/ (no native library at all)");
  }
  if (![...have].some((e) => e.startsWith("Samples~/Basic/Models/")))
    missing.push("Samples~/Basic/Models/");
  return missing;
}

function readManifest() {
  return JSON.parse(readFileSync(manifestPath, "utf8"));
}

function check(tag) {
  const manifest = readManifest();
  const problems = versionProblems({
    packageVersion: manifest.version,
    crate: crateVersion(),
    tag,
  });
  for (const p of problems) console.error(`error: ${p}`);
  if (problems.length > 0) process.exit(1);
  console.log(`com.ugoe.unity ${manifest.version}`);
}

function sync() {
  const text = readFileSync(manifestPath, "utf8");
  const version = crateVersion();
  // 整形 (キーの順・インデント) を保つため、version の行だけを書き換える
  writeFileSync(manifestPath, text.replace(/^(\s*"version":\s*)"[^"]*"/m, `$1"${version}"`));
  console.log(`com.ugoe.unity/package.json version: ${version}`);
}

function pack(outDir, allowMissing) {
  if (!outDir) throw new Error("pack needs an output directory");
  const out = resolve(outDir);
  mkdirSync(out, { recursive: true });
  // npm pack は package/ の下に入れ、時刻を固定した tarball を作る (UPM の tarball と同じ形)。
  // com.ugoe.unity/ には .gitignore・.npmignore を置かないので、Runtime/Plugins/ と Samples~/Basic/Models/ も入る。
  const result = JSON.parse(
    execFileSync("npm", ["pack", "--json", "--pack-destination", out], {
      cwd: pkg,
      encoding: "utf8",
      shell: process.platform === "win32",
    }),
  );
  const file = join(out, result[0].filename);
  const entries = result[0].files.map((f) => f.path);
  const missing = missingEntries(entries, { allowMissing });
  if (missing.length > 0) {
    for (const m of missing) console.error(`missing in the tarball: ${m}`);
    console.error(
      "place the native libraries and samples first (node bindings/unity/build.mjs --plugins <dir>)",
    );
    process.exit(1);
  }
  const sha = createHash("sha256").update(readFileSync(file)).digest("hex");
  writeFileSync(`${file}.sha256`, `${sha}  ${basename(file)}\n`);
  console.log(`wrote ${file} (${entries.length} files, sha256 ${sha})`);
}

if (import.meta.main) {
  const [command, arg, ...rest] = process.argv.slice(2);
  if (command === "check") check(arg);
  else if (command === "sync") sync();
  else if (command === "pack") pack(arg, rest.includes("--allow-missing"));
  else {
    console.error(
      "usage: node bindings/unity/release.mjs check [<tag>] | sync | pack <dir> [--allow-missing]",
    );
    process.exit(2);
  }
}
