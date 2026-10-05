#!/usr/bin/env node
// PuppetSurface (src/surface.ts) を本物の wasm とヘッドレスの Chrome で確かめる (`just player-browser`、T0936)。
//
// 1. ugoe-wasm をビルドして JS バインディングを <out>/wasm に生成する (`node scripts/wasm.mjs build`。`--no-build` で省く)。
// 2. src/surface.ts を TypeScript の `transpileModule` で <out>/surface.js にする (surface.ts は何も import しない)。
// 3. 小さなサーバでページ (index.html・scenarios.js)・surface.js・wasm・見本のモデルを配り、ヘッドレスの Chrome
//    (playwright-core。WebGL2 は SwiftShader) で開いて、scenarios.js のシナリオを順に流す。
//
//   node packages/player/browser-test/run.mjs [--no-build] [--only NAME,...] [--out DIR]
//
// 要るもの: Google Chrome か Chromium (環境変数 CHROME で場所を指定できる)、`just wasm` に要るもの (wasm-bindgen-cli)。
// 置き場所を packages/player にした理由 (ライセンス) は README.md。

import { spawnSync } from "node:child_process";
import { createReadStream, mkdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { extname, join, resolve } from "node:path";
import { chromium } from "playwright-core";
import ts from "typescript";

const here = import.meta.dirname;
const root = join(here, "..", "..", "..");
const args = process.argv.slice(2);

function option(name, fallback) {
  const i = args.indexOf(name);
  return i >= 0 && i + 1 < args.length ? args[i + 1] : fallback;
}

const out = resolve(option("--out", join(root, "target", "player-browser")));
const only = option("--only", null)?.split(",") ?? null;
const build = !args.includes("--no-build");
/** 全体の上限 (ミリ秒)。SwiftShader の WebPuppet の作成は 1 回で数秒かかることがある。 */
const TIMEOUT_MS = 300_000;

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

// --- 1. wasm ---

mkdirSync(out, { recursive: true });
if (build) {
  const r = spawnSync("node", [join(root, "scripts", "wasm.mjs"), "build", join(out, "wasm")], {
    cwd: root,
    stdio: "inherit",
  });
  if (r.status !== 0) {
    console.error("failed to build ugoe-wasm");
    process.exit(1);
  }
}

// --- 2. surface.ts → surface.js ---

const source = readFileSync(join(here, "..", "src", "surface.ts"), "utf8");
const { outputText } = ts.transpileModule(source, {
  compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext },
  fileName: "surface.ts",
});
writeFileSync(join(out, "surface.js"), outputText);

// --- 3. 配って開く ---

const TYPES = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".wasm": "application/wasm",
};
/** URL のパス → ファイル。ここに無いものは 404。 */
function fileFor(path) {
  if (path === "/" || path === "/index.html") return join(here, "index.html");
  if (path === "/scenarios.js") return join(here, "scenarios.js");
  if (path === "/surface.js") return join(out, "surface.js");
  if (path === "/samples/avatar.ugoe") return join(root, "assets", "samples", "avatar.ugoe");
  const m = path.match(/^\/wasm\/([\w.-]+)$/);
  return m ? join(out, "wasm", m[1]) : null;
}
const server = createServer((req, res) => {
  const file = fileFor(new URL(req.url ?? "/", "http://localhost").pathname);
  try {
    if (file === null) throw new Error("not found");
    const size = statSync(file).size;
    res.writeHead(200, {
      "Content-Type": TYPES[extname(file)] ?? "application/octet-stream",
      "Content-Length": size,
    });
    createReadStream(file).pipe(res);
  } catch {
    res.writeHead(404).end();
  }
});
await new Promise((done) => server.listen(0, "127.0.0.1", done));
const { port } = server.address();

const browser = await chromium.launch({
  executablePath: findChrome(),
  headless: true,
  // GPU の無い環境 (CI・WSL) でも同じように動くよう、WebGL2 はソフトウェアの実装 (SwiftShader) で描く。
  // WebGPU は有効にしないので、PuppetSurface の "auto" は WebGL2 で作る。
  args: ["--use-angle=swiftshader", "--enable-unsafe-swiftshader"],
});
let failed = 0;
/** ページのコンソール (wgpu の警告が多いので、画面には出さずに <out>/page.log に書く)。 */
const pageLog = [];
try {
  const page = await browser.newPage();
  page.on("console", (m) => pageLog.push(`[${m.type()}] ${m.text()}`));
  page.on("pageerror", (e) => pageLog.push(`[pageerror] ${e.message}`));
  await page.goto(`http://127.0.0.1:${port}/`);
  await page.waitForFunction(() => typeof window.runScenarios === "function");
  const results = await Promise.race([
    page.evaluate((names) => window.runScenarios(names), only),
    new Promise((_, reject) =>
      setTimeout(() => reject(new Error(`timed out after ${TIMEOUT_MS / 1000}s`)), TIMEOUT_MS),
    ),
  ]);
  for (const r of results) {
    console.log(`${r.ok ? "ok  " : "FAIL"} ${r.name}${r.error ? `: ${r.error}` : ""}`);
    if (!r.ok) failed++;
  }
  if (results.length === 0) {
    console.error("no scenario matched --only");
    failed++;
  }
  console.log(`passed ${results.length - failed} of ${results.length} scenarios`);
} catch (e) {
  console.error(e instanceof Error ? e.message : e);
  failed++;
} finally {
  await browser.close();
  server.close();
  writeFileSync(join(out, "page.log"), `${pageLog.join("\n")}\n`);
}
if (failed > 0) console.error(`see ${join(out, "page.log")} for the console of the page`);
process.exit(failed === 0 ? 0 : 1);
