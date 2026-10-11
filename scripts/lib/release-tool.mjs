// 版と sha256 を固定した道具を GitHub の Release から取得する (scripts/secrets.mjs の gitleaks・scripts/actionlint.mjs の
// actionlint と shellcheck)。置き場は `<メインの作業ツリー>/target/tools/<名前>-<版>/` (worktree でも共有する。リポジトリには置かない)。

import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { dirname, isAbsolute, join, resolve } from "node:path";

/** 実行する機械の鍵 (`${platform}-${arch}`)。道具ごとの Release のファイルの表を引くのに使う。 */
export function platformKey(platform = process.platform, arch = process.arch) {
  return `${platform}-${arch}`;
}

/** メインの作業ツリー (worktree なら、その元)。`root` が Git の作業ツリーでなければ `root`。 */
export function mainWorktree(root) {
  const r = spawnSync("git", ["rev-parse", "--git-common-dir"], { cwd: root, encoding: "utf8" });
  if (r.status !== 0) return root;
  const common = r.stdout.trim();
  return dirname(isAbsolute(common) ? common : resolve(root, common));
}

/**
 * 道具を (無ければ取得して sha256 を確かめてから展開して) 用意し、実行ファイルのパスを返す。
 * - `root`: リポジトリのルート (置き場をメインの作業ツリーから決める)。
 * - `name`・`version`: 置き場のディレクトリの名前に使う。
 * - `asset`: `{ file, sha256, exe }`。`exe` は展開した中の実行ファイルの相対パス。
 * - `url`: `asset.file` の URL。
 */
export async function ensureReleaseTool({ root, name, version, asset, url }) {
  const dir = join(mainWorktree(root), "target", "tools", `${name}-${version}`);
  const exe = join(dir, asset.exe);
  if (existsSync(exe)) return exe;

  console.log(`downloading ${url}`);
  const res = await fetch(url);
  if (!res.ok) throw new Error(`failed to download ${url}: ${res.status} ${res.statusText}`);
  const data = Buffer.from(await res.arrayBuffer());
  const sha = createHash("sha256").update(data).digest("hex");
  if (sha !== asset.sha256) {
    throw new Error(`sha256 mismatch for ${asset.file}: expected ${asset.sha256}, got ${sha}`);
  }
  mkdirSync(dir, { recursive: true });
  const archive = join(dir, asset.file);
  writeFileSync(archive, data);
  // tar.gz と zip のどちらも tar で展開する (Windows の tar は bsdtar で zip も読める)
  const r = spawnSync("tar", ["-xf", archive, "-C", dir], { stdio: "inherit" });
  rmSync(archive, { force: true });
  if (r.status !== 0 || !existsSync(exe)) throw new Error(`failed to extract ${asset.file}`);
  return exe;
}
