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
import { delimiter, join } from "node:path";

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
