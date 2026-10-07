#!/usr/bin/env node
// fuzz のターゲットを cargo-fuzz (libFuzzer) で回す (docs/testing.md#fuzz、docs/adr/0032-fuzzing-with-cargo-fuzz.md)。
//
//   node fuzz/run.mjs <target> [秒]            # 1 つのターゲットを回す (既定は 60 秒)。`just fuzz <target> [秒]`
//   node fuzz/run.mjs all [秒]                 # すべてのターゲットを順に、それぞれ [秒] ずつ回す
//   node fuzz/run.mjs <target> [秒] -- <引数>  # `--` の後ろは libFuzzer に渡す (例: -jobs=4 -workers=4)
//   node fuzz/run.mjs --check                  # 依存のライセンスの検査・整形の検査・clippy・すべてのターゲットのビルド
//   node fuzz/run.mjs --lint                   # --check のうち stable でできるもの (ビルド以外。PR ごとの CI が流す)
//   node fuzz/run.mjs --list                   # ターゲットと種の置き場所を表示する
//
// 見つかった入力は fuzz/artifacts/<target>/ に、育てた入力の集まり (コーパス) は fuzz/corpus/<target>/ に残る (どちらも git の管理外)。
// 再現は `cargo +nightly fuzz run <target> fuzz/artifacts/<target>/<ファイル>`。直したら、その入力を broken_input.rs の
// 決まった入力に足す (docs/testing.md#fuzz)。
//
// nightly のツールチェイン (環境変数 UGOE_FUZZ_TOOLCHAIN、既定は nightly) と cargo-fuzz が要る。
// このクレートはルートのワークスペースの外にあり、fuzz/target/ でビルドする (cargo-fuzz のビルドは mbx を通さない)。

import { spawnSync } from "node:child_process";
import { mkdirSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import { cargoEnvWithMbx } from "../scripts/lib/mbx.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..");

// ターゲットと種の置き場所 (ルートからの相対。libFuzzer は置き場所の下を再帰的に読む)。
// 種は読むだけで、新しい入力は fuzz/corpus/<target>/ に書く。
const TARGETS = {
  bundle: ["assets/compat"],
  module: ["assets/compat"],
  image: ["fuzz/seeds/image"],
  json: ["spec/examples"],
  ffi: ["assets/compat", "fuzz/seeds/ffi"],
};

// libFuzzer の既定の引数: 1 つの入力に 10 秒を超えたら止まらないループとみなす。確保の上限は libFuzzer の既定 (2048 MB)。
const LIBFUZZER_ARGS = ["-timeout=10"];

const toolchain = process.env.UGOE_FUZZ_TOOLCHAIN ?? "nightly";
// mbx (docs/workflow/build-cache.md): stable の整形と clippy は just と同じく通し、nightly の cargo-fuzz のビルド
// (計装した別の成果物で、置き場を共有しても使い回せない) は通さない。
const stableEnv = cargoEnvWithMbx(process.env).env;
const fuzzEnv = { ...process.env, MBX_DISABLE: "1" };

const args = process.argv.slice(2);
const dashDash = args.indexOf("--");
const own = dashDash < 0 ? args : args.slice(0, dashDash);
const extra = dashDash < 0 ? [] : args.slice(dashDash + 1);

if (own.includes("--list")) {
  for (const [target, seeds] of Object.entries(TARGETS)) {
    console.log(`${target}\t${seeds.join(" ")}`);
  }
  process.exit(0);
}

if (own.includes("--check") || own.includes("--lint")) {
  run("node", [join(root, "scripts/check-licenses.mjs"), "--fuzz"], stableEnv);
  // 整形と clippy はルートの rust-toolchain.toml (stable) で行う (ワークスペースと同じ規則にする。libfuzzer-sys は stable でもビルドできる)。
  // clippy は libfuzzer-sys のビルドスクリプトが同梱の libFuzzer (C++) をコンパイルするので、C++ のコンパイラが要る。
  // --locked (ライセンスの検査の cargo metadata も): ランタイムのクレートに依存を足して fuzz/Cargo.lock が古くなったら、ここで気づく (fuzz/README.md)
  const manifest = ["--manifest-path", join(here, "Cargo.toml")];
  run("cargo", ["fmt", ...manifest, "--", "--check"], stableEnv);
  run("cargo", ["clippy", "--locked", ...manifest, "--", "-D", "warnings"], stableEnv);
  if (own.includes("--check")) cargoFuzz(["build"]);
  process.exit(0);
}

const [name, seconds = "60"] = own;
const names = name === "all" ? Object.keys(TARGETS) : [name];
if (name === undefined || names.some((n) => !(n in TARGETS))) {
  console.error(
    `usage: node fuzz/run.mjs <${[...Object.keys(TARGETS), "all"].join("|")}> [秒] [-- <libFuzzer の引数>]`,
  );
  process.exit(2);
}
if (!/^\d+$/.test(seconds)) {
  console.error(`error: 秒は整数で書く (${seconds})`);
  process.exit(2);
}

for (const target of names) {
  const corpus = join(here, "corpus", target);
  mkdirSync(corpus, { recursive: true });
  const seeds = TARGETS[target].map((dir) => join(root, dir));
  console.log(`== ${target}: ${seconds} 秒 (種: ${TARGETS[target].join(" ")})`);
  cargoFuzz([
    "run",
    target,
    relative(root, corpus),
    ...seeds.map((s) => relative(root, s)),
    "--",
    `-max_total_time=${seconds}`,
    ...LIBFUZZER_ARGS,
    ...extra,
  ]);
}

/** `cargo +<toolchain> fuzz <args>` をルートで実行する (cargo-fuzz はルートの fuzz/ を見つける)。 */
function cargoFuzz(fuzzArgs) {
  const probe = spawnSync("cargo", [`+${toolchain}`, "fuzz", "--version"], {
    cwd: root,
    env: fuzzEnv,
    stdio: "ignore",
  });
  if (probe.status !== 0) {
    console.error(
      [
        `error: cargo-fuzz か ${toolchain} のツールチェインが無い。次で入れる:`,
        `  rustup toolchain install ${toolchain} --profile minimal --component clippy,rustfmt`,
        "  cargo install cargo-fuzz --locked",
      ].join("\n"),
    );
    process.exit(1);
  }
  run("cargo", [`+${toolchain}`, "fuzz", ...fuzzArgs], fuzzEnv);
}

function run(command, commandArgs, env) {
  const result = spawnSync(command, commandArgs, { cwd: root, env, stdio: "inherit" });
  if (result.status !== 0) process.exit(result.status ?? 1);
}
