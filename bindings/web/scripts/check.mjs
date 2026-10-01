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

import { spawnSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync } from "node:fs";
import { createRequire } from "node:module";
import { join } from "node:path";

const pkg = join(import.meta.dirname, "..");
const wasmDir = join(pkg, "src", "wasm");
if (!existsSync(join(wasmDir, "ugoe_wasm_bg.wasm"))) {
  mkdirSync(wasmDir, { recursive: true });
  copyFileSync(join(pkg, "stub", "ugoe_wasm.d.ts"), join(wasmDir, "ugoe_wasm.d.ts"));
  console.log("note: src/wasm is not generated; checking against stub/ugoe_wasm.d.ts");
}

const tsc = createRequire(import.meta.url).resolve("typescript/bin/tsc");
const r = spawnSync(process.execPath, [tsc, "-p", "tsconfig.json"], { cwd: pkg, stdio: "inherit" });
process.exitCode = r.status ?? 1;
