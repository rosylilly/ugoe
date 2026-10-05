// mbx (mr-boxington、https://mr-boxington.jdx.dev/) が cargo を包んでいるかの判定。
// 手順と注意点は docs/workflow/build-cache.md。
//
// 判定のしかた: 次の両方を満たせば有効とみなす。
// 1. `MBX_DISABLE` が無いか、"0" か "false" (大文字小文字を問わない) である。mbx 自身と同じ規則で、
//    それ以外の値なら mbx の shim は素の cargo を呼ぶ。
// 2. PATH で最初に見つかる `cargo` が、`mbx setup` の入れた shim である。shim の隣には
//    mbx の実行ファイルの場所を書いた `mbx-target` がある。
// mise の `[wrappers.cargo]` や `mr_boxington` で包んだ場合は判定できず、無効とみなす (従来どおりの動きになるだけで、壊れはしない)。

import { existsSync, statSync } from "node:fs";
import { homedir } from "node:os";
import { delimiter, isAbsolute, join } from "node:path";

/** `MBX_DISABLE` の値が mbx を無効にするか (mbx の `mbx_disabled_value` と同じ規則)。 */
export function mbxDisabledValue(value) {
  if (value === undefined) return false;
  return value !== "0" && value.toLowerCase() !== "false";
}

/** PATH (`delimiter` 区切り) で最初に見つかる `cargo` のディレクトリ。無ければ undefined。 */
export function findCargoDir(path, { isFile = defaultIsFile, names = cargoNames() } = {}) {
  for (const dir of (path ?? "").split(delimiter)) {
    if (dir && names.some((name) => isFile(join(dir, name)))) return dir;
  }
  return undefined;
}

/** mbx が cargo を包んでいるか。`env` は process.env の形。 */
export function mbxEnabled(env = process.env, { isFile = defaultIsFile } = {}) {
  if (mbxDisabledValue(env.MBX_DISABLE)) return false;
  const dir = findCargoDir(env.PATH ?? env.Path, { isFile });
  return dir !== undefined && isFile(join(dir, "mbx-target"));
}

function defaultIsFile(path) {
  return existsSync(path) && statSync(path).isFile();
}

function cargoNames() {
  return process.platform === "win32" ? ["cargo.exe", "cargo.cmd", "cargo"] : ["cargo"];
}

/**
 * `mbx setup` が cargo の shim を入れるディレクトリ。justfile の `mbx_shim_dir` (`data_local_directory()/mbx/bin`) と同じ場所。
 * Linux は `$XDG_DATA_HOME` (絶対パスのとき) か `~/.local/share`、macOS は `~/Library/Application Support`。
 * Windows では justfile と同じく扱わない (undefined)。
 */
export function mbxShimDir(
  env = process.env,
  { platform = process.platform, home = homedir() } = {},
) {
  if (platform === "win32") return undefined;
  if (platform === "darwin") return join(home, "Library", "Application Support", "mbx", "bin");
  const xdg = env.XDG_DATA_HOME;
  const base = xdg && isAbsolute(xdg) ? xdg : join(home, ".local", "share");
  return join(base, "mbx", "bin");
}

/**
 * just を通らずに cargo を呼ぶスクリプト (bindings/godot/build.mjs など) のための環境。
 * PATH の cargo が mbx でなくても、shim が入っていて `MBX_DISABLE` が無ければ、justfile と同じく shim を PATH の先頭に足す。
 *
 * 返す値: `env` (cargo に渡す環境)、`viaMbx` (その環境の cargo が mbx を通るか)、
 * `installed` (shim が入っているか。`viaMbx` が false でも、入っていれば共有の target に mbx の読み取り専用のハードリンクがありうる)。
 */
export function cargoEnvWithMbx(
  env = process.env,
  { isFile = defaultIsFile, platform = process.platform, home = homedir() } = {},
) {
  const shim = mbxShimDir(env, { platform, home });
  const installed =
    shim !== undefined && isFile(join(shim, "cargo")) && isFile(join(shim, "mbx-target"));
  if (mbxEnabled(env, { isFile })) return { env, viaMbx: true, installed };
  if (installed && !mbxDisabledValue(env.MBX_DISABLE)) {
    const current = env.PATH ?? "";
    const next = { ...env, PATH: current ? `${shim}${delimiter}${current}` : shim };
    return { env: next, viaMbx: true, installed };
  }
  return { env, viaMbx: false, installed };
}

/**
 * just を通らずに cargo を呼ぶスクリプトの始めに呼ぶ。`cargoEnvWithMbx` と同じ判定で、mbx を通すなら `env` (既定は process.env) の
 * PATH に shim を足す (justfile の `export PATH` と同じ)。process.env を書き換えると、`env` を渡さずに起動する子プロセスの
 * cargo も、cargo を呼ぶ pnpm・tauri・cargo-xwin なども mbx を通る。
 *
 * 返す値: その環境の cargo が mbx を通るか。shim が無い・`MBX_DISABLE` がある・Windows では何も変えずに false を返す
 * (PATH の cargo がすでに shim なら何も変えずに true)。
 */
export function exportMbxPath(
  env = process.env,
  { isFile = defaultIsFile, platform = process.platform, home = homedir() } = {},
) {
  const { env: next, viaMbx } = cargoEnvWithMbx(env, { isFile, platform, home });
  if (next !== env) env.PATH = next.PATH;
  return viaMbx;
}
