#!/usr/bin/env node
// 部品の見本の画面をヘッドレスの Chrome で撮り、描けたことを検査する (docs/ui.md#そのほか)。
//
//   node packages/ui/scripts/shots.mjs <出力先> [--no-build]
//   (just ui-shots <出力先>)
//
// - 見本の画面を vite で書き出し (gallery/dist)、vite preview で配る。
// - テーマ (light・dark) × 密度 (compact・comfortable) × 言語 (en・ja) の 8 通りと、狭い幅 (360px) を撮る。
// - それぞれで、画面が描き終わったこと・スクリプトのエラーが無いこと・属性が URL のとおりであること・
//   操作できる部品のすべてに読み上げの名前があることを確かめる。PNG は <出力先>/<名前>.png。
// - Chrome は環境変数 CHROME か、PATH の google-chrome / chromium を使う。CI には入れていない (人が Bits UI の
//   更新や見た目の変更のときに実行し、画像を見比べる。ADR 0029 の 5)。

import { execFile, spawnSync } from "node:child_process";
import { mkdirSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";
import { Window } from "happy-dom";
import { build, preview } from "vite";

const args = process.argv.slice(2);
const outArg = args.find((a) => !a.startsWith("--"));
if (!outArg) {
  console.error("usage: shots.mjs <出力先> [--no-build]");
  process.exit(2);
}
const outDir = resolve(outArg);
const configFile = fileURLToPath(new URL("../gallery/vite.config.ts", import.meta.url));

function findChrome() {
  if (process.env.CHROME) return process.env.CHROME;
  for (const name of ["google-chrome", "google-chrome-stable", "chromium", "chromium-browser"]) {
    const r = spawnSync("which", [name], { encoding: "utf8" });
    if (r.status === 0) return r.stdout.trim();
  }
  throw new Error("Chrome が見つからない (環境変数 CHROME で指定する)");
}

const chrome = findChrome();
const chromeArgs = (width, height) => [
  "--headless=new",
  "--disable-gpu",
  "--hide-scrollbars",
  "--no-first-run",
  "--no-default-browser-check",
  "--force-device-scale-factor=1",
  `--window-size=${width},${height}`,
  "--virtual-time-budget=5000",
];

/** 中の文字を名前にできる役割 (ボタン・タブなど)。まとまり (toolbar など) と入力欄はできない。 */
const NAME_FROM_CONTENT = new Set(["button", "tab", "radio", "checkbox", "switch", "option"]);

/** 読み上げの名前 (aria-label・aria-labelledby・<label for>・中の文字の順に見る)。 */
function accessibleName(doc, el) {
  const label = el.getAttribute("aria-label");
  if (label?.trim()) return label.trim();
  const ids = el.getAttribute("aria-labelledby");
  if (ids) {
    const text = ids
      .split(/\s+/)
      .map((id) => doc.getElementById(id)?.textContent?.trim() ?? "")
      .join(" ")
      .trim();
    if (text) return text;
  }
  if (el.id) {
    const forLabel = doc.querySelector(`label[for="${el.id}"]`);
    if (forLabel?.textContent?.trim()) return forLabel.textContent.trim();
  }
  const role = el.getAttribute("role") ?? el.tagName.toLowerCase();
  return NAME_FROM_CONTENT.has(role) ? (el.textContent?.trim() ?? "") : "";
}

const INTERACTIVE =
  'button, input:not([type="hidden"]), select, textarea, [role="slider"], [role="switch"], [role="checkbox"], [role="radio"], [role="tab"], [role="spinbutton"], [role="toolbar"], [role="radiogroup"], [role="tablist"]';

/** 書き出した DOM を検査する。問題の一覧を返す。 */
function inspect(html, expected) {
  const problems = [];
  const window = new Window();
  const doc = new window.DOMParser().parseFromString(html, "text/html");
  const root = doc.documentElement;
  if (root.dataset.galleryError) problems.push(`スクリプトのエラー: ${root.dataset.galleryError}`);
  if (root.dataset.galleryReady !== "true") problems.push("描き終わらなかった");
  if (root.dataset.theme !== expected.theme) problems.push(`data-theme が ${root.dataset.theme}`);
  if (root.dataset.density !== expected.density)
    problems.push(`data-density が ${root.dataset.density}`);
  if (root.lang !== expected.lang) problems.push(`lang が ${root.lang}`);
  const controls = [...doc.querySelectorAll(INTERACTIVE)].filter(
    (el) => !el.closest('[aria-hidden="true"], [hidden]'),
  );
  if (controls.length < 20) problems.push(`部品が少ない (${controls.length})`);
  for (const el of controls) {
    if (!accessibleName(doc, el)) {
      problems.push(`読み上げの名前が無い: ${el.outerHTML.slice(0, 120)}`);
    }
  }
  window.close();
  return problems;
}

// 同じプロセスで vite preview が応えるので、Chrome は非同期で待つ (同期で待つと preview が止まる)
const run = promisify(execFile);

const PNG_SIGNATURE = "89504e470d0a1a0a";

if (!args.includes("--no-build")) {
  await build({ configFile, logLevel: "warn" });
}
const server = await preview({ configFile, preview: { port: 0, strictPort: false } });
const base = server.resolvedUrls?.local[0];
if (!base) throw new Error("vite preview の URL が無い");
mkdirSync(outDir, { recursive: true });

const cases = [];
for (const theme of ["light", "dark"]) {
  for (const density of ["compact", "comfortable"]) {
    for (const lang of ["en", "ja"]) {
      cases.push({ name: `${theme}-${density}-${lang}`, theme, density, lang, width: 1280 });
    }
  }
}
cases.push({
  name: "narrow-light-comfortable-ja",
  theme: "light",
  density: "comfortable",
  lang: "ja",
  width: 360,
});

let failed = 0;
try {
  for (const c of cases) {
    const url = `${base}?theme=${c.theme}&density=${c.density}&lang=${c.lang}`;
    const height = c.width < 600 ? 2400 : 1400;
    const { stdout: dom } = await run(chrome, [...chromeArgs(c.width, height), "--dump-dom", url], {
      encoding: "utf8",
      maxBuffer: 64 * 1024 * 1024,
      timeout: 60_000,
    });
    const problems = inspect(dom, c);
    const png = resolve(outDir, `${c.name}.png`);
    await run(chrome, [...chromeArgs(c.width, height), `--screenshot=${png}`, url], {
      timeout: 60_000,
    });
    const bytes = readFileSync(png);
    if (bytes.subarray(0, 8).toString("hex") !== PNG_SIGNATURE) problems.push("PNG でない");
    if (problems.length > 0) {
      failed++;
      console.error(`NG ${c.name}`);
      for (const p of problems) console.error(`   ${p}`);
    } else {
      console.log(`ok ${png}`);
    }
  }
} finally {
  await new Promise((r) => server.httpServer.close(r));
}
process.exitCode = failed > 0 ? 1 : 0;
