#!/usr/bin/env node
// ランタイム層のクレートが std 無し (no_std + alloc) でビルドできるかの検査。定義の正はこのファイルの NOSTD_CRATES。
// `aarch64-unknown-none` は std そのものが無く、core と alloc がビルド済みで配られるターゲットなので、どこかで std に依存すると
// (依存の feature の統合の漏れを含めて) ビルドが失敗する。docs/adr/0031-no-std-runtime-and-host-neutral-c-abi.md、
// docs/portability.md#ランタイム層が守る制約-移植性のため。
//
//   node scripts/nostd-check.mjs run [<crate>...]  検査を流す (クレートを省けばすべて)。ターゲットが無ければ入れ方を出して失敗する
//                                                  (`just nostd-check`。CI の rust (runtime))。ugoe-ffi では、std の無い形の
//                                                  単体テスト (ホスト) と、静的ライブラリを C の見本とリンクする検査も流す
//                                                  (T0491。crates/ugoe-ffi/README.md#std-の無い環境)
//   node scripts/nostd-check.mjs targets           要るターゲットを出す (CI の `rustup target add` が使う)
//   node scripts/nostd-check.mjs check             一覧のクレートがワークスペースにあり、feature `std` を既定で持つかと、
//                                                  MATH_CRATES の clippy.toml (数学関数の禁止) が揃っているかを検査する (`just groups`)
//   node scripts/nostd-check.mjs list              クレートの一覧

import { execFileSync, spawnSync } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
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
  "ugoe-ffi",
];

/**
 * std の f32・f64 の数学関数を clippy の `disallowed_methods` で禁じ、`ugoe_core::math` (常に libm) を通させるクレート
 * (T0180。docs/adr/0031-no-std-runtime-and-host-neutral-c-abi.md#数学関数は-libm-に寄せる)。各クレートの `crates/<名前>/clippy.toml` は
 * 同じ内容で、ルートの clippy.toml の設定をすべて含む (clippy は最も近い clippy.toml だけを読むため)。
 */
export const MATH_CRATES = [
  "ugoe-core",
  "ugoe-format",
  "ugoe-runtime",
  "ugoe-physics",
  "ugoe-motion",
  "ugoe",
  "ugoe-ffi",
];

/** 静的ライブラリをリンクして調べる C ABI のクレート (T0491)。 */
export const FFI_CRATE = "ugoe-ffi";

/**
 * std の無い形の ugoe-ffi の静的ライブラリが、ホストに解決を任せてよいシンボル (ホストの C ライブラリの関数)。
 * アロケータ (crates/ugoe-ffi/src/nostd.rs) が呼ぶ確保の関数と、パニックの処理が呼ぶ `abort`。
 * `memcpy` などは aarch64-unknown-none の compiler_builtins が持つので入らない。
 */
export const HOST_SYMBOLS = ["abort", "aligned_alloc", "calloc", "free", "malloc", "realloc"];

/** HOST_SYMBOLS の C の宣言 (形だけの定義を作るのに使う)。 */
const HOST_STUBS = {
  abort: "_Noreturn void abort(void) { for (;;) {} }",
  aligned_alloc: "void *aligned_alloc(size_t a, size_t n) { (void)a; (void)n; return NULL; }",
  calloc: "void *calloc(size_t c, size_t n) { (void)c; (void)n; return NULL; }",
  free: "void free(void *p) { (void)p; }",
  malloc: "void *malloc(size_t n) { (void)n; return NULL; }",
  realloc: "void *realloc(void *p, size_t n) { (void)p; (void)n; return NULL; }",
};

/** C の見本をコンパイルするターゲット (clang の `--target`)。NOSTD_TARGET と同じ機械。 */
export const C_TARGET = "aarch64-none-elf";

/** C の見本 (crates/ugoe-ffi/examples/c/nostd.c) の入口。リンクの起点にする。 */
export const SAMPLE_ENTRY = "ugoe_nostd_sample";

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
 * std の無い形の静的ライブラリを作る `cargo rustc` の引数。巻き戻さないプロファイル `nostd` (ルートの Cargo.toml) で、
 * 作る種類を staticlib だけにする (cdylib はこのターゲットに無い)。
 */
export function staticlibArgs(target = NOSTD_TARGET) {
  return [
    "rustc",
    "-p",
    FFI_CRATE,
    "--lib",
    "--crate-type",
    "staticlib",
    "--no-default-features",
    "--profile",
    "nostd",
    "--target",
    target,
  ];
}

/** リンカ (lld) の出力から、未解決のシンボルの名前を並べて返す (重なりを除き、名前の順)。 */
export function undefinedSymbols(output) {
  const names = new Set();
  for (const match of output.matchAll(/undefined symbol: (\S+)/g)) names.add(match[1]);
  return [...names].sort();
}

/** ホストの C ライブラリのもの (HOST_SYMBOLS) でない未解決のシンボル。 */
export function unexpectedSymbols(symbols, allowed = HOST_SYMBOLS) {
  return symbols.filter((name) => !allowed.includes(name));
}

/** HOST_SYMBOLS を形だけ定義する C のソース (ホストの C ライブラリの代わり。リンクが通ることを確かめる)。 */
export function hostStubSource(symbols = HOST_SYMBOLS) {
  const lines = [
    "/* ホストの C ライブラリの代わり (scripts/nostd-check.mjs が作る)。 */",
    "#include <stddef.h>",
  ];
  for (const name of symbols) lines.push(HOST_STUBS[name]);
  return `${lines.join("\n")}\n`;
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

/** rustup のツールチェインに入っている lld (rust-lld)。 */
function rustLld() {
  const sysroot = execFileSync("rustc", ["--print", "sysroot"], { encoding: "utf8" }).trim();
  const host = execFileSync("rustc", ["-vV"], { encoding: "utf8" }).match(/^host: (\S+)$/m)?.[1];
  const exe = process.platform === "win32" ? "rust-lld.exe" : "rust-lld";
  return join(sysroot, "lib", "rustlib", host ?? "", "bin", exe);
}

/**
 * ugoe-ffi の std の無い形の検査 (T0491): ホストで単体テストを流し、aarch64-unknown-none の静的ライブラリを作って
 * C の見本 (crates/ugoe-ffi/examples/c/nostd.c) とリンクし、未解決のシンボルが HOST_SYMBOLS だけであることを確かめる。
 * 実機での動作は確かめない (crates/ugoe-ffi/README.md#std-の無い環境)。
 */
function checkFfiStaticlib() {
  console.log(`== ${FFI_CRATE}: std の無い形の単体テスト (ホスト)`);
  try {
    execFileSync("cargo", ["test", "-p", FFI_CRATE, "--lib", "--no-default-features"], {
      stdio: "inherit",
    });
  } catch {
    console.error(`error: ${FFI_CRATE} の std の無い形の単体テストが失敗した`);
    return 1;
  }
  const args = staticlibArgs();
  console.log(`== ${FFI_CRATE} (${NOSTD_TARGET}, staticlib): cargo ${args.join(" ")}`);
  try {
    execFileSync("cargo", args, { stdio: "inherit" });
  } catch {
    console.error(`error: ${FFI_CRATE} の静的ライブラリを std 無し (${NOSTD_TARGET}) で作れない`);
    return 1;
  }
  const clang = process.env.CLANG ?? "clang";
  if (spawnSync(clang, ["--version"]).status !== 0) {
    console.error(
      `error: C の見本を ${C_TARGET} 向けにコンパイルする clang が無い (CLANG で指定できる)`,
    );
    console.error("  入れ方: sudo apt-get install clang (macOS は Xcode の clang でよい)");
    return 1;
  }
  const lld = rustLld();
  const lib = join("target", NOSTD_TARGET, "nostd", "libugoe_ffi.a");
  const tmp = mkdtempSync(join(tmpdir(), "ugoe-ffi-nostd-"));
  try {
    const cc = (source, out) =>
      spawnSync(
        clang,
        [
          `--target=${C_TARGET}`,
          "-ffreestanding",
          "-fno-builtin",
          "-std=c11",
          "-Wall",
          "-Wextra",
          "-Werror",
          "-pedantic",
          "-O2",
          "-I",
          join("crates", FFI_CRATE, "include"),
          "-c",
          source,
          "-o",
          out,
        ],
        { stdio: "inherit" },
      ).status === 0;
    const sample = join(tmp, "nostd.o");
    if (!cc(join("crates", FFI_CRATE, "examples", "c", "nostd.c"), sample)) {
      console.error("error: C の見本 (nostd.c) をコンパイルできない");
      return 1;
    }
    const link = (inputs) =>
      spawnSync(
        lld,
        [
          "-flavor",
          "gnu",
          "--gc-sections",
          "-e",
          SAMPLE_ENTRY,
          "--error-limit=0",
          "-o",
          join(tmp, "sample.elf"),
          ...inputs,
        ],
        { encoding: "utf8" },
      );
    // ホストの C ライブラリ無しでリンクし、未解決のシンボルを集める
    const bare = link([sample, lib]);
    const symbols = undefinedSymbols(`${bare.stdout}${bare.stderr}`);
    const unexpected = unexpectedSymbols(symbols);
    if (unexpected.length > 0) {
      console.error(`${bare.stdout}${bare.stderr}`);
      console.error(
        `error: ホストの C ライブラリのもの以外の未解決のシンボルがある: ${unexpected.join(", ")}`,
      );
      return 1;
    }
    // HOST_SYMBOLS を形だけ定義すればリンクが通る (ほかの原因で失敗しない) ことを確かめる
    const stubSource = join(tmp, "host.c");
    writeFileSync(stubSource, hostStubSource());
    const stub = join(tmp, "host.o");
    if (!cc(stubSource, stub)) return 1;
    const full = link([sample, lib, stub]);
    if (full.status !== 0) {
      console.error(`${full.stdout}${full.stderr}`);
      console.error(`error: ${FFI_CRATE} の静的ライブラリを C の見本とリンクできない`);
      return 1;
    }
    console.log(
      `staticlib ok: unresolved symbols are host C library functions only (${symbols.join(", ")})`,
    );
    return 0;
  } finally {
    rmSync(tmp, { recursive: true, force: true });
  }
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
  if (crates.includes(FFI_CRATE)) return checkFfiStaticlib();
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
