#!/usr/bin/env node
// 計測のページ (index.html) をヘッドレスの Chrome で開き、結果を表で出す (T0192、docs/testing.md#1-フレームの評価)。
//
//   node bindings/web/examples/timing/measure.mjs [--runs 5] [--backend auto|webgl2|webgpu] [--frames 6000] [--batch 200]
//                                                 [--max-us 300]
//
// - Chrome は tests/ui-audit と同じもの (CHROME で指定。既定は google-chrome など) を、リポジトリのルートの devDependencies の
//   playwright-core で起動する (WebGL2 は SwiftShader。`update` は CPU だけで動くので、描画の API は時間に効かない)。
// - `--runs` 回ページを開き直し、回ごとの表と、計測ごとの中央値の中央値を出す。測った条件 (load average・Chrome の版・
//   ページの `navigator.userAgent`・描画の API・タイマーの刻み) も出す。
// - `--max-us` を付けると、`update` と `follow` の中央値 (`--runs` 回の中央値) を上限 (µs) と比べ、超えたものがあればページを
//   開き直して 2 回まで測り直し (check.mjs の `RETRIES`)、それでも超えたものを `OVER` と示して終了コード 1 で終わる
//   (T0194。`just perf` と週に 1 度の CI の perf.yml が流す。docs/testing.md#遅くなったら気づく検査)。
// - 生成物 (generated.ugoe・bindings/web/dist) か Chrome が無ければ、作り方・指定の仕方を示して終了コード 2 で終わる。

import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { cpus, loadavg } from "node:os";
import { join } from "node:path";
import { parseArgs } from "node:util";
import { chromium } from "playwright-core";
import { checkLimit, summarize } from "./check.mjs";
import { serve } from "./serve.mjs";

const { values: args } = parseArgs({
  options: {
    runs: { type: "string", default: "1" },
    backend: { type: "string", default: "auto" },
    frames: { type: "string", default: "6000" },
    batch: { type: "string", default: "200" },
    "max-us": { type: "string" },
  },
});
const runs = Number(args.runs);
if (!(Number.isInteger(runs) && runs >= 1)) {
  console.error(`--runs must be a positive integer (got ${args.runs})`);
  process.exit(2);
}
const maxUs = args["max-us"] === undefined ? null : Number(args["max-us"]);
if (maxUs !== null && !(maxUs > 0 && Number.isFinite(maxUs))) {
  console.error(`--max-us must be a positive number (got ${args["max-us"]})`);
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
  if (process.env.CHROME) {
    if (existsSync(process.env.CHROME)) return process.env.CHROME;
    console.error(`Chrome is not found at CHROME=${process.env.CHROME}`);
    process.exit(2);
  }
  for (const name of ["google-chrome", "google-chrome-stable", "chromium", "chromium-browser"]) {
    const r = spawnSync("which", [name], { encoding: "utf8" });
    if (r.status === 0) return r.stdout.trim();
  }
  console.error(
    "Chrome is not found: install Google Chrome or Chromium, or set CHROME to its path",
  );
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
const loads = () =>
  loadavg()
    .map((l) => l.toFixed(2))
    .join(" ");

/** ページを 1 回開いて測り、表を出して結果を返す。失敗したら `null`。 */
async function measure(label, first) {
  const load = loads();
  const page = await browser.newPage();
  page.on("pageerror", (error) => console.error(`page error: ${error}`));
  await page.goto(url.href);
  await page.waitForFunction(() => window.ugoeTiming?.done, null, { timeout: 0 });
  const state = await page.evaluate(() => window.ugoeTiming);
  await page.close();
  if (state.error) {
    console.error(`${label} failed: ${state.error}`);
    return null;
  }
  if (first) {
    for (const [key, value] of Object.entries(state.environment)) {
      console.log(`${`${key}:`.padEnd(8)} ${value}`);
    }
  }
  console.log();
  console.log(`${label} (load average before: ${load}). per frame (us): best / median`);
  for (const r of state.results) {
    console.log(
      `  ${r.model.padEnd(12)} ${String(r.vertices).padStart(5)} vertices  ${r.measurement.padEnd(24)} ${format(r.best)} / ${format(r.median)}`,
    );
  }
  return state.results;
}

console.log(`chrome:  ${browser.version()}`);
console.log(`cpu:     ${cpus()[0]?.model ?? "?"} (${cpus().length} threads)`);
const all = [];
let failed = false;
let over = 0;
try {
  for (let run = 1; run <= runs; run++) {
    const results = await measure(`run ${run}/${runs}`, run === 1);
    if (!results) {
      failed = true;
      break;
    }
    all.push(results);
  }
  if (!failed) {
    const summary = summarize(all);
    if (runs > 1) {
      console.log();
      console.log(`median of ${runs} runs (us): best / median`);
      for (const r of summary) {
        console.log(
          `  ${r.model.padEnd(12)} ${r.measurement.padEnd(24)} ${format(r.best)} / ${format(r.median)}`,
        );
      }
    }
    if (maxUs !== null) {
      let retry = 0;
      const rows = await checkLimit(summary, maxUs, async () => {
        retry++;
        // 測り直しのページが失敗したら、前の値のまま判定する (`checkLimit` は見つからない行を変えない)
        return (await measure(`over the limit; measuring again (${retry})`, false)) ?? [];
      });
      console.log();
      console.log("check (us): median / limit");
      for (const r of rows) {
        console.log(
          `  ${r.model.padEnd(12)} ${r.measurement.padEnd(24)} ${format(r.median)} / ${format(r.limit)}  ${r.ok ? "ok" : "OVER"}`,
        );
      }
      over = rows.filter((r) => !r.ok).length;
    }
  }
} finally {
  await browser.close();
  server.close();
}
if (failed) process.exit(1);

console.log();
console.log(`load average after: ${loads()}`);
if (over > 0) {
  console.error(`${over} measurement(s) over the limit (docs/testing.md)`);
  process.exit(1);
}
