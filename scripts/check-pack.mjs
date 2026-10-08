#!/usr/bin/env node
// 実機で手で確かめるための一式を、WSL2 から Windows のフォルダーにまとめる (`just check-pack [出力先]`。T0007)。
// 手順は docs/manual-check.md#用意する。
//
//   node scripts/check-pack.mjs [出力先 (WSL のパス)] [--force]
//
// - 出力先を省くと、Windows の利用者のフォルダー (%USERPROFILE%) の下の `ugoe-check`。
// - `installers/`: `just app::build PLATFORM=windows` が target/dist/<アプリ>/windows/ に作ったインストーラ。
// - `samples/`: 見本のモデル (assets/samples の .ugoe と .ugoemod) と、スタジオの見本のプロファイル
//   (`ugoe-sample-profile.json`・`ugoe-sample-profile.en.json`)。プロファイルはモデルを相対パスで持つ (T0865) ので、そのまま写す。
//   `ugoe-check` のフォルダーを動かしても開ける。
// - `fixtures/`: エディタの取り込みの確認に使う PSD (assets/fixtures/character.psd・layers.psd)。
// - `manual-check.md`: 確認表の写し (結果を書き込んで返してもらう)。
// 前の一式があっても、確認表の写しは `--force` を付けないと上書きしない (書き込んだ結果を消さないため)。見本のプロファイルは
// スタジオが書き換えない (見本の印。T0866) ので、モデルと同じくいつも上書きする。

import { execFileSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { PROFILE_FILES } from "../apps/studio/scripts/sample-profile.mjs";

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

/**
 * 見本のモデルとモジュールと見本のプロファイルを、`to` (`samples/`) にいつも上書きでそのまま写す。見本のプロファイルには
 * 見本の印があり、スタジオは書き換えない (変更は写しに保存させる。docs/studio.md#書き換えないプロファイル、T0866) ので、
 * 前のものを残さない (印の無い前の版の見本も置き換える)。
 */
export function packSamples(to) {
  const samplesDir = join(root, "assets", "samples");
  for (const file of filesUnder(samplesDir, [".ugoe", ".ugoemod"])) {
    copy(join(samplesDir, file), join(to, file));
  }
  for (const name of Object.values(PROFILE_FILES)) {
    copy(join(samplesDir, name), join(to, name));
  }
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

  packSamples(join(dest, "samples"));
  for (const name of ["character.psd", "layers.psd"]) {
    copy(join(root, "assets", "fixtures", name), join(dest, "fixtures", name));
  }
  const checkPath = join(dest, "manual-check.md");
  writeUnlessExists(
    checkPath,
    () => copyFileSync(join(root, "docs", "manual-check.md"), checkPath),
    force,
  );

  console.log(`\npacked into ${dest} (${winDest})`);
  for (const name of installers) console.log(`  installers\\${name}`);
  console.log(`  samples\\ (with ${PROFILE_FILES.ja}), fixtures\\, manual-check.md`);
  console.log("\nnext: docs/manual-check.md (the first round on Windows)");
}

if (process.argv[1] !== undefined && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main(process.argv.slice(2));
}
