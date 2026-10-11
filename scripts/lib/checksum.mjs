// 配布物の隣に置く SHA-256 のファイル (`<ファイル>.sha256`) の読み書き (docs/release.md#配布物を確かめる)。
// 中身は `sha256sum` と同じ 1 行 (`<16 進のダイジェスト>  <ファイル名>`) なので、利用者は同じディレクトリで `sha256sum -c` で確かめられる。
// scripts/ffi-dist.mjs (C ABI のアーカイブ) と scripts/app-build.mjs (アプリのインストーラ) が使う。

import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { basename } from "node:path";

/** ファイルの SHA-256 (16 進の小文字)。 */
export function sha256File(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

/** `<ファイル>.sha256` の中身 (`sha256sum` の形の 1 行。ファイル名はディレクトリを除いたもの)。 */
export function checksumLine(sha, path) {
  return `${sha}  ${basename(path)}\n`;
}

/** `<ファイル>.sha256` を隣に書き、ダイジェストを返す。 */
export function writeChecksum(path) {
  const sha = sha256File(path);
  writeFileSync(`${path}.sha256`, checksumLine(sha, path));
  return sha;
}

/** `<ファイル>.sha256` に書いたダイジェスト (行の最初の語)。 */
export function readChecksum(sumsPath) {
  return readFileSync(sumsPath, "utf8").split(/\s+/)[0];
}
