#!/usr/bin/env node
// ランタイム層のクレートが std 無し (no_std + alloc) でビルドできるかの検査。定義の正はこのファイルの NOSTD_CRATES。
// `aarch64-unknown-none` は std そのものが無く、core と alloc がビルド済みで配られるターゲットなので、どこかで std に依存すると
// (依存の feature の統合の漏れを含めて) ビルドが失敗する。docs/adr/0031-no-std-runtime-and-host-neutral-c-abi.md、
// docs/portability.md#ランタイム層が守る制約-移植性のため。
//
//   node scripts/nostd-check.mjs run [<crate>...]  検査を流す (クレートを省けばすべて)。ターゲットが無ければ入れ方を出して失敗する
//                                                  (`just nostd-check`。CI の rust (runtime))
//   node scripts/nostd-check.mjs targets           要るターゲットを出す (CI の `rustup target add` が使う)
//   node scripts/nostd-check.mjs check             一覧のクレートがワークスペースにあり、feature `std` を既定で持つかと、
//                                                  MATH_CRATES の clippy.toml (数学関数の禁止) が揃っているかを検査する (`just groups`)
//   node scripts/nostd-check.mjs list              クレートの一覧

import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { exportMbxPath } from "./lib/mbx.mjs";

/** std の無いターゲット。 */
export const NOSTD_TARGET = "aarch64-unknown-none";

/**
 * std 無しでビルドできるクレート。ランタイム層のクレートを no_std にしたら、ここに足す (ADR 0031。T0180〜T0183・T0491)。
 * 依存の向きの順 (下の層から) に並べる。
 */
export const NOSTD_CRATES = [
  "ugoe-core",
  "ugoe-model",
  "ugoe-format",
  "ugoe-runtime",
  "ugoe-physics",
  "ugoe-motion",
  "ugoe",
];

/**
 * std の f32・f64 の数学関数を clippy の `disallowed_methods` で禁じ、`ugoe_core::math` (常に libm) を通させるクレート
 * (T0180。docs/adr/0031-no-std-runtime-and-host-neutral-c-abi.md#数学関数は-libm-に寄せる)。各クレートの `crates/<名前>/clippy.toml` は
 * 同じ内容で、ルートの clippy.toml の設定をすべて含む (clippy は最も近い clippy.toml だけを読むため)。
 */
export const MATH_CRATES = ["ugoe-core", "ugoe-runtime", "ugoe-physics", "ugoe-motion", "ugoe"];

/** クレートを省けばすべて、知らないクレートは例外。 */
export function select(crates, names) {
  if (names.length === 0) return [...crates];
  for (const name of names) {
    if (!crates.includes(name)) {
      throw new Error(`unknown crate: ${name} (${crates.join(", ")})`);
    }
  }
  return names;
}

/**
 * 1 つのクレートの `cargo clippy` の引数。feature の統合で std が紛れ込まないよう、クレートは 1 つずつ検査する。
 * テストと example は std を使うので、ライブラリだけを見る。
 */
export function clippyArgs(name, target = NOSTD_TARGET) {
  return [
    "clippy",
    "-p",
    name,
    "--lib",
    "--no-default-features",
    "--target",
    target,
    "--",
    "-D",
    "warnings",
  ];
}

/**
 * 一覧を `cargo metadata --no-deps` の結果と比べ、問題を文字列の配列で返す (空なら問題なし)。
 * @param {string[]} crates
 * @param {{ packages: Array<{ name: string, features: Record<string, string[]> }> }} metadata
 */
export function validate(crates, metadata) {
  const errors = [];
  const packages = new Map(metadata.packages.map((p) => [p.name, p]));
  if (new Set(crates).size !== crates.length) errors.push("同じクレートが 2 回ある");
  for (const name of crates) {
    const pkg = packages.get(name);
    if (pkg === undefined) {
      errors.push(`${name} はワークスペースのクレートではない`);
      continue;
    }
    if (pkg.features.std === undefined) errors.push(`${name}: feature std が無い`);
    if (!(pkg.features.default ?? []).includes("std")) {
      errors.push(`${name}: feature std が既定で有効になっていない`);
    }
  }
  return errors;
}

/**
 * MATH_CRATES の clippy.toml を検査し、問題を文字列の配列で返す (空なら問題なし)。
 * @param {string[]} crates
 * @param {string} root ルートの clippy.toml の中身
 * @param {(name: string) => string | undefined} read クレートの clippy.toml の中身 (無ければ undefined)
 */
export function validateClippyConfigs(crates, root, read) {
  const errors = [];
  const settings = root
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line !== "" && !line.startsWith("#"));
  let first;
  for (const name of crates) {
    const text = read(name);
    if (text === undefined) {
      errors.push(`${name}: clippy.toml が無い (数学関数の禁止)`);
      continue;
    }
    if (!text.includes("disallowed-methods"))
      errors.push(`${name}: clippy.toml に disallowed-methods が無い`);
    const lines = new Set(text.split("\n").map((line) => line.trim()));
    for (const setting of settings) {
      if (!lines.has(setting))
        errors.push(`${name}: clippy.toml にルートの設定 \`${setting}\` が無い`);
    }
    if (first === undefined) first = { name, text };
    else if (text !== first.text) errors.push(`${name}: clippy.toml が ${first.name} のものと違う`);
  }
  return errors;
}

function readClippyConfig(name) {
  const path = `crates/${name}/clippy.toml`;
  return existsSync(path) ? readFileSync(path, "utf8") : undefined;
}

function installedTargets() {
  // rust-toolchain.toml のツールチェインのターゲット (リポジトリの中で実行する)
  return execFileSync("rustup", ["target", "list", "--installed"], { encoding: "utf8" })
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean);
}

function metadata() {
  return JSON.parse(
    execFileSync("cargo", ["metadata", "--no-deps", "--format-version", "1"], {
      encoding: "utf8",
      maxBuffer: 64 * 1024 * 1024,
    }),
  );
}

function run(crates) {
  if (!installedTargets().includes(NOSTD_TARGET)) {
    console.error(`error: ターゲットが入っていない: ${NOSTD_TARGET}`);
    console.error(`  入れ方: rustup target add ${NOSTD_TARGET}`);
    return 1;
  }
  for (const name of crates) {
    const args = clippyArgs(name);
    console.log(`== ${name} (${NOSTD_TARGET}): cargo ${args.join(" ")}`);
    try {
      execFileSync("cargo", args, { stdio: "inherit" });
    } catch {
      console.error(`error: ${name} が std 無し (${NOSTD_TARGET}) でビルドできない`);
      return 1;
    }
  }
  return 0;
}

function main([command, ...names]) {
  // mbx (ビルドのキャッシュ。docs/workflow/build-cache.md) の shim が入っていれば、justfile と同じく cargo を mbx に通す
  exportMbxPath();
  switch (command) {
    case "run": {
      let crates;
      try {
        crates = select(NOSTD_CRATES, names);
      } catch (e) {
        console.error(`error: ${e.message}`);
        return 1;
      }
      return run(crates);
    }
    case "targets":
      console.log(NOSTD_TARGET);
      return 0;
    case "check": {
      const errors = [
        ...validate(NOSTD_CRATES, metadata()),
        ...validateClippyConfigs(
          MATH_CRATES,
          readFileSync("clippy.toml", "utf8"),
          readClippyConfig,
        ),
      ];
      for (const e of errors) console.error(`error: ${e}`);
      if (errors.length === 0) {
        console.log(
          `nostd crates ok (${NOSTD_CRATES.length} crates, clippy.toml of ${MATH_CRATES.length} crates)`,
        );
      }
      return errors.length > 0 ? 1 : 0;
    }
    case "list":
      for (const name of NOSTD_CRATES) console.log(name);
      return 0;
    default:
      console.error("usage: node scripts/nostd-check.mjs <run [<crate>...]|targets|check|list>");
      return 1;
  }
}

if (import.meta.main) process.exitCode = main(process.argv.slice(2));
