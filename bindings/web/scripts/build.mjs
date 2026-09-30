#!/usr/bin/env node
// @ugoe/runtime をビルドする (`pnpm --filter @ugoe/runtime build`、`just web-runtime`)。
//
// 1. ugoe-wasm をビルドし、JS バインディングを src/wasm/ に生成する (リポジトリの scripts/wasm.mjs。gitignore 済み)
// 2. TypeScript を dist/ に出力する
// 3. 生成したバインディングと wasm を dist/wasm/ に写す
//
// `--no-wasm` を付けると 1 を飛ばし、src/wasm/ にある生成物を使う。

import { spawnSync } from "node:child_process";
import { cpSync, existsSync, rmSync } from "node:fs";
import { createRequire } from "node:module";
import { join } from "node:path";

const pkg = join(import.meta.dirname, "..");
const root = join(pkg, "..", "..");
const generated = join(pkg, "src", "wasm");
const dist = join(pkg, "dist");

/** コマンドを実行し、失敗したらその終了コードで終える。 */
function run(cmd, args) {
  const r = spawnSync(cmd, args, { cwd: pkg, stdio: "inherit" });
  if (r.error) {
    console.error(`error: failed to run ${cmd}: ${r.error.message}`);
    process.exit(1);
  }
  if (r.status !== 0) process.exit(r.status ?? 1);
}

if (!process.argv.includes("--no-wasm")) {
  run(process.execPath, [join(root, "scripts", "wasm.mjs"), "build", generated]);
}
if (!existsSync(join(generated, "ugoe_wasm_bg.wasm"))) {
  console.error("error: src/wasm/ugoe_wasm_bg.wasm is missing (build without --no-wasm)");
  process.exit(1);
}

rmSync(dist, { recursive: true, force: true });
const tsc = createRequire(import.meta.url).resolve("typescript/bin/tsc");
run(process.execPath, [tsc, "-p", "tsconfig.build.json"]);
cpSync(generated, join(dist, "wasm"), { recursive: true });
console.log(`built ${dist}`);
