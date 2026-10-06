#!/usr/bin/env node
// PuppetSurface (src/surface.ts) と Player を本物の wasm とヘッドレスの Chrome で確かめる (`just player-browser`、T0936・T0939)。
//
// 1. ugoe-wasm をビルドして JS バインディングを <out>/wasm に生成する (`node scripts/wasm.mjs build`。`--no-build` で省く)。
// 2. @ugoe/player (src/index.ts。PuppetSurface と Player) を Vite と Svelte のプラグインで <out>/bundle/player.js にまとめる。
// 3. 小さなサーバでページ (index.html・scenarios.js)・player.js・wasm・見本のモデルを配り、ヘッドレスの Chrome
//    (playwright-core。WebGL2 は SwiftShader) で開いて、scenarios.js のシナリオを順に流す。
//
//   node packages/player/browser-test/run.mjs [--no-build] [--only NAME,...] [--out DIR] [--max-webgl-contexts N]
//
// `--max-webgl-contexts` は Chrome の WebGL のコンテキストの数の上限 (既定は 256。Chrome の既定の 16 で流すと、捨てた
// canvas のコンテキストが上限に数えられて、今のコンテキストを失うことがある。T0475 の再現に使った)。
//
// 要るもの: Google Chrome か Chromium (環境変数 CHROME で場所を指定できる)、`just wasm` に要るもの (wasm-bindgen-cli)。
// 置き場所を packages/player にした理由 (ライセンス) は README.md。

import { spawnSync } from "node:child_process";
import { createReadStream, mkdirSync, statSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { extname, join, resolve } from "node:path";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { chromium } from "playwright-core";
import { build as viteBuild } from "vite";

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
const maxWebglContexts = Number(option("--max-webgl-contexts", "256"));
/** 全体の上限 (ミリ秒)。SwiftShader の WebPuppet の作成は 1 回で数秒かかることがある。 */
const TIMEOUT_MS = 300_000;
/** ページの大きさ (CSS ピクセル)。devicePixelRatio を変えるときも同じ大きさで上書きする。 */
const VIEWPORT = { width: 800, height: 600 };

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

// --- 2. @ugoe/player → player.js ---

// Player (`.svelte.ts` の runes) と依存 (@ugoe/i18n・@ugoe/face-tracking・svelte) を 1 つの ES モジュールにまとめる (T0939)。
// wasm はまとめず、ページが /wasm/ から読んで渡す (アプリと同じく、生成物の置き場所はホストが決める)。
await viteBuild({
  configFile: false,
  root: here,
  logLevel: "warn",
  plugins: [svelte()],
  define: { "process.env.NODE_ENV": JSON.stringify("production") },
  build: {
    lib: {
      entry: join(here, "..", "src", "index.ts"),
      formats: ["es"],
      fileName: () => "player.js",
    },
    outDir: join(out, "bundle"),
    emptyOutDir: true,
    minify: false,
    reportCompressedSize: false,
  },
});

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
  if (path === "/player.js") return join(out, "bundle", "player.js");
  const sample = path.match(/^\/samples\/((?:dress-up\/)?[\w-]+\.ugoe(?:mod)?)$/);
  if (sample) return join(root, "assets", "samples", sample[1]);
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
  // 作り直しを繰り返すシナリオは、捨てた canvas のコンテキストが GC されるまで数に入るので、上限 (16) に当たって
  // 今のコンテキストを失わないよう、上限を上げる (T0938)。
  // GPU プロセスを何度も落とすシナリオ (T0944) のため、落ちた回数の上限 (超えると Chrome は GPU を諦め、ソフトウェアの
  // 代わりも無ければブラウザごと終わる) と、落ちたときにページの WebGL を止める仕組み (`getContext` が `null` を返す) を外す。
  args: [
    "--use-angle=swiftshader",
    "--enable-unsafe-swiftshader",
    `--max-active-webgl-contexts=${maxWebglContexts}`,
    "--disable-gpu-process-crash-limit",
    "--disable-domain-blocking-for-3d-apis",
  ],
});
let failed = 0;
/** ページのコンソール (wgpu の警告が多いので、画面には出さずに <out>/page.log に書く)。 */
const pageLog = [];
try {
  const page = await browser.newPage({ viewport: VIEWPORT, deviceScaleFactor: 1 });
  page.on("console", (m) => pageLog.push(`[${m.type()}] ${m.text()}`));
  page.on("pageerror", (e) => pageLog.push(`[pageerror] ${e.message}`));
  // devicePixelRatio だけを途中で変える (倍率の違う画面へウィンドウを移したのと同じ。ページの大きさは変えない。T0939)。
  // Playwright はページの倍率を作るときにしか決められないので、CDP で直接上書きする。
  // ヘッドレスの Chrome は倍率の上書きだけではメディアクエリを評価し直さず (`matches` は変わるが `change` が届かない)、
  // 次に評価し直すまで知らせが遅れる。そこで、エミュレートするメディアの型を "" と "screen" (どちらも画面) で
  // 交互に切り替えて評価し直させる (ページから見える値は倍率のほかに変わらない)。
  const cdp = await page.context().newCDPSession(page);
  let mediaToggle = false;
  await page.exposeFunction("setDevicePixelRatio", async (ratio) => {
    await cdp.send("Emulation.setDeviceMetricsOverride", {
      ...VIEWPORT,
      deviceScaleFactor: ratio,
      mobile: false,
    });
    mediaToggle = !mediaToggle;
    await cdp.send("Emulation.setEmulatedMedia", { media: mediaToggle ? "screen" : "" });
  });
  // GPU プロセスを落とす (CDP の `Browser.crashGpuProcess`。T0944)。`delay` (ミリ秒) を渡すと、待たずに戻って、その後で
  // Node の側から落とす (ページの JS が同期的に動いている間 (wasm の作成の途中など) にも落とせる)。
  const browserCdp = await browser.newBrowserCDPSession();
  await page.exposeFunction("crashGpuProcess", async (delay) => {
    if (delay === undefined || delay === null) {
      await browserCdp.send("Browser.crashGpuProcess");
      return;
    }
    setTimeout(() => {
      browserCdp.send("Browser.crashGpuProcess").catch(() => {});
    }, delay);
  });
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
