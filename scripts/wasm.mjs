#!/usr/bin/env node
// `just wasm` と `just wasm-bindgen-version` の実体。Windows (PowerShell) でも同じように動くよう、
// bash ではなく Node で書く。
//
//   node scripts/wasm.mjs version       Cargo.lock に記録された wasm-bindgen の版を出力する
//   node scripts/wasm.mjs build <out>   ugoe-wasm をビルドし、JS バインディングを <out> に生成する
//
// wasm-bindgen-cli は Cargo.lock の wasm-bindgen と同じ版でなければならない:
//   cargo install wasm-bindgen-cli --version <version> --locked

import { execFileSync, spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { exportMbxPath } from "./lib/mbx.mjs";

// mbx (ビルドのキャッシュ。docs/workflow/build-cache.md) の shim が入っていれば、justfile と同じく cargo を mbx に通す
exportMbxPath();

const root = join(import.meta.dirname, "..");

/** Cargo.lock に記録された wasm-bindgen の版。 */
function lockedVersion() {
  const lock = readFileSync(join(root, "Cargo.lock"), "utf8");
  const m = lock.match(/^name = "wasm-bindgen"\r?\nversion = "([^"]+)"/m);
  if (!m) {
    console.error("error: wasm-bindgen is not found in Cargo.lock");
    process.exit(1);
  }
  return m[1];
}

/** 導入されている wasm-bindgen-cli の版。見つからなければ null。 */
function installedVersion() {
  const r = spawnSync("wasm-bindgen", ["--version"], { encoding: "utf8" });
  if (r.status !== 0) return null;
  return r.stdout.trim().split(/\s+/)[1] ?? null;
}

/** コマンドを実行し、失敗したらその終了コードで終える。 */
function run(cmd, args) {
  const r = spawnSync(cmd, args, { cwd: root, stdio: "inherit" });
  if (r.error) {
    console.error(`error: failed to run ${cmd}: ${r.error.message}`);
    process.exit(1);
  }
  if (r.status !== 0) process.exit(r.status ?? 1);
}

function build(out) {
  const want = lockedVersion();
  const have = installedVersion();
  if (have !== want) {
    console.error(`error: wasm-bindgen-cli ${want} is required (found: ${have ?? "none"}).`);
    console.error(`  cargo install wasm-bindgen-cli --version ${want} --locked`);
    process.exit(1);
  }
  run("cargo", ["build", "-p", "ugoe-wasm", "--target", "wasm32-unknown-unknown", "--release"]);
  // CARGO_TARGET_DIR などで target/ の場所が変わっていても追えるよう、cargo に尋ねる
  const { target_directory } = JSON.parse(
    execFileSync("cargo", ["metadata", "--no-deps", "--format-version", "1"], {
      cwd: root,
      encoding: "utf8",
      maxBuffer: 64 * 1024 * 1024,
    }),
  );
  const wasm = join(target_directory, "wasm32-unknown-unknown", "release", "ugoe_wasm.wasm");
  run("wasm-bindgen", ["--target", "web", "--out-dir", out, wasm]);
}

const [command, out] = process.argv.slice(2);
if (command === "version") {
  console.log(lockedVersion());
} else if (command === "build" && out) {
  build(out);
} else {
  console.error("usage: node scripts/wasm.mjs version | build <out-dir>");
  process.exit(2);
}
