#!/usr/bin/env node
// 手元の OS 以外のターゲット (`--target`) で clippy を流す検査。定義の正はこのファイルの CHECKS。
// `#[cfg(windows)]`・`#[cfg(target_os = "macos")]` の中のコードは、ubuntu の CI と Linux の手元ではコンパイルされないので、
// ターゲットを変えて検査する (リンクはしないので、ターゲットの標準ライブラリだけがあればよい)。docs/testing.md#ci-のジョブ。
//
//   node scripts/cross-check.mjs run [<name>...]   検査を流す (名前を省けばすべて)。ターゲットが無ければ入れ方を出して失敗する
//                                                  (`just cross-check` / `just vcam-check`。CI の rust (cross))
//   node scripts/cross-check.mjs targets [<name>...]  要るターゲットを空白で区切って出す (CI の `rustup target add` が使う)
//   node scripts/cross-check.mjs check             定義のクレートがワークスペースにあるかを検査する (`just groups`)
//   node scripts/cross-check.mjs list              検査とクレートとターゲットの一覧

import { execFileSync } from "node:child_process";

/**
 * 検査の定義。`crates` を `targets` のそれぞれで `cargo clippy --all-targets -- -D warnings` する。
 * @type {Record<string, { crates: string[], targets: string[], features?: string[] }>}
 */
export const CHECKS = {
  // スタジオの仮想カメラ (apps/studio/vcam、ADR 0028 の 8.)。unsafe の大半は Windows (Media Foundation) と
  // macOS (CoreMediaIO) の cfg の中にあり、`undocumented_unsafe_blocks` などの拒否はここでしか効かない
  vcam: {
    crates: ["ugoe-studio-vcam"],
    targets: ["x86_64-pc-windows-msvc", "aarch64-apple-darwin"],
  },
};

/** 名前の並びから検査を選ぶ (空ならすべて)。知らない名前は例外。 */
export function select(checks, names) {
  if (names.length === 0) return Object.entries(checks);
  return names.map((name) => {
    const def = checks[name];
    if (def === undefined) {
      throw new Error(`unknown check: ${name} (${Object.keys(checks).join(", ")})`);
    }
    return [name, def];
  });
}

/** 選んだ検査が要るターゲット (重なりを除き、出てきた順)。 */
export function requiredTargets(selected) {
  return [...new Set(selected.flatMap(([, def]) => def.targets))];
}

/** 要るターゲットのうち、入っていないもの。 */
export function missingTargets(required, installed) {
  const have = new Set(installed);
  return required.filter((t) => !have.has(t));
}

/** 1 つのターゲットでの `cargo clippy` の引数。 */
export function clippyArgs(def, target) {
  const args = ["clippy", ...def.crates.flatMap((name) => ["-p", name])];
  if (def.features?.length) args.push("--features", def.features.join(","));
  args.push("--all-targets", "--target", target, "--", "-D", "warnings");
  return args;
}

/**
 * 定義を `cargo metadata --no-deps` の結果と比べ、問題を文字列の配列で返す (空なら問題なし)。
 * @param {Record<string, { crates: string[], targets: string[] }>} checks
 * @param {{ packages: Array<{ name: string }> }} metadata
 */
export function validate(checks, metadata) {
  const errors = [];
  const packages = new Set(metadata.packages.map((p) => p.name));
  for (const [name, def] of Object.entries(checks)) {
    if (def.crates.length === 0) errors.push(`${name}: クレートが 1 つもない`);
    if (def.targets.length === 0) errors.push(`${name}: ターゲットが 1 つもない`);
    for (const krate of def.crates) {
      if (!packages.has(krate)) errors.push(`${name}: ${krate} はワークスペースのクレートではない`);
    }
  }
  return errors;
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

function run(selected) {
  const missing = missingTargets(requiredTargets(selected), installedTargets());
  if (missing.length > 0) {
    console.error(`error: ターゲットが入っていない: ${missing.join(", ")}`);
    console.error(`  入れ方: rustup target add ${missing.join(" ")}`);
    return 1;
  }
  for (const [name, def] of selected) {
    for (const target of def.targets) {
      const args = clippyArgs(def, target);
      console.log(`== ${name} (${target}): cargo ${args.join(" ")}`);
      try {
        execFileSync("cargo", args, { stdio: "inherit" });
      } catch {
        console.error(`error: ${name} (${target}) の clippy が失敗した`);
        return 1;
      }
    }
  }
  return 0;
}

function main([command, ...names]) {
  switch (command) {
    case "run":
    case "targets": {
      let selected;
      try {
        selected = select(CHECKS, names);
      } catch (e) {
        console.error(`error: ${e.message}`);
        return 1;
      }
      if (command === "run") return run(selected);
      console.log(requiredTargets(selected).join(" "));
      return 0;
    }
    case "check": {
      const errors = validate(CHECKS, metadata());
      for (const e of errors) console.error(`error: ${e}`);
      if (errors.length === 0)
        console.log(`cross checks ok (${Object.keys(CHECKS).length} checks)`);
      return errors.length > 0 ? 1 : 0;
    }
    case "list": {
      for (const [name, def] of Object.entries(CHECKS))
        console.log(`${name}: ${def.crates.join(" ")} (${def.targets.join(", ")})`);
      return 0;
    }
    default:
      console.error(
        "usage: node scripts/cross-check.mjs <run [<name>...]|targets [<name>...]|check|list>",
      );
      return 1;
  }
}

if (import.meta.main) process.exitCode = main(process.argv.slice(2));
