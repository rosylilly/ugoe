#!/usr/bin/env node
// 実機で手で確かめるための一式を、WSL2 から Windows のフォルダーにまとめる (`just check-pack [出力先]`。T0007)。
// 手順は docs/manual-check.md#用意する。
//
//   node scripts/check-pack.mjs [出力先 (WSL のパス)] [--force]
//
// - 出力先を省くと、Windows の利用者のフォルダー (%USERPROFILE%) の下の `ugoe-check`。
// - `installers/`: `just app::build PLATFORM=windows` が target/dist/<アプリ>/windows/ に作ったインストーラ。
// - `samples/`: 見本のモデル (assets/samples の .ugoe と .ugoemod)。
// - `fixtures/`: エディタの取り込みの確認に使う PSD (assets/fixtures/character.psd・layers.psd)。
// - `ugoe-sample-profile.json`: スタジオの見本のプロファイル (apps/studio/scripts/sample-profile.mjs と同じ中身)。
//   モデルを Windows のパス (`C:\Users\...\ugoe-check\samples\avatar.ugoe`) で指す。
// - `manual-check.md`: 確認表の写し (結果を書き込んで返してもらう)。
// 前の一式があっても、プロファイルと確認表の写しは `--force` を付けないと上書きしない (書き込んだ結果を消さないため)。

import { execFileSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { join, win32 } from "node:path";
import { pathToFileURL } from "node:url";
import { sampleProfile } from "../apps/studio/scripts/sample-profile.mjs";

const root = join(import.meta.dirname, "..");

/** まとめるインストーラのアプリ (target/dist/<アプリ>/windows/)。 */
export const APPS = ["editor", "viewer", "studio"];

/**
 * WSL のパスを Windows のパスにする (`/mnt/c/Users/a` → `C:\Users\a`)。`/mnt/<ドライブ>/` の下でなければ null
 * (Linux の側のファイルは `\\wsl.localhost\...` で読めるが遅く、WSL を止めると読めないので使わない)。
 */
export function windowsPath(path) {
  const m = path.match(/^\/mnt\/([a-zA-Z])(\/.*)?$/);
  if (!m) return null;
  const rest = (m[2] ?? "").replace(/\/+$/, "").replaceAll("/", "\\");
  return `${m[1].toUpperCase()}:${rest || "\\"}`;
}

/** 見本のプロファイルの中のファイルのパスを、`samples` (Windows のパス) の下に置き換える。 */
export function windowsSampleProfile(samples, lang = "ja") {
  const profile = sampleProfile(samples, lang);
  // sampleProfile は node:path の join (Linux では `/`) でつなぐので、Windows の区切りにそろえる
  const fix = (p) => win32.normalize(p);
  for (const scene of profile.scenes) {
    scene.model = fix(scene.model);
    scene.modules = scene.modules.map(fix);
  }
  for (const hotkey of profile.hotkeys) {
    if (hotkey.action.kind === "dressUp" && hotkey.action.module) {
      hotkey.action.module = fix(hotkey.action.module);
    }
  }
  return profile;
}

/** Windows の利用者のフォルダー (WSL のパス)。分からなければ null。 */
function windowsHome() {
  try {
    const home = execFileSync("cmd.exe", ["/c", "echo %USERPROFILE%"], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
      cwd: "/mnt/c",
    }).trim();
    return execFileSync("wslpath", ["-u", home], { encoding: "utf8" }).trim();
  } catch {
    return null;
  }
}

/** `dir` の下の、`exts` のどれかで終わるファイル (相対のパス)。 */
function filesUnder(dir, exts) {
  const out = [];
  for (const name of readdirSync(dir)) {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) {
      out.push(...filesUnder(path, exts).map((f) => join(name, f)));
    } else if (exts.some((e) => name.endsWith(e))) {
      out.push(name);
    }
  }
  return out;
}

function copy(from, to) {
  mkdirSync(join(to, ".."), { recursive: true });
  copyFileSync(from, to);
}

/** 書き込んだ結果を消さないよう、`--force` が無ければ既存のファイルを残す。 */
function writeUnlessExists(path, write, force) {
  if (existsSync(path) && !force) {
    console.log(`kept ${path} (exists; add --force to overwrite)`);
    return;
  }
  write();
  console.log(`wrote ${path}`);
}

function main(argv) {
  const force = argv.includes("--force");
  const args = argv.filter((a) => !a.startsWith("--"));
  let dest = args[0];
  if (dest === undefined) {
    const home = windowsHome();
    if (home === null) {
      console.error(
        "could not find the Windows user folder; pass the destination (e.g. /mnt/c/Users/<name>/ugoe-check)",
      );
      process.exit(2);
    }
    dest = join(home, "ugoe-check");
  }
  const winDest = windowsPath(dest);
  if (winDest === null) {
    console.error(
      `${dest} is not under /mnt/<drive>/; pick a folder that Windows can open directly (e.g. /mnt/c/Users/<name>/ugoe-check)`,
    );
    process.exit(2);
  }
  mkdirSync(dest, { recursive: true });

  const installers = [];
  for (const app of APPS) {
    const dir = join(root, "target", "dist", app, "windows");
    if (!existsSync(dir)) continue;
    for (const name of readdirSync(dir).filter((n) => n.endsWith(".exe"))) {
      copy(join(dir, name), join(dest, "installers", name));
      installers.push(name);
    }
  }
  if (installers.length === 0) {
    console.warn(
      "no Windows installers in target/dist/*/windows/ (run `just app::build PLATFORM=windows` first)",
    );
  }

  const samplesDir = join(root, "assets", "samples");
  for (const file of filesUnder(samplesDir, [".ugoe", ".ugoemod"])) {
    copy(join(samplesDir, file), join(dest, "samples", file));
  }
  for (const name of ["character.psd", "layers.psd"]) {
    copy(join(root, "assets", "fixtures", name), join(dest, "fixtures", name));
  }

  const profilePath = join(dest, "ugoe-sample-profile.json");
  const profile = windowsSampleProfile(win32.join(winDest, "samples"));
  writeUnlessExists(
    profilePath,
    () => writeFileSync(profilePath, `${JSON.stringify(profile, null, 2)}\n`),
    force,
  );
  const checkPath = join(dest, "manual-check.md");
  writeUnlessExists(
    checkPath,
    () => copyFileSync(join(root, "docs", "manual-check.md"), checkPath),
    force,
  );

  console.log(`\npacked into ${dest} (${winDest})`);
  for (const name of installers) console.log(`  installers\\${name}`);
  console.log(`  samples\\, fixtures\\, ugoe-sample-profile.json, manual-check.md`);
  console.log("\nnext: docs/manual-check.md (the first round on Windows)");
}

if (process.argv[1] !== undefined && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main(process.argv.slice(2));
}
