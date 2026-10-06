#!/usr/bin/env node
// 3 つのアプリの UI を監査する (`just ui-audit`。tests/ui-audit/README.md)。
//
//   node tests/ui-audit/run.mjs [--only editor,viewer,studio] [--quick] [--no-build] [--out DIR] [--compare DIR] [--strict]
//
// 1. アプリのフロントエンドをビルドする (`just wasm` と `vite build`)。エディタには見本のプロジェクトも作る (`ugoe sample dress-up`)。
// 2. ビルドした画面を小さな静的サーバで配り、ヘッドレスの Chrome (playwright-core) で開く。Tauri の代わりに偽のホストを置く
//    (エディタ: lib/editor-host.mjs、スタジオ: lib/studio-host.mjs。ビューアはブラウザの口のまま)。
// 3. ライト・ダーク × 英語・日本語 × 広い幅・狭い幅、と devicePixelRatio 2・200% の拡大で、場面ごとに撮って検査する
//    (コントラスト・横のはみ出し・24px 未満の押せるもの・名前の無い部品 (axe-core)・reduced motion の下の動き)。
//    広い幅のライトの英語では Tab でフォーカスをたどる。
// 4. <out>/<アプリ>/ に画像 (.png)・アクセシビリティの木 (.aria.yml)・Tab の順 (tab.txt)・報告 (report.md) を、
//    <out>/summary.md と summary.json にまとめを書く。`--compare` は前の summary.json と比べて増減を出す。
//
// `--strict` なら見つけたものがあれば終了コード 1 (既定は、場面が開けなかったときだけ 1)。`just ci` には含めない (重いので)。

import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { chromium } from "playwright-core";
import { APPS } from "./lib/apps.mjs";
import { auditScene, PAGE_CHECKS } from "./lib/audit.mjs";
import { appReport, compare, countsTable, formatCompare, summarize } from "./lib/report.mjs";
import { serve } from "./lib/serve.mjs";
import { formatWalk, tabWalk, walkProblems } from "./lib/walk.mjs";

const root = resolve(import.meta.dirname, "..", "..");
const target = process.env.CARGO_TARGET_DIR ?? join(root, "target");

const args = process.argv.slice(2);
function option(name, fallback) {
  const i = args.indexOf(name);
  return i >= 0 ? args[i + 1] : fallback;
}
const only = option("--only", Object.keys(APPS).join(",")).split(",");
for (const app of only) {
  if (!(app in APPS)) {
    console.error(`unknown app: ${app} (choose from ${Object.keys(APPS).join(", ")})`);
    process.exit(2);
  }
}
const quick = args.includes("--quick");
const build = !args.includes("--no-build");
const strict = args.includes("--strict");
const out = resolve(option("--out", join(target, "ui-audit")));
const compareDir = option("--compare", null);
// 比べる前の結果は、書き換える前に読んでおく (同じ場所に出すときのため)
const before = compareDir
  ? JSON.parse(readFileSync(join(resolve(compareDir), "summary.json"), "utf8"))
  : null;

// --- 道具 ---

function findChrome() {
  if (process.env.CHROME) return process.env.CHROME;
  for (const name of ["google-chrome", "google-chrome-stable", "chromium", "chromium-browser"]) {
    const r = spawnSync("which", [name], { encoding: "utf8" });
    if (r.status === 0) return r.stdout.trim();
  }
  console.error("Chrome is not found (set CHROME to its path)");
  process.exit(2);
}

function run(cmd, cmdArgs, options = {}) {
  console.log(`$ ${cmd} ${cmdArgs.join(" ")}`);
  const r = spawnSync(cmd, cmdArgs, { cwd: root, stdio: "inherit", ...options });
  if (r.status !== 0) {
    console.error(`failed: ${cmd} ${cmdArgs.join(" ")}`);
    process.exit(1);
  }
}

const dist = (app) => join(out, "dist", app);
const sample = {
  project: join(out, "sample", "project", "dress-up.ugoeproj"),
  bundle: join(out, "sample", "bundle", "dress-up.ugoe"),
};

// --- 1. ビルド ---

if (build) {
  for (const app of only) {
    run("just", ["wasm", APPS[app].wasm]);
    run("pnpm", [
      "--filter",
      APPS[app].package,
      "exec",
      "vite",
      "build",
      "--outDir",
      dist(app),
      "--emptyOutDir",
      "--logLevel",
      "warn",
    ]);
  }
  if (only.includes("editor")) {
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
}
for (const app of only) {
  if (!existsSync(join(dist(app), "index.html"))) {
    console.error(`${dist(app)} is missing (run without --no-build)`);
    process.exit(2);
  }
}
if (only.includes("editor") && !existsSync(sample.project)) {
  console.error(`${sample.project} is missing (run without --no-build)`);
  process.exit(2);
}

// --- 2〜3. 撮って検査する ---

/** アプリの場面の組 (名前・テーマ・言語・窓の大きさ・devicePixelRatio・Tab をたどるか)。 */
function variants(app, quickRun) {
  const { wide, narrow } = APPS[app];
  const list = [];
  for (const theme of ["light", "dark"]) {
    for (const lang of ["en", "ja"]) {
      for (const [width, size] of [
        ["wide", wide],
        ["narrow", narrow],
      ]) {
        if (quickRun && !(theme === "light" && lang === "en")) continue;
        list.push({
          name: `${lang}-${theme}-${width}`,
          theme,
          lang,
          width,
          size,
          dpr: 1,
          tab: theme === "light" && lang === "en" && width === "wide",
        });
      }
    }
  }
  if (!quickRun && APPS[app].half) {
    list.push({
      name: "en-light-half",
      theme: "light",
      lang: "en",
      width: "half",
      size: APPS[app].half,
      dpr: 1,
    });
  }
  if (!quickRun && APPS[app].xwide) {
    // 1600px ほどの広い窓 (ビューアの 2 列の右の列・スタジオの中央の列の伸び方を撮る。T0952)
    for (const [theme, lang] of [
      ["light", "en"],
      ["dark", "ja"],
    ]) {
      list.push({
        name: `${lang}-${theme}-xwide`,
        theme,
        lang,
        width: "xwide",
        size: APPS[app].xwide,
        dpr: 1,
      });
    }
  }
  if (!quickRun) {
    list.push({
      name: "ja-light-wide-dpr2",
      theme: "light",
      lang: "ja",
      width: "wide",
      size: wide,
      dpr: 2,
      mainOnly: true,
    });
    // 200% の拡大: CSS の窓の幅が半分になり、1 CSS px を 2 つの画素で描く
    list.push({
      name: "en-light-zoom200",
      theme: "light",
      lang: "en",
      width: "zoom200",
      size: [wide[0] / 2, wide[1] / 2],
      dpr: 2,
    });
  }
  return list;
}

const browser = await chromium.launch({
  executablePath: findChrome(),
  headless: true,
  args: [
    "--use-angle=swiftshader",
    "--enable-unsafe-swiftshader",
    "--force-prefers-reduced-motion",
  ],
});

const summaries = {};
let failedScenes = 0;
const appendix = {};

for (const app of only) {
  const def = APPS[app];
  const dir = join(out, app);
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  const server = await serve(dist(app));
  const scenes = [];
  const tabText = [];
  const unknownCommands = new Set();
  console.log(`\n== ${app} (${server.url})`);

  for (const v of variants(app, quick)) {
    const context = await browser.newContext({
      viewport: { width: v.size[0], height: v.size[1] },
      deviceScaleFactor: v.dpr,
      colorScheme: v.theme,
      locale: v.lang === "en" ? "en-US" : "ja-JP",
      reducedMotion: "reduce",
    });
    const page = await context.newPage();
    const errors = [];
    page.on("pageerror", (e) => errors.push(e.message));
    page.on("console", (m) => {
      if (m.type() === "error" && !m.location().url.endsWith("favicon.ico")) errors.push(m.text());
    });
    await page.addInitScript(PAGE_CHECKS);
    // 前の場面の設定 (言語・テーマ) を残さない
    await page.addInitScript(() => {
      if (sessionStorage.getItem("ui-audit") === null) {
        localStorage.clear();
        sessionStorage.setItem("ui-audit", "1");
      }
    });
    const ctx = {
      url: server.url,
      lang: v.lang,
      osLocale: v.lang === "en" ? "en-US" : "ja-JP",
      home: join(out, "sample", "project"),
      sample,
      async scene(name, { aria = true } = {}) {
        const full = name === "main" ? v.name : `${v.name}-${name}`;
        await page.waitForTimeout(400);
        await page.screenshot({ path: join(dir, `${full}.png`) });
        if (aria)
          writeFileSync(join(dir, `${full}.aria.yml`), await page.locator("body").ariaSnapshot());
        const r = await auditScene(page, { lang: v.lang });
        scenes.push({ name: full, findings: r.findings, stats: r.stats });
        console.log(
          `  ${full}: ${r.findings.length} finding(s), text ${r.stats.textChecked} checked / ${r.stats.textByPixels} by pixels / ${r.stats.textUnmeasured} unmeasured`,
        );
      },
    };
    try {
      await def.setup(page, ctx);
      await def.load(page, ctx);
      await ctx.scene("main");
      if (!v.mainOnly) {
        if (v.tab) {
          const stops = await tabWalk(page, 120);
          tabText.push(`## ${v.name}\n\n${formatWalk(stops)}\n`);
          scenes.push({
            name: `${v.name}-tab`,
            findings: walkProblems(stops).map((p) => ({ check: "focus", ...p })),
          });
          await page.keyboard.press("Escape");
          await page.mouse.click(1, 1);
        }
        // 縦に送る所 (パネル・狭い幅の画面) は下まで送って撮る
        {
          const scrolled = await page.evaluate(() => {
            let did = false;
            for (const el of [document.scrollingElement, ...document.querySelectorAll("*")]) {
              if (!el || el.scrollHeight <= el.clientHeight + 40) continue;
              const o = getComputedStyle(el).overflowY;
              if (el !== document.scrollingElement && o !== "auto" && o !== "scroll") continue;
              el.scrollTop = el.scrollHeight;
              did = true;
            }
            return did;
          });
          if (scrolled) {
            await ctx.scene("bottom", { aria: false });
            await page.evaluate(() => {
              for (const el of [document.scrollingElement, ...document.querySelectorAll("*")])
                if (el) el.scrollTop = 0;
            });
          }
        }
        await def.extras(page, ctx);
      }
    } catch (e) {
      failedScenes++;
      const message = String(e.message ?? e).split("\n")[0];
      console.log(`  ${v.name}: FAILED ${message}`);
      scenes.push({
        name: v.name,
        findings: [{ check: "error", target: "scene", detail: message }],
      });
      await page.screenshot({ path: join(dir, `${v.name}-FAILED.png`) }).catch(() => {});
    }
    if (errors.length) {
      scenes.push({
        name: v.name,
        findings: errors.map((m) => ({ check: "error", target: m.slice(0, 160), detail: "" })),
      });
    }
    for (const c of ctx.editorHost?.unknown ?? []) unknownCommands.add(c);
    await context.close();
  }
  await server.close();

  const summary = summarize(scenes);
  summaries[app] = summary;
  let extra = "";
  if (tabText.length) extra += `## Tab の順\n\n${tabText.join("\n")}\n`;
  if (unknownCommands.size) {
    extra += `## 偽のホストが知らないコマンド\n\nnull を返した (lib/editor-host.mjs に足す): ${[...unknownCommands].sort().join(", ")}\n`;
  }
  appendix[app] = [...unknownCommands];
  writeFileSync(join(dir, "report.md"), appReport(app, summary, extra));
  if (tabText.length) writeFileSync(join(dir, "tab.txt"), tabText.join("\n"));
}
await browser.close();

// --- 4. まとめ ---

const table = countsTable(summaries);
writeFileSync(
  join(out, "summary.md"),
  `# UI の監査のまとめ\n\n数は「別のもの (延べ)」。同じ要素が複数の場面にあれば別のものは 1 つ。詳細は <アプリ>/report.md。\n\n${table}\n`,
);
const json = {
  generated: new Date().toISOString(),
  quick,
  apps: summaries,
  unknownCommands: appendix,
};
writeFileSync(join(out, "summary.json"), `${JSON.stringify(json, null, 2)}\n`);
console.log(`\n${table}\n\nreports: ${out}/<app>/report.md, ${out}/summary.md`);

if (before) {
  console.log(`\ncompared with ${compareDir}:\n${formatCompare(compare(before.apps, summaries))}`);
}

const found = Object.values(summaries).some((s) => s.findings.length > 0);
process.exit(failedScenes > 0 || (strict && found) ? 1 : 0);
