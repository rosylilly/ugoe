#!/usr/bin/env node
// @ugoe/runtime の型検査 (`pnpm --filter @ugoe/runtime check`)。
//
// ugoe-wasm の JS バインディング (src/wasm/。`pnpm --filter @ugoe/runtime build` で生成) が無くても
// 検査できるよう、無ければ手書きの型 (stub/ugoe_wasm.d.ts) を src/wasm/ に置いてから tsc を実行する。
// 生成物があればそちらで検査する (CI は生成してから検査する)。

import { spawnSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync } from "node:fs";
import { createRequire } from "node:module";
import { join } from "node:path";

const pkg = join(import.meta.dirname, "..");
const generated = join(pkg, "src", "wasm", "ugoe_wasm.d.ts");
if (!existsSync(generated)) {
  mkdirSync(join(pkg, "src", "wasm"), { recursive: true });
  copyFileSync(join(pkg, "stub", "ugoe_wasm.d.ts"), generated);
  console.log("note: src/wasm is not generated; checking against stub/ugoe_wasm.d.ts");
}

const tsc = createRequire(import.meta.url).resolve("typescript/bin/tsc");
const r = spawnSync(process.execPath, [tsc, "-p", "tsconfig.json"], { cwd: pkg, stdio: "inherit" });
process.exitCode = r.status ?? 1;
