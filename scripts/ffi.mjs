#!/usr/bin/env node
// `just ffi-header` と `just ffi-check` の実体 (crates/ugoe-ffi/README.md#ヘッダの生成)。
//
//   node scripts/ffi.mjs header   cbindgen で crates/ugoe-ffi/include/ugoe.h を作り直す
//   node scripts/ffi.mjs check    ヘッダが最新かを検査し、C の見本をビルドして実行する
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
  const system = { darwin: ["-framework", "CoreFoundation"], linux: ["-lm", "-lpthread", "-ldl"] };
  run(process.env.CC ?? "cc", [
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
