// `cargo … --locked` が、lock のファイルが古いために失敗したかの判定と、そのときに出す短い文。
// scripts/check-licenses.mjs の `cargo metadata --locked` が使う (直し方は docs/testing.md#fuzz と bindings/godot/README.md)。

import { dirname, isAbsolute, join, relative, sep } from "node:path";

// cargo の文言: 新しい版は "cannot update the lock file <path> because --locked was passed to prevent this"、
// 古い版は "the lock file <path> needs to be updated but --locked was passed to prevent this"
const STALE_LOCK =
  /lock file (.+?) (?:because --locked was passed|needs to be updated but --locked was passed)/;

/** cargo の標準エラー出力から、古いと言われた lock のファイルのパス。lock が古いための失敗でなければ undefined。 */
export function staleLockPath(stderr) {
  return STALE_LOCK.exec(stderr ?? "")?.[1];
}

/**
 * lock のファイルが古いための失敗なら、どのファイルが古いかと直し方を書いた短い文 (複数行)。そうでなければ undefined。
 * パスは `root` (リポジトリのルート) からの相対で書く (`root` の外ならそのまま)。
 */
export function staleLockMessage(stderr, root) {
  const lock = staleLockPath(stderr);
  if (lock === undefined) return undefined;
  const shown = (path) => {
    const rel = relative(root, path);
    if (rel === "" || rel.startsWith("..") || isAbsolute(rel)) return path;
    return rel.split(sep).join("/");
  };
  const manifest = shown(join(dirname(lock), "Cargo.toml"));
  return [
    `error: ${shown(lock)} が古い (Cargo.toml の依存と合わず、--locked で止めた)`,
    `直し方: cargo update --manifest-path ${manifest} --workspace で足りない依存を足し、lock のファイルをコミットする`,
  ].join("\n");
}
