#!/usr/bin/env node
// 手元の OS 以外のターゲット (`--target`) で clippy を流す検査。定義の正はこのファイルの CHECKS。
// `#[cfg(windows)]`・`#[cfg(target_os = "macos")]` の中のコードは、ubuntu の CI と Linux の手元ではコンパイルされないので、
// ターゲットを変えて検査する (リンクはしないので、ターゲットの標準ライブラリだけがあればよい)。docs/testing.md#ci-のジョブ。
//
//   node scripts/cross-check.mjs run [<name>...]   検査を流す (名前を省けばすべて)。ターゲットが無ければ入れ方を出して失敗する
//                                                  (`just cross-check` / `just vcam-check`。CI の rust (cross))
//   node scripts/cross-check.mjs targets [<name>...]  要るターゲットを空白で区切って出す (CI の `rustup target add` が使う)
//   node scripts/cross-check.mjs apt [<name>...]   要る apt のパッケージを空白で区切って出す (CI の `apt-get install` が使う)
//   node scripts/cross-check.mjs check             定義のクレートがワークスペースにあるかを検査する (`just groups`)
//   node scripts/cross-check.mjs list              検査とクレートとターゲットの一覧

import { execFileSync } from "node:child_process";
import { existsSync } from "node:fs";
import { delimiter, join } from "node:path";
import { OS_CRATES } from "./ci-groups.mjs";
import { exportMbxPath } from "./lib/mbx.mjs";

/**
 * @typedef {{ crates: string[], targets: string[], features?: string[], allFeatures?: boolean, commands?: string[], apt?: string[] }} CheckDef
 */

/**
 * 検査の定義。`crates` を `targets` のそれぞれで `cargo clippy --all-targets -- -D warnings` する。
 * feature は、すべて (`--all-features`。選んだクレートの feature をすべて有効にする) なら `allFeatures: true`、
 * 一部だけなら `features` に `クレート/feature` を並べる (両方は書けない)。すべてで足りるなら `allFeatures` を使う
 * (feature を足したときの書き漏れが無い)。
 * 依存のビルドスクリプトがターゲット向けの C などをコンパイルするなら、そのツールのコマンドを `commands` に、
 * 入れる apt のパッケージを `apt` に書く (無ければ run が入れ方を出して失敗する)。
 * @type {Record<string, CheckDef>}
 */
export const CHECKS = {
  // スタジオの仮想カメラ (apps/studio/vcam、ADR 0028 の 8.)。unsafe の大半は Windows (Media Foundation) と
  // macOS (CoreMediaIO) の cfg の中にあり、`undocumented_unsafe_blocks` などの拒否はここでしか効かない
  vcam: {
    crates: ["ugoe-studio-vcam"],
    targets: ["x86_64-pc-windows-msvc", "aarch64-apple-darwin"],
  },
  // スタジオの GPU の優先度 (apps/studio/gpu-priority、ADR 0034)。unsafe はすべて Windows の cfg の中にある
  "gpu-priority": {
    crates: ["ugoe-studio-gpu-priority"],
    targets: ["x86_64-pc-windows-msvc"],
  },
  // エディタ・ビューア・スタジオの src-tauri の Windows のコード (エディタの資格情報の保存先など。T0013)。
  // 依存の ring (ugoe-ai の rustls) の C と tauri-build のリソース (.rc) はターゲット向けにコンパイルされるので、
  // MinGW の gcc と windres がある windows-gnu で検査する (msvc は MSVC の SDK が要る)。macOS は Apple の SDK が要り
  // ubuntu ではできないので、CI の rust (macos) のジョブが macOS の上で `just lint-group apps` を流す
  apps: {
    crates: ["ugoe-editor", "ugoe-viewer", "ugoe-studio"],
    targets: ["x86_64-pc-windows-gnu"],
    allFeatures: true,
    commands: ["x86_64-w64-mingw32-gcc", "x86_64-w64-mingw32-windres"],
    apt: ["gcc-mingw-w64-x86-64"],
  },
  // ugoe-ai と、CI の rust (windows) の clippy が見るクレート (ci-groups.mjs の OS_CRATES) の Windows のコード
  // (ugoe-mcp のルートの扱い・unix だけのテストの補助など。T0027・T0962)。OS_CRATES は rust (windows) のランナーでも
  // --all-targets で検査されるが、手元の Linux で同じ誤りを先に見つけるためにここでも検査する。ugoe-ai は rust (windows) に入らない。
  // 依存の ring (ugoe-ai の rustls) の C をコンパイルするので、apps と同じく MinGW の gcc がある windows-gnu で検査する。
  // feature は `just lint` と `just lint-os` の --all-features に揃える
  tools: {
    crates: ["ugoe-ai", ...OS_CRATES.filter((name) => name !== "ugoe-ai")],
    targets: ["x86_64-pc-windows-gnu"],
    allFeatures: true,
    commands: ["x86_64-w64-mingw32-gcc"],
    apt: ["gcc-mingw-w64-x86-64"],
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

/** 選んだ検査が要るツールのコマンドと apt のパッケージ (重なりを除き、出てきた順)。 */
export function requiredTools(selected) {
  return {
    commands: [...new Set(selected.flatMap(([, def]) => def.commands ?? []))],
    apt: [...new Set(selected.flatMap(([, def]) => def.apt ?? []))],
  };
}

/** 要るターゲットのうち、入っていないもの。 */
export function missingTargets(required, installed) {
  const have = new Set(installed);
  return required.filter((t) => !have.has(t));
}

/** 1 つのターゲットでの `cargo clippy` の引数。 */
export function clippyArgs(def, target) {
  const args = ["clippy", ...def.crates.flatMap((name) => ["-p", name])];
  if (def.allFeatures) args.push("--all-features");
  else if (def.features?.length) args.push("--features", def.features.join(","));
  args.push("--all-targets", "--target", target, "--", "-D", "warnings");
  return args;
}

/**
 * 定義を `cargo metadata --no-deps` の結果と比べ、問題を文字列の配列で返す (空なら問題なし)。
 * @param {Record<string, CheckDef>} checks
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
    if (def.allFeatures && def.features?.length) {
      errors.push(`${name}: allFeatures と features は両方を書けない`);
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

/** PATH にコマンドがあるか。 */
function hasCommand(command) {
  const dirs = (process.env.PATH ?? "").split(delimiter).filter(Boolean);
  const exts = process.platform === "win32" ? ["", ".exe", ".cmd", ".bat"] : [""];
  return dirs.some((dir) => exts.some((ext) => existsSync(join(dir, command + ext))));
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
  const tools = requiredTools(selected);
  const absent = tools.commands.filter((c) => !hasCommand(c));
  if (absent.length > 0) {
    console.error(`error: コマンドが無い: ${absent.join(", ")}`);
    console.error(`  入れ方 (Debian / Ubuntu): sudo apt-get install ${tools.apt.join(" ")}`);
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
  // mbx (ビルドのキャッシュ。docs/workflow/build-cache.md) の shim が入っていれば、justfile と同じく cargo を mbx に通す
  exportMbxPath();
  switch (command) {
    case "run":
    case "targets":
    case "apt": {
      let selected;
      try {
        selected = select(CHECKS, names);
      } catch (e) {
        console.error(`error: ${e.message}`);
        return 1;
      }
      if (command === "run") return run(selected);
      if (command === "apt") console.log(requiredTools(selected).apt.join(" "));
      else console.log(requiredTargets(selected).join(" "));
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
        "usage: node scripts/cross-check.mjs <run [<name>...]|targets [<name>...]|apt [<name>...]|check|list>",
      );
      return 1;
  }
}

if (import.meta.main) process.exitCode = main(process.argv.slice(2));
