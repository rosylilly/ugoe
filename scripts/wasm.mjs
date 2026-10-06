#!/usr/bin/env node

// `just wasm`・`just wasm-fresh`・`just wasm-bindgen-version` の実体。Windows (PowerShell) でも同じように動くよう、
// bash ではなく Node で書く。
//
//   node scripts/wasm.mjs version       Cargo.lock に記録された wasm-bindgen の版を出力する
//   node scripts/wasm.mjs build <out>   ugoe-wasm をビルドし、JS バインディングを <out> に生成する
//   node scripts/wasm.mjs check         アプリの生成物 (APP_OUTS) が古いかを確かめる。古ければ一覧を出して終了コード 1
//   node scripts/wasm.mjs fresh         アプリの生成物のうち古いものだけを作り直す (`just wasm-fresh`。web-check と pre-push の前)
//
// wasm-bindgen-cli は Cargo.lock の wasm-bindgen と同じ版でなければならない:
//   cargo install wasm-bindgen-cli --version <version> --locked
//
// 古さの判定 (T0062。docs/testing.md#wasm-の生成物の古さ): `build` は生成物の隣に、入力 (ugoe-wasm と依存する
// ワークスペースのクレートの Cargo.toml・build.rs・src/、ルートの Cargo.toml・Cargo.lock (wasm-bindgen の版を含む)・
// rust-toolchain.toml) の指紋 (FINGERPRINT_FILE) を書く。`check`・`fresh` はビルドせずに今の指紋を求めて比べる。
// 生成物の無いアプリは飛ばす (型検査は bindings/web のスタブを読む)。指紋の無い生成物は古いとみなす。

import { execFileSync, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { join, relative, resolve } from "node:path";
import { exportMbxPath } from "./lib/mbx.mjs";

const repoRoot = join(import.meta.dirname, "..");

/** アプリの生成物の場所 (git の管理外。各アプリの `pnpm wasm`)。 */
export const APP_OUTS = [
  "apps/editor/src/lib/viewport/generated",
  "apps/viewer/src/lib/wasm/generated",
  "apps/studio/src/lib/wasm/generated",
];
/** 生成物の隣に書く指紋のファイルの名前。 */
export const FINGERPRINT_FILE = "ugoe_wasm.fingerprint";
/** 指紋の求め方を変えたら上げる (前の求め方の指紋を古いとみなすため)。 */
const FINGERPRINT_VERSION = "1";
/** 指紋に入れるルートのファイル。 */
const ROOT_INPUTS = ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml"];

/** Cargo.lock に記録された wasm-bindgen の版。見つからなければ undefined。 */
export function lockedVersion(root = repoRoot) {
  const lock = readFileSync(join(root, "Cargo.lock"), "utf8");
  return lock.match(/^name = "wasm-bindgen"\r?\nversion = "([^"]+)"/m)?.[1];
}

/** ルートの Cargo.toml の `[workspace.dependencies]` のうち、path を持つもの (名前 -> ディレクトリ)。 */
function workspaceCrateDirs(root) {
  const toml = readFileSync(join(root, "Cargo.toml"), "utf8");
  const dirs = new Map();
  for (const m of toml.matchAll(/^([\w-]+)\s*=\s*\{[^}\n]*\bpath\s*=\s*"([^"]+)"/gm)) {
    dirs.set(m[1], m[2]);
  }
  return dirs;
}

/** Cargo.toml の dev 以外の依存の節 (`[dependencies]`・`[build-dependencies]`・`[target.'…'.dependencies]`) にある名前。 */
export function nonDevDependencies(toml) {
  const names = new Set();
  let inDeps = false;
  for (const line of toml.split(/\r?\n/)) {
    const header = line.match(/^\s*\[([^\]]+)\]/);
    if (header) {
      inDeps = /(^|\.)(build-)?dependencies$/.test(header[1].trim());
      continue;
    }
    const dep = inDeps && line.match(/^\s*([\w-]+)\s*[=.]/);
    if (dep) names.add(dep[1]);
  }
  return names;
}

/** ugoe-wasm と、(dev 以外で) 依存するワークスペースのクレートのディレクトリ (ルートからの相対。並べ替え済み)。 */
export function wasmCrateDirs(root = repoRoot) {
  const dirs = workspaceCrateDirs(root);
  const seen = new Set();
  const queue = ["ugoe-wasm"];
  while (queue.length > 0) {
    const name = queue.pop();
    if (seen.has(name)) continue;
    seen.add(name);
    const dir = dirs.get(name) ?? `crates/${name}`;
    for (const dep of nonDevDependencies(readFileSync(join(root, dir, "Cargo.toml"), "utf8"))) {
      if (dirs.has(dep) && !seen.has(dep)) queue.push(dep);
    }
  }
  return [...seen].map((name) => dirs.get(name) ?? `crates/${name}`).sort();
}

/** ディレクトリの下のファイル (再帰。ルートからの相対で `/` 区切り)。 */
function walk(root, dir, out) {
  const abs = join(root, dir);
  if (!existsSync(abs)) return;
  for (const entry of readdirSync(abs, { withFileTypes: true })) {
    const path = `${dir}/${entry.name}`;
    if (entry.isDirectory()) walk(root, path, out);
    else if (entry.isFile()) out.push(path);
  }
}

/** 指紋に入れるファイル (ルートからの相対。並べ替え済み)。テスト・例・README は wasm の生成物に効かないので入れない。 */
export function inputFiles(root = repoRoot) {
  const files = ROOT_INPUTS.filter((f) => existsSync(join(root, f)));
  for (const dir of wasmCrateDirs(root)) {
    for (const f of ["Cargo.toml", "build.rs"]) {
      if (existsSync(join(root, dir, f))) files.push(`${dir}/${f}`);
    }
    walk(root, `${dir}/src`, files);
  }
  return files.sort();
}

/** 今の入力の指紋 (SHA-256 の 16 進)。ビルドせずに求める。 */
export function fingerprint(root = repoRoot) {
  const hash = createHash("sha256");
  hash.update(`ugoe-wasm fingerprint ${FINGERPRINT_VERSION}\0`);
  for (const file of inputFiles(root)) {
    hash.update(`${file}\0`);
    hash.update(readFileSync(join(root, file)));
    hash.update("\0");
  }
  return hash.digest("hex");
}

/**
 * アプリの生成物のうち古いもの。生成物の無い (ディレクトリが無いか空の) ものは飛ばす。
 * @returns {{ out: string, reason: string }[]}
 */
export function staleOutputs(root = repoRoot, outs = APP_OUTS, current = fingerprint(root)) {
  const stale = [];
  for (const out of outs) {
    const dir = join(root, out);
    if (!existsSync(dir) || !statSync(dir).isDirectory() || readdirSync(dir).length === 0) continue;
    const file = join(dir, FINGERPRINT_FILE);
    if (!existsSync(file)) {
      stale.push({ out, reason: `no ${FINGERPRINT_FILE} (generated before it was recorded)` });
    } else if (readFileSync(file, "utf8").trim() !== current) {
      stale.push({ out, reason: "ugoe-wasm or its inputs changed since it was generated" });
    }
  }
  return stale;
}

/** 導入されている wasm-bindgen-cli の版。見つからなければ null。 */
function installedVersion() {
  const r = spawnSync("wasm-bindgen", ["--version"], { encoding: "utf8" });
  if (r.status !== 0) return null;
  return r.stdout.trim().split(/\s+/)[1] ?? null;
}

/** コマンドを実行し、失敗したらその終了コードで終える。 */
function run(cmd, args) {
  const r = spawnSync(cmd, args, { cwd: repoRoot, stdio: "inherit" });
  if (r.error) {
    console.error(`error: failed to run ${cmd}: ${r.error.message}`);
    process.exit(1);
  }
  if (r.status !== 0) process.exit(r.status ?? 1);
}

/** wasm-bindgen-cli の版が Cargo.lock と合っていなければ、入れ方を出して終える。 */
function requireWasmBindgen(hint) {
  const want = lockedVersion();
  if (!want) {
    console.error("error: wasm-bindgen is not found in Cargo.lock");
    process.exit(1);
  }
  const have = installedVersion();
  if (have !== want) {
    console.error(`error: wasm-bindgen-cli ${want} is required (found: ${have ?? "none"}).`);
    console.error(`  cargo install wasm-bindgen-cli --version ${want} --locked`);
    if (hint) console.error(hint);
    process.exit(1);
  }
}

/** ugoe-wasm を 1 回ビルドし、JS バインディングと指紋を `outs` のそれぞれに書く。 */
function build(outs, hint) {
  requireWasmBindgen(hint);
  // mbx (ビルドのキャッシュ。docs/workflow/build-cache.md) の shim が入っていれば、justfile と同じく cargo を mbx に通す
  // (pre-push のように just を通らずに呼ばれても)
  exportMbxPath();
  // ビルドの前に求める (ビルドの間にソースが変われば、次の確かめで古いとみなされる)
  const print = fingerprint();
  run("cargo", ["build", "-p", "ugoe-wasm", "--target", "wasm32-unknown-unknown", "--release"]);
  // CARGO_TARGET_DIR などで target/ の場所が変わっていても追えるよう、cargo に尋ねる
  const { target_directory } = JSON.parse(
    execFileSync("cargo", ["metadata", "--no-deps", "--format-version", "1"], {
      cwd: repoRoot,
      encoding: "utf8",
      maxBuffer: 64 * 1024 * 1024,
    }),
  );
  const wasm = join(target_directory, "wasm32-unknown-unknown", "release", "ugoe_wasm.wasm");
  for (const out of outs) {
    run("wasm-bindgen", ["--target", "web", "--out-dir", out, wasm]);
    writeFileSync(join(resolve(repoRoot, out), FINGERPRINT_FILE), `${print}\n`);
  }
}

function reportStale(stale) {
  for (const { out, reason } of stale) console.error(`  ${out}: ${reason}`);
}

function main() {
  const [command, out] = process.argv.slice(2);
  if (command === "version") {
    const version = lockedVersion();
    if (!version) {
      console.error("error: wasm-bindgen is not found in Cargo.lock");
      process.exit(1);
    }
    console.log(version);
  } else if (command === "build" && out) {
    build([out]);
  } else if (command === "check") {
    const stale = staleOutputs();
    if (stale.length > 0) {
      console.error("error: the generated ugoe-wasm bindings are stale:");
      reportStale(stale);
      console.error("regenerate them with `just wasm-fresh` (or `just wasm <out>`)");
      process.exit(1);
    }
    console.log("the generated ugoe-wasm bindings are up to date");
  } else if (command === "fresh") {
    const stale = staleOutputs();
    if (stale.length === 0) {
      console.log("the generated ugoe-wasm bindings are up to date");
      return;
    }
    console.log("regenerating the stale ugoe-wasm bindings:");
    for (const { out, reason } of stale) console.log(`  ${out}: ${reason}`);
    const outs = stale.map((s) => s.out);
    build(
      outs,
      `or remove the stale ${outs.length > 1 ? "directories" : "directory"} (the type check then reads bindings/web/stub):\n` +
        outs.map((o) => `  rm -r ${relative(process.cwd(), join(repoRoot, o)) || o}`).join("\n"),
    );
  } else {
    console.error("usage: node scripts/wasm.mjs version | build <out-dir> | check | fresh");
    process.exit(2);
  }
}

if (import.meta.main ?? process.argv[1] === import.meta.filename) main();
