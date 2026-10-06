#!/usr/bin/env node
// エディタの画面を本物の wasm とヘッドレスの Chrome で動かし、GPU の喪失のまわりを確かめる (`just editor-browser`、T1656)。
//
// 1. エディタのフロントエンドをビルドする (`just wasm` と `vite build`。出力は <out>/dist)。見本のプロジェクトと同じモデルの
//    `.ugoe` と `.ugoemod` を `ugoe sample dress-up` で作る (<out>/sample)。`--no-build` で省く。
// 2. dist を小さな静的サーバで配り、ヘッドレスの Chrome (playwright-core。WebGL2 は SwiftShader) で開く。Tauri の代わりに
//    `just ui-audit` の偽のホスト (tests/ui-audit/lib/editor-host.mjs) を置く (見本のプロジェクトを開き、`.ugoemod` を読める)。
// 3. scenarios.mjs のシナリオを、1 つずつ新しいページで流す。
//
//   node apps/editor/browser-test/run.mjs [--no-build] [--only NAME,...] [--out DIR]
//
// 要るもの: Google Chrome か Chromium (環境変数 CHROME で場所を指定できる)、`just wasm` に要るもの (wasm-bindgen-cli)、
// cargo (見本を作る)。詳しくは README.md。

import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { chromium } from "playwright-core";
import {
  createEditorHost,
  editorPageInit,
  ipcBinding,
} from "../../../tests/ui-audit/lib/editor-host.mjs";
import { serve } from "../../../tests/ui-audit/lib/serve.mjs";
import { PAGE_INIT, SCENARIOS } from "./scenarios.mjs";

const here = import.meta.dirname;
const root = join(here, "..", "..", "..");
const args = process.argv.slice(2);

function option(name, fallback) {
  const i = args.indexOf(name);
  return i >= 0 && i + 1 < args.length ? args[i + 1] : fallback;
}

const out = resolve(option("--out", join(root, "target", "editor-browser")));
const only = option("--only", null)?.split(",") ?? null;
const build = !args.includes("--no-build");
/** 1 つのシナリオの上限 (ミリ秒)。SwiftShader の WebPuppet の作成は 1 回で数秒かかることがある。 */
const SCENARIO_TIMEOUT_MS = 180_000;
const VIEWPORT = { width: 1440, height: 900 };

for (const name of only ?? []) {
  if (!(name in SCENARIOS)) {
    console.error(`unknown scenario: ${name} (choose from ${Object.keys(SCENARIOS).join(", ")})`);
    process.exit(2);
  }
}

function findChrome() {
  if (process.env.CHROME) return process.env.CHROME;
  for (const name of ["google-chrome", "google-chrome-stable", "chromium", "chromium-browser"]) {
    const r = spawnSync("which", [name], { encoding: "utf8" });
    if (r.status === 0) return r.stdout.trim();
  }
  console.error(
    "Chrome is not found: install Google Chrome or Chromium, or set CHROME to its path",
  );
  process.exit(2);
}

function run(cmd, cmdArgs) {
  console.log(`$ ${cmd} ${cmdArgs.join(" ")}`);
  const r = spawnSync(cmd, cmdArgs, { cwd: root, stdio: "inherit" });
  if (r.status !== 0) {
    console.error(`failed: ${cmd} ${cmdArgs.join(" ")}`);
    process.exit(1);
  }
}

const dist = join(out, "dist");
const sample = {
  project: join(out, "sample", "project", "dress-up.ugoeproj"),
  bundle: join(out, "sample", "bundle", "dress-up.ugoe"),
};

// --- 1. ビルド ---

if (build) {
  run("just", ["wasm"]);
  run("pnpm", [
    "--filter",
    "@ugoe/editor",
    "exec",
    "vite",
    "build",
    "--outDir",
    dist,
    "--emptyOutDir",
    "--logLevel",
    "warn",
  ]);
  rmSync(join(out, "sample"), { recursive: true, force: true });
  run("just", [
    "cli",
    "sample",
    "dress-up",
    "-o",
    join(out, "sample", "bundle"),
    "--project",
    join(out, "sample", "project"),
  ]);
}
if (!existsSync(join(dist, "index.html")) || !existsSync(sample.project)) {
  console.error(`${dist} or ${sample.project} is missing (run without --no-build)`);
  process.exit(2);
}
// wasm-bindgen のグルー (Vite が import.meta.glob の生成物を別のチャンクにする)。シナリオが同じ URL を読んで、エディタと
// 同じ WebPuppet のクラスを包む。
const wasmChunk = readdirSync(join(dist, "assets")).find(
  (n) => n.startsWith("ugoe_wasm") && n.endsWith(".js"),
);
if (wasmChunk === undefined) {
  console.error(`${dist}/assets has no ugoe_wasm chunk (is ugoe-wasm built?)`);
  process.exit(2);
}

// --- 2〜3. 開いてシナリオを流す ---

const server = await serve(dist);
const browser = await chromium.launch({
  executablePath: findChrome(),
  headless: true,
  // packages/player/browser-test/run.mjs と同じく、GPU の無い環境 (CI・WSL) でも WebGL2 を SwiftShader で描く。
  // WebGPU は有効にしないので、エディタのビューポートは WebGL2 で作る。
  args: ["--use-angle=swiftshader", "--enable-unsafe-swiftshader"],
});
mkdirSync(out, { recursive: true });
/** ページのコンソール (wgpu の警告が多いので、画面には出さずに <out>/page.log に書く)。 */
const pageLog = [];
let failed = 0;
let ran = 0;
try {
  for (const [name, scenario] of Object.entries(SCENARIOS)) {
    if (only !== null && !only.includes(name)) continue;
    ran++;
    pageLog.push(`--- ${name}`);
    const context = await browser.newContext({
      viewport: VIEWPORT,
      deviceScaleFactor: 1,
      locale: "en-US",
      reducedMotion: "reduce",
    });
    const page = await context.newPage();
    page.on("console", (m) => pageLog.push(`[${m.type()}] ${m.text()}`));
    page.on("pageerror", (e) => pageLog.push(`[pageerror] ${e.message}`));
    const host = createEditorHost(sample);
    await page.exposeFunction("__ugoeAuditIpc", ipcBinding(host));
    await page.addInitScript(editorPageInit, { osLocale: "en-US", home: join(out, "sample") });
    await page.addInitScript(PAGE_INIT);
    const ctx = {
      url: server.url,
      wasm: `${server.url}/assets/${wasmChunk}`,
      sample,
      modules: join(out, "sample", "bundle"),
    };
    let timer;
    try {
      await Promise.race([
        scenario(page, ctx),
        new Promise((_, reject) => {
          timer = setTimeout(
            () => reject(new Error(`timed out after ${SCENARIO_TIMEOUT_MS / 1000}s`)),
            SCENARIO_TIMEOUT_MS,
          );
        }),
      ]);
      console.log(`ok   ${name}`);
    } catch (e) {
      failed++;
      const message = e instanceof Error ? e.message : String(e);
      console.log(`FAIL ${name}: ${message.split("\n")[0]}`);
      pageLog.push(`[FAIL] ${message}`);
      await page.screenshot({ path: join(out, `${name}-FAILED.png`) }).catch(() => {});
    } finally {
      clearTimeout(timer);
      await context.close();
    }
  }
  if (ran === 0) {
    console.error("no scenario matched --only");
    failed++;
  }
  console.log(`passed ${ran - failed} of ${ran} scenarios`);
} finally {
  await browser.close();
  await server.close();
  writeFileSync(join(out, "page.log"), `${pageLog.join("\n")}\n`);
}
if (failed > 0) console.error(`see ${join(out, "page.log")} for the console of the page`);
process.exit(failed === 0 ? 0 : 1);
