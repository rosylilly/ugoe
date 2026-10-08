#!/usr/bin/env node
// `just ffi-header` と `just ffi-check` の実体 (crates/ugoe-ffi/README.md#ヘッダの生成)。
//
//   node scripts/ffi.mjs header   cbindgen で crates/ugoe-ffi/include/ugoe.h を作り直す
//   node scripts/ffi.mjs check    ヘッダが最新かを検査し、C の見本をビルドして実行する。std の無い形の静的ライブラリも
//                                 ホストの C ライブラリとリンクして実行する (crates/ugoe-ffi/README.md#std-の無い環境)
//
// cbindgen は MPL-2.0 なので、ワークスペースの依存には入れず CLI として使う。版は CBINDGEN_VERSION に揃える:
//   cargo install cbindgen --version <CBINDGEN_VERSION> --locked

import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { exportMbxPath } from "./lib/mbx.mjs";

// mbx (ビルドのキャッシュ。docs/workflow/build-cache.md) の shim が入っていれば、justfile と同じく cargo を mbx に通す
exportMbxPath();

const CBINDGEN_VERSION = "0.29.4";

const root = join(import.meta.dirname, "..");
const crate = join(root, "crates", "ugoe-ffi");
const header = join(crate, "include", "ugoe.h");

/** コマンドを実行し、失敗したらその終了コードで終える。 */
function run(cmd, args, options = {}) {
  const r = spawnSync(cmd, args, { cwd: root, stdio: "inherit", ...options });
  if (r.error) {
    console.error(`error: failed to run ${cmd}: ${r.error.message}`);
    process.exit(1);
  }
  if (r.status !== 0) process.exit(r.status ?? 1);
  return r;
}

/** cbindgen の版を確かめてから、ヘッダを `out` に生成する。 */
function generate(out) {
  const r = spawnSync("cbindgen", ["--version"], { encoding: "utf8" });
  const installed = r.status === 0 ? r.stdout.trim().split(/\s+/)[1] : null;
  if (installed !== CBINDGEN_VERSION) {
    console.error(
      `error: cbindgen ${CBINDGEN_VERSION} is required (found: ${installed ?? "none"}).\n` +
        `  cargo install cbindgen --version ${CBINDGEN_VERSION} --locked`,
    );
    process.exit(1);
  }
  run("cbindgen", [
    "--config",
    join(crate, "cbindgen.toml"),
    "--crate",
    "ugoe-ffi",
    "--output",
    out,
    crate,
  ]);
}

function check() {
  const tmp = mkdtempSync(join(tmpdir(), "ugoe-ffi-"));
  try {
    const fresh = join(tmp, "ugoe.h");
    generate(fresh);
    if (readFileSync(fresh, "utf8") !== readFileSync(header, "utf8")) {
      console.error("error: crates/ugoe-ffi/include/ugoe.h is out of date; run `just ffi-header`");
      process.exit(1);
    }
    console.log("header ok");
  } finally {
    rmSync(tmp, { recursive: true, force: true });
  }

  // C の見本を静的ライブラリにリンクして実行する。
  run("cargo", ["build", "-p", "ugoe-ffi"]);
  const out = join(root, "target", "ffi-sample");
  mkdirSync(out, { recursive: true });
  const windows = process.platform === "win32";
  const exe = join(out, windows ? "sample.exe" : "sample");
  const lib = join(root, "target", "debug", windows ? "ugoe_ffi.lib" : "libugoe_ffi.a");
  const system = {
    darwin: ["-framework", "CoreFoundation"],
    linux: ["-lm", "-lpthread", "-ldl"],
    // `cargo rustc -p ugoe-ffi --lib --crate-type staticlib -- --print native-static-libs` が x86_64-pc-windows-msvc で出す
    // `kernel32.lib ntdll.lib userenv.lib ws2_32.lib dbghelp.lib /defaultlib:msvcrt` に合わせる。最後の C ライブラリは
    // `-fms-runtime-lib=dll` で揃える (clang の既定は静的な libcmt で、Rust の側の msvcrt とぶつかる)。
    // `-fms-runtime-lib` が効くのはコンパイル (オブジェクトに埋める `/DEFAULTLIB:msvcrt.lib`) だけで、clang のドライバは
    // リンクの行に `-defaultlib:libcmt` を必ず足す。MSVC の link.exe はそれで静的な libucrt を選び、見本の `__imp_fopen` などが
    // 解決できずに落ちる (lld-link は通るので cargo-xwin では見えない。T0079)。`-nodefaultlib:libcmt` でその既定を外す。
    // `_CRT_SECURE_NO_WARNINGS` は、MSVC の C ライブラリが標準の `fopen` などに付ける非推奨の警告 (-Werror で止まる) を消す
    win32: [
      "-fms-runtime-lib=dll",
      "-Wl,-nodefaultlib:libcmt",
      "-D_CRT_SECURE_NO_WARNINGS",
      "-lkernel32",
      "-lntdll",
      "-luserenv",
      "-lws2_32",
      "-ldbghelp",
    ],
  };
  // Windows では `cl` ではなく clang (gcc 形式の引数のドライバ) を使う: clang は Visual Studio と Windows SDK を自分で
  // 探すので、開発者のシェル (vcvars) の外でも MSVC のライブラリとリンクでき、警告の扱い (-Werror -pedantic) を Linux・macOS と
  // 同じ引数で書ける。CI のランナー (windows-latest) には LLVM が入っている。CC で替えるときも gcc 形式の引数のドライバにする
  run(process.env.CC ?? (windows ? "clang" : "cc"), [
    "-std=c11",
    "-Wall",
    "-Wextra",
    "-Werror",
    "-pedantic",
    "-I",
    join(crate, "include"),
    join(crate, "examples", "c", "sample.c"),
    lib,
    ...(system[process.platform] ?? []),
    "-o",
    exe,
  ]);
  // 見本のモデル、法線マップ付きの見本 (ライティング)、着せ替えの見本 (本体とモジュール) で動かす。
  const samples = join(root, "assets", "samples");
  const dressUp = join(samples, "dress-up");
  const modules = readdirSync(dressUp)
    .filter((f) => f.endsWith(".ugoemod"))
    .sort()
    .map((f) => join(dressUp, f));
  run(exe, [join(samples, "avatar.ugoe")]);
  run(exe, ["--lit", join(samples, "avatar-lit.ugoe")]);
  run(exe, [join(dressUp, "dress-up.ugoe"), ...modules]);

  checkNostd([
    join(samples, "avatar.ugoe"),
    join(samples, "avatar-lit.ugoe"),
    join(dressUp, "dress-up.ugoe"),
  ]);
}

/**
 * std の無い形 (`--no-default-features`、プロファイル `nostd`) の静的ライブラリをホスト向けに作り、フリースタンディングの
 * 見本 (examples/c/nostd.c) とホストの C ライブラリ (アロケータとパニックの処理が呼ぶ `malloc`・`free`・`abort` など) と
 * リンクして実行する。aarch64-unknown-none でのリンクの検査は `just nostd-check` (scripts/nostd-check.mjs)。
 * Windows (MSVC) の C ライブラリは `aligned_alloc` を持たないので飛ばす (std の無い形は std の無い環境のためのもの)。
 */
function checkNostd(models) {
  if (process.platform === "win32") {
    console.log("skip: the std-less static library is not checked on Windows");
    return;
  }
  run("cargo", [
    "rustc",
    "-p",
    "ugoe-ffi",
    "--lib",
    "--crate-type",
    "staticlib",
    "--no-default-features",
    // わざとパニックする `ugoe_test_panic` を足す (ヘッダには出ない。ほかは配る形と同じ)
    "--features",
    "panic-test",
    "--profile",
    "nostd",
  ]);
  const out = join(root, "target", "ffi-sample");
  const exe = join(out, "nostd-sample");
  const examples = join(crate, "examples", "c");
  run(process.env.CC ?? "cc", [
    "-std=c11",
    "-Wall",
    "-Wextra",
    "-Werror",
    "-pedantic",
    "-I",
    join(crate, "include"),
    join(examples, "nostd.c"),
    join(examples, "nostd_main.c"),
    join(root, "target", "nostd", "libugoe_ffi.a"),
    "-o",
    exe,
  ]);
  for (const model of models) run(exe, [model]);
  checkNostdPanic(exe);
}

/**
 * std の無い形でパニックしたら、`ugoe_set_panic_handler` で登録したホストの関数にメッセージと位置が届き、その後で
 * ホストの `abort` で止まることを確かめる (crates/ugoe-ffi/README.md#パニックのメッセージをホストに渡す-t0494)。
 */
function checkNostdPanic(exe) {
  const r = spawnSync(exe, ["--panic"], { cwd: root, encoding: "utf8" });
  const expected =
    /^nostd-sample: ugoe panicked: panicked at \S*src\/nostd\.rs:\d+:\d+: test panic requested by the host \(42\)$/m;
  const problems = [];
  if (r.signal !== "SIGABRT")
    problems.push(`expected SIGABRT, got status ${r.status} signal ${r.signal}`);
  if (!expected.test(r.stderr)) problems.push("the panic message did not reach the host handler");
  if (/not NUL-terminated|returned/.test(r.stderr)) problems.push("unexpected output");
  if (problems.length > 0) {
    console.error(`error: ${exe} --panic: ${problems.join("; ")}\n--- stderr ---\n${r.stderr}`);
    process.exit(1);
  }
  console.log("nostd panic handler ok");
}

const [command] = process.argv.slice(2);
if (command === "header") {
  generate(header);
} else if (command === "check") {
  check();
} else {
  console.error("usage: node scripts/ffi.mjs header|check");
  process.exit(2);
}
