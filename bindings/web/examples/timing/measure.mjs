#!/usr/bin/env node
// 計測のページ (index.html) をヘッドレスの Chrome で開き、結果を表で出す (T0192、docs/testing.md#1-フレームの評価)。
//
//   node bindings/web/examples/timing/measure.mjs [--runs 5] [--backend auto|webgl2|webgpu] [--frames 6000] [--batch 200]
//
// - Chrome は tests/ui-audit と同じもの (CHROME で指定。既定は google-chrome など) を、リポジトリのルートの devDependencies の
//   playwright-core で起動する (WebGL2 は SwiftShader。`update` は CPU だけで動くので、描画の API は時間に効かない)。
// - `--runs` 回ページを開き直し、回ごとの表と、計測ごとの中央値の中央値を出す。測った条件 (load average・Chrome の版・
//   ページの `navigator.userAgent`・描画の API・タイマーの刻み) も出す。

import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { cpus, loadavg } from "node:os";
import { join } from "node:path";
import { parseArgs } from "node:util";
import { chromium } from "playwright-core";
import { serve } from "./serve.mjs";

const { values: args } = parseArgs({
  options: {
    runs: { type: "string", default: "1" },
    backend: { type: "string", default: "auto" },
    frames: { type: "string", default: "6000" },
    batch: { type: "string", default: "200" },
  },
});
const runs = Number(args.runs);
if (!(Number.isInteger(runs) && runs >= 1)) {
  console.error(`--runs must be a positive integer (got ${args.runs})`);
  process.exit(2);
}

const here = import.meta.dirname;
for (const [file, hint] of [
  [
    join(here, "generated.ugoe"),
    "cargo run --release -p ugoe --example puppet_timing -- --write bindings/web/examples/timing/generated.ugoe",
  ],
  [join(here, "..", "..", "dist", "index.js"), "just web-runtime"],
]) {
  if (!existsSync(file)) {
    console.error(`${file} is missing; run \`${hint}\` first (bindings/web/README.md)`);
    process.exit(2);
  }
}

function findChrome() {
  if (process.env.CHROME) return process.env.CHROME;
  for (const name of ["google-chrome", "google-chrome-stable", "chromium", "chromium-browser"]) {
    const r = spawnSync("which", [name], { encoding: "utf8" });
    if (r.status === 0) return r.stdout.trim();
  }
  console.error("Chrome is not found (set CHROME to its path)");
  process.exit(2);
}

const server = await serve(0);
const url = new URL(`http://127.0.0.1:${server.address().port}/`);
url.searchParams.set("backend", args.backend);
url.searchParams.set("frames", args.frames);
url.searchParams.set("batch", args.batch);

const browser = await chromium.launch({
  executablePath: findChrome(),
  headless: true,
  args: ["--use-angle=swiftshader", "--enable-unsafe-swiftshader"],
});

const format = (value) => value.toFixed(1).padStart(9);
const median = (list) => [...list].sort((a, b) => a - b)[Math.floor(list.length / 2)];
const loads = () =>
  loadavg()
    .map((l) => l.toFixed(2))
    .join(" ");

console.log(`chrome:  ${browser.version()}`);
console.log(`cpu:     ${cpus()[0]?.model ?? "?"} (${cpus().length} threads)`);
const all = [];
let failed = false;
try {
  for (let run = 1; run <= runs; run++) {
    const load = loads();
    const page = await browser.newPage();
    page.on("pageerror", (error) => console.error(`page error: ${error}`));
    await page.goto(url.href);
    await page.waitForFunction(() => window.ugoeTiming?.done, null, { timeout: 0 });
    const state = await page.evaluate(() => window.ugoeTiming);
    await page.close();
    if (state.error) {
      console.error(`run ${run} failed: ${state.error}`);
      failed = true;
      break;
    }
    if (run === 1) {
      for (const [key, value] of Object.entries(state.environment)) {
        console.log(`${`${key}:`.padEnd(8)} ${value}`);
      }
    }
    console.log();
    console.log(`run ${run}/${runs} (load average before: ${load}). per frame (us): best / median`);
    for (const r of state.results) {
      console.log(
        `  ${r.model.padEnd(12)} ${String(r.vertices).padStart(5)} vertices  ${r.measurement.padEnd(24)} ${format(r.best)} / ${format(r.median)}`,
      );
    }
    all.push(state.results);
  }
} finally {
  await browser.close();
  server.close();
}
if (failed) process.exit(1);

if (runs > 1) {
  console.log();
  console.log(`median of ${runs} runs (us): best / median`);
  for (const [i, r] of all[0].entries()) {
    const best = median(all.map((results) => results[i].best));
    const med = median(all.map((results) => results[i].median));
    console.log(
      `  ${r.model.padEnd(12)} ${r.measurement.padEnd(24)} ${format(best)} / ${format(med)}`,
    );
  }
}
console.log();
console.log(`load average after: ${loads()}`);
