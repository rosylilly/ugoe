#!/usr/bin/env node
// ランタイム層のクレートが std 無し (no_std + alloc) でビルドできるかの検査。定義の正はこのファイルの NOSTD_CRATES。
// `aarch64-unknown-none` は std そのものが無く、core と alloc がビルド済みで配られるターゲットなので、どこかで std に依存すると
// (依存の feature の統合の漏れを含めて) ビルドが失敗する。docs/adr/0031-no-std-runtime-and-host-neutral-c-abi.md、
// docs/portability.md#ランタイム層が守る制約-移植性のため。
// 32 ビットで 64 ビットのアトミック操作の無い `thumbv7em-none-eabihf` でも同じ clippy を流し、ugoe-ffi の静的ライブラリを作る
// (`AtomicU64` のような、64 ビットの機械では見つからない退行を見つける。T0495・T0070)。
//
//   node scripts/nostd-check.mjs run [<crate>...]  検査を流す (クレートを省けばすべて)。ターゲットが無ければ入れ方を出して失敗する
//                                                  (`just nostd-check`。CI の rust (runtime))。HOST_TEST_CRATES では std の無い形の
//                                                  単体テストをホストで流し (T0186・T0491)、ugoe-ffi では静的ライブラリを
//                                                  C の見本とリンクする検査も流す (T0491。crates/ugoe-ffi/README.md#std-の無い環境)
//   node scripts/nostd-check.mjs targets           要るターゲット (NOSTD_TARGETS) を出す (CI の `rustup target add` が使う)
//   node scripts/nostd-check.mjs check             一覧のクレートがワークスペースにあり、feature `std` を既定で持つかと、
//                                                  HOST_TEST_CRATES が開発時の依存に自分を置いていないかと、
//                                                  MATH_CRATES の clippy.toml (数学関数の禁止) が揃っているかを検査する (`just groups`)
//   node scripts/nostd-check.mjs list              クレートの一覧
//   node scripts/nostd-check.mjs build-std [<target>...]
//                                                  core と alloc が配られない tier 3 のターゲット (BUILD_STD_TARGETS。省けばすべて) で、
//                                                  nightly の `-Zbuild-std` で ugoe-ffi の静的ライブラリを作り、未解決のシンボルが
//                                                  HOST_SYMBOLS と同じかを llvm-nm で確かめる (`just nostd-build-std`。nightly と
//                                                  rust-src と llvm-tools が要る。CI は週に 1 度の nostd-build-std.yml。T0495・T0071)

import { execFileSync, spawnSync } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { exportMbxPath } from "./lib/mbx.mjs";

/** std の無いターゲット。ugoe-ffi の静的ライブラリを C の見本とリンクするのはこのターゲットだけ。 */
export const NOSTD_TARGET = "aarch64-unknown-none";

/**
 * 64 ビットのアトミック操作の無い std の無いターゲット (32 ビットの Arm Cortex-M4F。ポインタの幅の比較と交換はある)。
 * rustup が stable で core と alloc を配る tier 2 のターゲットで、ゲーム機の 32 ビットのターゲット (tier 3) の代わりに検査する (T0070)。
 * NOSTD_TARGET と同じ clippy を流す (`cfg(target_has_atomic = "64")` で分けたコードの、こちら側の枝も lint にかけるため。
 * 型検査までなので、ビルドだけと比べて時間はほとんど変わらない)。ugoe-ffi は静的ライブラリを作るところまでで、リンクはしない。
 */
export const ATOMIC32_TARGET = "thumbv7em-none-eabihf";

/** `just nostd-check` が要るターゲット (`targets` が出す。CI の `rustup target add`)。 */
export const NOSTD_TARGETS = [NOSTD_TARGET, ATOMIC32_TARGET];

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
  "ugoe-wasm",
];

/**
 * std の無い形 (`--no-default-features`) の単体テストをホストで流すクレート。テストのコードも clippy にかける。
 * 開発時の依存で自分の feature を有効にすると std の無い形にならないので、そのクレートは開発時の依存に自分を置かない
 * (ugoe-format は `png` を使う結合テストを `required-features` にしている。T0186)。NOSTD_CRATES の順に並べる。
 */
export const HOST_TEST_CRATES = ["ugoe-format", "ugoe-ffi"];

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

/**
 * core と alloc がビルド済みで配られない tier 3 のターゲット。nightly の `-Zbuild-std` で ugoe-ffi の静的ライブラリを作る (T0495・T0071)。
 * 今のゲーム機と同じ 64 ビットの AArch64 と、64 ビットのアトミック操作の無い 32 ビットの Arm (crates/ugoe-ffi/README.md の表)。
 */
export const BUILD_STD_TARGETS = ["aarch64-nintendo-switch-freestanding", "armv6k-nintendo-3ds"];

/** `build-std` の nightly のツールチェイン (`UGOE_NIGHTLY_TOOLCHAIN` で変えられる。fuzz/run.mjs の `UGOE_FUZZ_TOOLCHAIN` と同じ形)。 */
export const NIGHTLY_TOOLCHAIN = process.env.UGOE_NIGHTLY_TOOLCHAIN ?? "nightly";

/**
 * `build-std` の成果物の置き場。stable の `just nostd-check`・`just ffi-check` と同じ target/ のプロファイル `nostd` に置くと、
 * ホストのビルドスクリプトなどが rustc の版の違いで作り直し合うので分ける。
 */
export const BUILD_STD_TARGET_DIR = join("target", "build-std");

/** 名前を省けばすべて、知らない名前は例外 (`kind` は例外のメッセージの種類)。 */
export function select(crates, names, kind = "crate") {
  if (names.length === 0) return [...crates];
  for (const name of names) {
    if (!crates.includes(name)) {
      throw new Error(`unknown ${kind}: ${name} (${crates.join(", ")})`);
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
 * std の無い形の単体テストのコードをホストで検査する `cargo clippy` の引数。`--profile test` でライブラリを `cfg(test)` で検査する
 * (`--tests` は結合テストと、テストでない形のライブラリも作るので使わない。後者は ugoe-ffi ではホストで作れない)。
 */
export function hostClippyArgs(name) {
  return [
    "clippy",
    "-p",
    name,
    "--lib",
    "--profile",
    "test",
    "--no-default-features",
    "--",
    "-D",
    "warnings",
  ];
}

/** std の無い形の単体テストをホストで流す `cargo test` の引数。 */
export function hostTestArgs(name) {
  return ["test", "-p", name, "--lib", "--no-default-features"];
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

/**
 * tier 3 のターゲットの静的ライブラリを nightly の `-Zbuild-std` で作る `cargo` の引数 (crates/ugoe-ffi/README.md の手順と同じ)。
 * `compiler-builtins-mem` で `memcpy` などを compiler_builtins に持たせ、未解決のシンボルを HOST_SYMBOLS だけにする。
 */
export function buildStdArgs(target, toolchain = NIGHTLY_TOOLCHAIN) {
  return [
    `+${toolchain}`,
    ...staticlibArgs(target),
    "--target-dir",
    BUILD_STD_TARGET_DIR,
    "-Zbuild-std=core,alloc",
    "-Zbuild-std-features=compiler-builtins-mem",
  ];
}

/** `llvm-nm -j` の出力のシンボルの名前 (アーカイブのメンバの見出し `<名前>.o:` と空行を除く)。 */
function nmNames(output) {
  return output
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line !== "" && !line.endsWith(":"));
}

/**
 * 静的ライブラリの未解決のシンボル。`llvm-nm -j --undefined-only` と `--defined-only` の出力から、どのメンバも定義しないものを
 * 名前の順に返す (メンバ同士で解決するものは除く)。リンクの `--gc-sections` と違って使われない関数の参照も残るが、
 * `compiler-builtins-mem` を付ければ HOST_SYMBOLS と同じになる (T0495)。
 */
export function archiveUndefinedSymbols(undefinedOutput, definedOutput) {
  const defined = new Set(nmNames(definedOutput));
  return [...new Set(nmNames(undefinedOutput))].filter((name) => !defined.has(name)).sort();
}

/** 決めた一覧 (HOST_SYMBOLS) との違い。`extra` は一覧に無い未解決のシンボル、`missing` は一覧にあって無いもの。 */
export function symbolDiff(symbols, expected = HOST_SYMBOLS) {
  return {
    extra: symbols.filter((name) => !expected.includes(name)),
    missing: expected.filter((name) => !symbols.includes(name)),
  };
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
 * HOST_TEST_CRATES を検査し、問題を文字列の配列で返す (空なら問題なし)。NOSTD_CRATES に含まれ、開発時の依存に自分を
 * 置いていない (置くとその feature がテストで有効になり、std の無い形にならない。T0186) こと。
 * @param {string[]} hostTests
 * @param {string[]} crates
 * @param {{ packages: Array<{ name: string, dependencies?: Array<{ name: string, kind: string | null }> }> }} metadata
 */
export function validateHostTests(hostTests, crates, metadata) {
  const errors = [];
  const packages = new Map(metadata.packages.map((p) => [p.name, p]));
  for (const name of hostTests) {
    if (!crates.includes(name)) errors.push(`HOST_TEST_CRATES: ${name} は NOSTD_CRATES に無い`);
    const self = (packages.get(name)?.dependencies ?? []).some(
      (d) => d.name === name && d.kind === "dev",
    );
    if (self) errors.push(`${name}: 開発時の依存に自分があり、std の無い形の単体テストにならない`);
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
 * HOST_TEST_CRATES の 1 つのクレートの、std の無い形の単体テストのコードの clippy とテストをホストで流す (T0186・T0491)。
 * ホストには std があるので、テストの側は std を使ってよい。ライブラリの feature が std の無い形になっていることを確かめる。
 */
function checkHostTests(name) {
  for (const args of [hostClippyArgs(name), hostTestArgs(name)]) {
    console.log(`== ${name}: std の無い形の単体テスト (ホスト): cargo ${args.join(" ")}`);
    try {
      execFileSync("cargo", args, { stdio: "inherit" });
    } catch {
      console.error(`error: ${name} の std の無い形の単体テストが失敗した`);
      return 1;
    }
  }
  return 0;
}

/** std の無い形の静的ライブラリを作る。失敗すれば 1 を返す。 */
function buildStaticlib(target) {
  const args = staticlibArgs(target);
  console.log(`== ${FFI_CRATE} (${target}, staticlib): cargo ${args.join(" ")}`);
  try {
    execFileSync("cargo", args, { stdio: "inherit" });
  } catch {
    console.error(`error: ${FFI_CRATE} の静的ライブラリを std 無し (${target}) で作れない`);
    return 1;
  }
  return 0;
}

/**
 * ugoe-ffi の std の無い形の検査 (T0491): aarch64-unknown-none の静的ライブラリを作って
 * C の見本 (crates/ugoe-ffi/examples/c/nostd.c) とリンクし、未解決のシンボルが HOST_SYMBOLS だけであることを確かめる。
 * 実機での動作は確かめない (crates/ugoe-ffi/README.md#std-の無い環境)。
 */
function checkFfiStaticlib() {
  if (buildStaticlib(NOSTD_TARGET) !== 0) return 1;
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

/** nightly のツールチェインの sysroot。ツールチェインが無ければ undefined。 */
function nightlySysroot(toolchain) {
  const out = spawnSync("rustc", [`+${toolchain}`, "--print", "sysroot"], { encoding: "utf8" });
  return out.status === 0 ? out.stdout.trim() : undefined;
}

/** nightly の llvm-tools の llvm-nm (rustc と同じ LLVM)。`LLVM_NM` で指定できる。 */
function llvmNm(toolchain, sysroot) {
  if (process.env.LLVM_NM) return process.env.LLVM_NM;
  const host = execFileSync("rustc", [`+${toolchain}`, "-vV"], { encoding: "utf8" }).match(
    /^host: (\S+)$/m,
  )?.[1];
  const exe = process.platform === "win32" ? "llvm-nm.exe" : "llvm-nm";
  return join(sysroot, "lib", "rustlib", host ?? "", "bin", exe);
}

/**
 * tier 3 のターゲットで ugoe-ffi の静的ライブラリを `-Zbuild-std` で作り、未解決のシンボルが HOST_SYMBOLS と同じかを確かめる
 * (T0071)。ターゲットごとに続け、失敗したターゲットの数を返す (道具が無ければ 1)。C の見本とのリンクはしない (32 ビットの
 * ターゲットごとに C の浮動小数点の ABI を合わせる必要があるため。リンクは stable の aarch64-unknown-none で `run` が確かめる)。
 */
function buildStd(targets, toolchain = NIGHTLY_TOOLCHAIN) {
  const sysroot = nightlySysroot(toolchain);
  const install = [
    `  入れ方: rustup toolchain install ${toolchain} --profile minimal --component rust-src,llvm-tools`,
    `         (入っていれば rustup component add rust-src llvm-tools --toolchain ${toolchain})`,
  ];
  if (sysroot === undefined) {
    console.error(
      `error: ツールチェイン ${toolchain} が無い (UGOE_NIGHTLY_TOOLCHAIN で指定できる)`,
    );
    for (const line of install) console.error(line);
    return 1;
  }
  if (!existsSync(join(sysroot, "lib", "rustlib", "src", "rust", "library"))) {
    console.error(`error: ${toolchain} に rust-src (core と alloc のソース) が無い`);
    for (const line of install) console.error(line);
    return 1;
  }
  const nm = llvmNm(toolchain, sysroot);
  if (spawnSync(nm, ["--version"]).status !== 0) {
    console.error(
      `error: llvm-nm (${nm}) が無い (${toolchain} の llvm-tools。LLVM_NM で指定できる)`,
    );
    for (const line of install) console.error(line);
    return 1;
  }
  // mbx (docs/workflow/build-cache.md) は nightly の -Z の付いたビルドを検証できずに止まるので通さない (fuzz/run.mjs と同じ)
  const env = { ...process.env, MBX_DISABLE: "1" };
  const failed = [];
  for (const target of targets) {
    const args = buildStdArgs(target, toolchain);
    console.log(`== ${FFI_CRATE} (${target}, staticlib, -Zbuild-std): cargo ${args.join(" ")}`);
    if (spawnSync("cargo", args, { stdio: "inherit", env }).status !== 0) {
      console.error(`error: ${FFI_CRATE} の静的ライブラリを ${target} で作れない`);
      failed.push(target);
      continue;
    }
    const lib = join(BUILD_STD_TARGET_DIR, target, "nostd", "libugoe_ffi.a");
    const list = (flag) =>
      spawnSync(nm, ["-j", flag, lib], { encoding: "utf8", maxBuffer: 64 * 1024 * 1024 });
    const undefinedOut = list("--undefined-only");
    const definedOut = list("--defined-only");
    if (undefinedOut.status !== 0 || definedOut.status !== 0) {
      console.error(`${undefinedOut.stderr}${definedOut.stderr}`);
      console.error(`error: ${lib} のシンボルを llvm-nm で読めない`);
      failed.push(target);
      continue;
    }
    const symbols = archiveUndefinedSymbols(undefinedOut.stdout, definedOut.stdout);
    const { extra, missing } = symbolDiff(symbols);
    if (extra.length > 0 || missing.length > 0) {
      console.error(`error: ${target} の未解決のシンボルが決めた一覧 (HOST_SYMBOLS) と違う`);
      if (extra.length > 0) console.error(`  一覧に無いもの: ${extra.join(", ")}`);
      if (missing.length > 0) console.error(`  一覧にあって無いもの: ${missing.join(", ")}`);
      failed.push(target);
      continue;
    }
    console.log(`build-std ok (${target}): unresolved symbols are ${symbols.join(", ")}`);
  }
  if (failed.length > 0) console.error(`error: build-std failed: ${failed.join(", ")}`);
  return failed.length;
}

/** 入っていないターゲット。 */
export function missingTargets(installed, targets = NOSTD_TARGETS) {
  return targets.filter((target) => !installed.includes(target));
}

function run(crates) {
  const missing = missingTargets(installedTargets());
  if (missing.length > 0) {
    console.error(`error: ターゲットが入っていない: ${missing.join(", ")}`);
    console.error(`  入れ方: rustup target add ${missing.join(" ")}`);
    return 1;
  }
  for (const name of crates) {
    for (const target of NOSTD_TARGETS) {
      const args = clippyArgs(name, target);
      console.log(`== ${name} (${target}): cargo ${args.join(" ")}`);
      try {
        execFileSync("cargo", args, { stdio: "inherit" });
      } catch {
        console.error(`error: ${name} が std 無し (${target}) でビルドできない`);
        return 1;
      }
    }
  }
  for (const name of HOST_TEST_CRATES) {
    if (crates.includes(name) && checkHostTests(name) !== 0) return 1;
  }
  if (crates.includes(FFI_CRATE)) {
    if (buildStaticlib(ATOMIC32_TARGET) !== 0) return 1;
    return checkFfiStaticlib();
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
      for (const target of NOSTD_TARGETS) console.log(target);
      return 0;
    case "check": {
      const meta = metadata();
      const errors = [
        ...validate(NOSTD_CRATES, meta),
        ...validateHostTests(HOST_TEST_CRATES, NOSTD_CRATES, meta),
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
    case "build-std": {
      let targets;
      try {
        targets = select(BUILD_STD_TARGETS, names, "target");
      } catch (e) {
        console.error(`error: ${e.message}`);
        return 1;
      }
      return buildStd(targets) > 0 ? 1 : 0;
    }
    default:
      console.error(
        "usage: node scripts/nostd-check.mjs <run [<crate>...]|targets|check|list|build-std [<target>...]>",
      );
      return 1;
  }
}

if (import.meta.main) process.exitCode = main(process.argv.slice(2));
