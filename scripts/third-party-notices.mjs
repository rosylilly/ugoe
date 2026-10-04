#!/usr/bin/env node

// アプリ (エディタ・ビューア・スタジオ) が同梱する第三者のもののライセンスの表記 (apps/*/public/third-party-notices.txt) を
// 生成する。`--check` は生成したものとコミットしたものを突き合わせ、食い違えば失敗する (`just licenses`)。
//
// 表記は次の 2 つから作る (docs/release.md#同梱物のライセンスの表記)。
// - MediaPipe の wasm とモデルの節: packages/face-tracking/third-party-notices.txt (アプリが mediapipe/ に写す表記) の
//   最初の節から終わりまで。
// - JS のライブラリの節: アプリを `vite build` (書き出さない) し、バンドルに入ったモジュールの node_modules の
//   パッケージを集め、それぞれの名前・版・ライセンス・ソースとライセンスの文 (パッケージの LICENSE をそのまま) を並べる。
//   バンドルに入らないもの (svelte のコンパイラの依存など) は載らない。版はロックファイルで決まる node_modules のもの。
//
// 使い方: `node scripts/third-party-notices.mjs` (書き出す)・`node scripts/third-party-notices.mjs --check` (検査)

import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { pathToFileURL } from "node:url";

const root = join(import.meta.dirname, "..");

/** 表記を同梱するアプリ (ディレクトリ・表記の中の名前)。 */
export const APPS = [
  { dir: "apps/editor", name: "ugoe editor" },
  { dir: "apps/viewer", name: "ugoe viewer" },
  { dir: "apps/studio", name: "ugoe studio" },
];

/** 表記の出力先 (アプリのディレクトリから)。 */
export const NOTICES_PATH = "public/third-party-notices.txt";

/** MediaPipe の節の元 (リポジトリのルートから)。 */
const FACE_TRACKING_NOTICES = "packages/face-tracking/third-party-notices.txt";

/** MediaPipe の節が扱うので、JS のライブラリの節に載せないパッケージ。 */
export const COVERED = new Set(["@mediapipe/tasks-vision"]);

/**
 * package.json に license の欄が無いパッケージの、確かめたライセンス
 * (scripts/check-licenses.mjs の NPM_EXCEPTIONS と同じもの。版が変われば向こうの検査が失敗する)。
 */
const LICENSE_OVERRIDES = { "svelte-toolbelt": "MIT" };

/** 区切りの線。 */
const RULE = "=".repeat(78);
const ENTRY_RULE = "-".repeat(78);

/** MIT のライセンスの許諾の文 (ライセンスの文のファイルを持たないパッケージで、著作権の表示に続けて載せる)。 */
const MIT_TEXT = `Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.`;

/**
 * バンドルのモジュールの ID (ファイルのパス) から、それを含む node_modules のパッケージのディレクトリを返す。
 * node_modules の外 (アプリやワークスペースのパッケージ) と仮想のモジュール (`\0` で始まる) は `null`。
 */
export function packageDirOf(id) {
  if (id.startsWith("\0")) return null;
  const path = id.replaceAll("\\", "/").split("?")[0];
  // pnpm の .pnpm/<名前>@<版>/node_modules/<名前> のように node_modules が重なるので、いちばん内側を取る
  const inner = path.lastIndexOf("/node_modules/");
  if (inner < 0) return null;
  const rest = path.slice(inner + "/node_modules/".length).split("/");
  const name = rest[0].startsWith("@") ? `${rest[0]}/${rest[1]}` : rest[0];
  return `${path.slice(0, inner)}/node_modules/${name}`;
}

/** パッケージのディレクトリ (`packageDirOf` の結果) の名前。 */
export function packageName(dir) {
  return dir.slice(dir.lastIndexOf("/node_modules/") + "/node_modules/".length);
}

/** package.json の repository を、ブラウザで開ける URL にする (無ければ npm のページ)。 */
export function sourceUrl(manifest) {
  const repo = manifest.repository;
  let url = typeof repo === "string" ? repo : repo?.url;
  if (!url) return `https://www.npmjs.com/package/${manifest.name}`;
  if (/^[\w.-]+\/[\w.-]+$/.test(url)) url = `github:${url}`;
  url = url
    .replace(/^github:/, "https://github.com/")
    .replace(/^gitlab:/, "https://gitlab.com/")
    .replace(/^git\+/, "")
    .replace(/^git:\/\//, "https://")
    .replace(/^ssh:\/\/git@/, "https://")
    .replace(/^git@([^:]+):/, "https://$1/")
    .replace(/\.git$/, "");
  const dir = typeof repo === "object" && repo.directory ? repo.directory : undefined;
  return dir && url.startsWith("https://github.com/") ? `${url}/tree/HEAD/${dir}` : url;
}

/** package.json の license を SPDX の式として読む (古い `licenses: [{ type }]` の形も受ける)。 */
export function licenseOf(manifest) {
  const { license, licenses } = manifest;
  if (typeof license === "string") return license;
  if (license && typeof license.type === "string") return license.type;
  if (Array.isArray(licenses) && licenses.length > 0) {
    return licenses.map((l) => (typeof l === "string" ? l : l.type)).join(" OR ");
  }
  return LICENSE_OVERRIDES[manifest.name];
}

/**
 * パッケージのライセンスの文のファイルを選ぶ。MIT と別のライセンスから選べる (`MIT OR Apache-2.0`) なら MIT のものを
 * 優先する。無ければ `undefined`。
 */
export function pickLicenseFile(files, license) {
  const candidates = files
    .filter((f) => /^(licen[cs]e|copying)/i.test(f) && !/\.spdx$/i.test(f))
    .sort();
  if (license && /\bOR\b/.test(license) && /\bMIT\b/.test(license)) {
    const mit = candidates.find((f) => /mit/i.test(f));
    if (mit) return mit;
  }
  return candidates[0];
}

/** LICENSE.spdx の PackageCopyrightText (ライセンスの文のファイルを持たない Tauri のプラグインが持つ)。 */
export function spdxCopyright(text) {
  const match = text.match(/^PackageCopyrightText:\s*(.+)$/m);
  return match ? match[1].trim() : undefined;
}

/**
 * node_modules のパッケージのディレクトリから、表記の 1 項目 (名前・版・ライセンス・ソース・ライセンスの文) を作る。
 * ライセンスの文が分からなければ例外。
 */
export function readPackage(dir) {
  const manifest = JSON.parse(readFileSync(join(dir, "package.json"), "utf8"));
  const license = licenseOf(manifest);
  if (!license) throw new Error(`${manifest.name} ${manifest.version}: ライセンスの式がない`);
  const files = readdirSync(dir);
  const file = pickLicenseFile(files, license);
  let text;
  if (file) {
    text = readFileSync(join(dir, file), "utf8");
  } else if (/\bMIT\b/.test(license) && files.includes("LICENSE.spdx")) {
    // ライセンスの文のファイルが無く、SPDX の文書に著作権者だけがある (Tauri のプラグイン)。MIT を選び、その文を付ける
    const copyright = spdxCopyright(readFileSync(join(dir, "LICENSE.spdx"), "utf8"));
    if (copyright) text = `MIT License\n\nCopyright (c) ${copyright}\n\n${MIT_TEXT}\n`;
  }
  if (text === undefined) {
    throw new Error(`${manifest.name} ${manifest.version}: ライセンスの文のファイルがない`);
  }
  return {
    name: manifest.name,
    version: manifest.version,
    license,
    source: sourceUrl(manifest),
    text: text.replaceAll("\r\n", "\n").trimEnd(),
  };
}

/** face-tracking の表記から、最初の節 (下線付きの見出し) から終わりまでを取り出す。 */
export function mediapipeSections(text) {
  const match = text.match(/^[^\n]+\n-{3,}\n/m);
  if (!match) throw new Error(`${FACE_TRACKING_NOTICES}: 節の見出しがない`);
  return text.slice(match.index).trimEnd();
}

/** アプリの表記の全文を組み立てる。`packages` は名前と版で並べておく。 */
export function renderNotices(appName, mediapipe, packages) {
  const title = `Third-party software and models bundled with ${appName}`;
  const lines = [
    title,
    "=".repeat(title.length),
    "",
    `${appName} itself is licensed under GPL-3.0-only (see the LICENSE file of the`,
    "application). This file lists the components that are distributed with it under",
    "other licenses. It is generated from the JavaScript bundle of the application",
    "(scripts/third-party-notices.mjs in the ugoe repository).",
    "",
    "",
    mediapipe,
    "",
    "",
    RULE,
    "",
    "",
    "JavaScript libraries bundled into the application",
    "-------------------------------------------------",
    "",
    "The following libraries are bundled into the JavaScript of the application.",
    "The license text of each library follows its entry. Where a library is offered",
    "under a choice of licenses including the MIT License, it is used under the MIT License.",
  ];
  for (const pkg of packages) {
    lines.push(
      "",
      ENTRY_RULE,
      "",
      `${pkg.name} ${pkg.version}`,
      `License: ${pkg.license}`,
      `Source: ${pkg.source}`,
      "",
      pkg.text,
    );
  }
  return `${lines.join("\n")}\n`;
}

/** アプリを書き出さずにビルドし、バンドルに入った node_modules のパッケージのディレクトリを返す。 */
export async function bundledPackageDirs(appDir) {
  const require = createRequire(join(appDir, "package.json"));
  const { build } = await import(pathToFileURL(require.resolve("vite")).href);
  const result = await build({
    root: appDir,
    configFile: join(appDir, "vite.config.ts"),
    logLevel: "silent",
    build: { write: false, reportCompressedSize: false },
  });
  const outputs = Array.isArray(result) ? result : [result];
  const dirs = new Set();
  for (const output of outputs) {
    for (const item of output.output) {
      if (item.type !== "chunk") continue;
      for (const id of Object.keys(item.modules)) {
        const dir = packageDirOf(id);
        if (dir) dirs.add(dir);
      }
    }
  }
  return dirs;
}

/** アプリの表記を生成する。 */
export async function generate(app) {
  const appDir = join(root, app.dir);
  const mediapipe = mediapipeSections(readFileSync(join(root, FACE_TRACKING_NOTICES), "utf8"));
  const packages = [...(await bundledPackageDirs(appDir))]
    .filter((dir) => !COVERED.has(packageName(dir)))
    .map(readPackage);
  // 同じパッケージの同じ版が 2 か所から入ることがある (pnpm の peer の組み合わせ違い)
  const unique = new Map(packages.map((pkg) => [`${pkg.name}@${pkg.version}`, pkg]));
  const sorted = [...unique.values()].sort(
    (a, b) => a.name.localeCompare(b.name, "en") || a.version.localeCompare(b.version, "en"),
  );
  return renderNotices(app.name, mediapipe, sorted);
}

async function main(argv) {
  const check = argv.includes("--check");
  let failed = false;
  for (const app of APPS) {
    const path = join(root, app.dir, NOTICES_PATH);
    const text = await generate(app);
    const current = existsSync(path) ? readFileSync(path, "utf8") : undefined;
    const rel = `${app.dir}/${NOTICES_PATH}`;
    if (current === text) continue;
    if (check) {
      console.error(
        `error: ${rel} が依存と食い違う (node scripts/third-party-notices.mjs で生成し直してコミットする。docs/release.md#同梱物のライセンスの表記)`,
      );
      failed = true;
    } else {
      mkdirSync(dirname(path), { recursive: true });
      writeFileSync(path, text);
      console.log(`wrote ${rel}`);
    }
  }
  if (check && !failed) console.log(`third-party notices ok (${APPS.length} apps)`);
  process.exitCode = failed ? 1 : 0;
}

if (process.argv[1] !== undefined && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await main(process.argv.slice(2));
}
