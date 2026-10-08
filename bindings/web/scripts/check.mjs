#!/usr/bin/env node
// @ugoe/runtime の型検査 (`pnpm --filter @ugoe/runtime check`)。
//
// ugoe-wasm の JS バインディング (src/wasm/。`pnpm --filter @ugoe/runtime build` で生成) が無くても
// 検査できるよう、無ければ手書きの型 (stub/ugoe_wasm.d.ts) を src/wasm/ に置いてから tsc を実行する。
// 生成物があればそちらで検査する (CI は生成してから検査する)。
//
// 生成物かどうかは wasm 本体 (ugoe_wasm_bg.wasm) の有無で見分ける (scripts/build.mjs と同じ)。
// 本体が無ければ src/wasm/ugoe_wasm.d.ts は前回の検査が置いたスタブの写しなので、毎回写し直す。
// 写しを残したままスタブだけが更新されると、古い型で検査して落ちるため。
//
// 生成物が今の ugoe-wasm より古ければ (scripts/wasm.mjs の指紋で見分ける)、作り直し方を警告に出す。
// `just web-check` は先に `just wasm-fresh` で作り直すので、警告が出るのは直接このスクリプトを呼んだときだけ。

import { spawnSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync } from "node:fs";
import { createRequire } from "node:module";
import { join } from "node:path";
import { staleOutputs, WASM_FILE, WEB_RUNTIME_OUT } from "../../../scripts/wasm.mjs";

const pkg = join(import.meta.dirname, "..");
const wasmDir = join(pkg, "src", "wasm");
if (!existsSync(join(wasmDir, WASM_FILE))) {
  mkdirSync(wasmDir, { recursive: true });
  copyFileSync(join(pkg, "stub", "ugoe_wasm.d.ts"), join(wasmDir, "ugoe_wasm.d.ts"));
  console.log("note: src/wasm is not generated; checking against stub/ugoe_wasm.d.ts");
} else {
  for (const { reason } of staleOutputs(undefined, [WEB_RUNTIME_OUT])) {
    console.warn(`warning: src/wasm is stale (${reason})`);
    console.warn(
      "  regenerate it with `just wasm-fresh` (or remove src/wasm to check against stub/ugoe_wasm.d.ts)",
    );
  }
}

const tsc = createRequire(import.meta.url).resolve("typescript/bin/tsc");
const r = spawnSync(process.execPath, [tsc, "-p", "tsconfig.json"], { cwd: pkg, stdio: "inherit" });
process.exitCode = r.status ?? 1;
