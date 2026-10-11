#!/usr/bin/env node

// `just changelog`・`just release-notes`・`just git-cliff-version` の実体 (docs/release.md#変更履歴とリリースノート)。
// Conventional Commits のコミットから git-cliff で変更履歴とリリースノートを作る。設定は scripts/cliff.toml。
// Windows (PowerShell) でも同じように動くよう、bash ではなく Node で書く。
//
//   node scripts/changelog.mjs version                       固定した git-cliff の版を出力する (CI はこの版を入れる)
//   node scripts/changelog.mjs changelog [<タグ>]            CHANGELOG.md を作り直す。<タグ> (まだ打っていないもの) を渡すと、
//                                                            未リリースのコミットをその版として書く (リリースの準備)
//   node scripts/changelog.mjs notes <タグ> [--output <file>] <タグ> の範囲 (1 つ前の版のタグから <タグ> まで) のリリースノート。
//                                                            <タグ> がまだ無ければ、未リリースのコミットをその版として出す (下見)
//
// git-cliff は GIT_CLIFF_VERSION の版でなければならない (版で出力が変わりうるので、手元と CI で揃える):
//   cargo install git-cliff --version <version> --locked

import { execFileSync, spawnSync } from "node:child_process";
import { join, resolve } from "node:path";

const repoRoot = join(import.meta.dirname, "..");

/** 使う git-cliff の版。上げたら `just changelog` の出力が変わらないかを確かめる。 */
export const GIT_CLIFF_VERSION = "2.14.2";
/** git-cliff の設定 (リポジトリのルートからの位置)。 */
export const CONFIG = "scripts/cliff.toml";
/** 作り直す変更履歴 (リポジトリのルートからの位置)。 */
export const CHANGELOG = "CHANGELOG.md";
/** 版のタグ (docs/release.md。scripts/cliff.toml の tag_pattern と同じ)。 */
export const VERSION_TAG = /^v[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$/;

/** `git-cliff --version` の出力 (例: `git-cliff 2.14.2`) から版を取り出す。取り出せなければ undefined。 */
export function parseCliffVersion(output) {
  return output.match(/^git-cliff\s+v?([0-9]+\.[0-9]+\.[0-9]+\S*)/m)?.[1];
}

/**
 * リリースノートを作る git-cliff の引数 (設定と出力先を除く)。
 * - `exists` が偽 (まだ打っていないタグ): 未リリースのコミットを `tag` の版として出す。
 * - `previous` (1 つ前の版のタグ) があれば `previous..tag`。
 * - 最初の版 (前のタグが無い) では、根のコミットから `tag` まで (`root..tag`。根のコミット自身は含まれないが、
 *   根は `chore(repo): bootstrap ...` で、もともと載らない)。
 */
export function notesArgs({ tag, exists, previous, root }) {
  if (!exists) return ["--unreleased", "--tag", tag, "--strip", "all"];
  const range = previous ? `${previous}..${tag}` : `${root}..${tag}`;
  return ["--strip", "all", range];
}

function git(args) {
  return execFileSync("git", args, { cwd: repoRoot, encoding: "utf8" }).trim();
}

/** 失敗したら undefined を返す git (タグが無い・前の版が無いなど)。 */
function gitMaybe(args) {
  const r = spawnSync("git", args, { cwd: repoRoot, encoding: "utf8" });
  return r.status === 0 ? r.stdout.trim() : undefined;
}

/** git-cliff が固定した版で入っているかを確かめる。違えば入れ方を出して終わる。 */
function checkCliff() {
  const r = spawnSync("git-cliff", ["--version"], { encoding: "utf8" });
  const found = r.status === 0 ? parseCliffVersion(r.stdout) : undefined;
  if (found === GIT_CLIFF_VERSION) return;
  console.error(
    found === undefined
      ? "git-cliff is not installed."
      : `git-cliff ${found} is installed, but ${GIT_CLIFF_VERSION} is required.`,
  );
  console.error(`Install it with: cargo install git-cliff --version ${GIT_CLIFF_VERSION} --locked`);
  process.exit(1);
}

function cliff(args) {
  const r = spawnSync("git-cliff", ["--config", CONFIG, ...args], {
    cwd: repoRoot,
    stdio: "inherit",
  });
  if (r.error) throw new Error(`failed to run git-cliff: ${r.error.message}`);
  if (r.status !== 0) process.exit(r.status ?? 1);
}

function requireVersionTag(tag) {
  if (!VERSION_TAG.test(tag ?? "")) {
    console.error(`expected a version tag like v1.0.0, got: ${tag ?? "(none)"}`);
    process.exit(1);
  }
}

function changelog(tag) {
  if (tag !== undefined) requireVersionTag(tag);
  checkCliff();
  cliff(["--output", CHANGELOG, ...(tag === undefined ? [] : ["--tag", tag])]);
}

function notes(tag, output) {
  requireVersionTag(tag);
  checkCliff();
  const exists = gitMaybe(["rev-parse", "--verify", "--quiet", `refs/tags/${tag}^{commit}`]);
  const args = notesArgs({
    tag,
    exists: exists !== undefined,
    previous: exists
      ? gitMaybe(["describe", "--tags", "--abbrev=0", "--match", "v[0-9]*", `${tag}^`])
      : undefined,
    root: exists ? git(["rev-list", "--max-parents=0", tag]).split("\n")[0] : undefined,
  });
  cliff([...args, ...(output === undefined ? [] : ["--output", output])]);
}

function main(argv) {
  const [command, ...rest] = argv;
  switch (command) {
    case "version":
      console.log(GIT_CLIFF_VERSION);
      return;
    case "changelog":
      changelog(rest[0] || undefined);
      return;
    case "notes": {
      const i = rest.indexOf("--output");
      // 出力先は呼んだ場所からの位置 (git-cliff はリポジトリのルートで動かす)
      const output = i >= 0 ? resolve(rest[i + 1]) : undefined;
      const tag = i >= 0 ? rest.toSpliced(i, 2)[0] : rest[0];
      notes(tag, output);
      return;
    }
    default:
      console.error(
        "usage: node scripts/changelog.mjs version | changelog [<tag>] | notes <tag> [--output <file>]",
      );
      process.exit(2);
  }
}

if (import.meta.main) main(process.argv.slice(2));
