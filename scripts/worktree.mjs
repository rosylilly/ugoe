#!/usr/bin/env node
// タスク用 git worktree の作成と削除。手順の正は docs/workflow/multi-agent.md。
//
//   node scripts/worktree.mjs add <TASK_ID> [slug]     .worktrees/<TASK_ID> を作る
//   node scripts/worktree.mjs remove <TASK_ID> [--delete-branch]
//
// add では、ビルドをやり直さずに済むよう main の作業ツリーの target/ をコピーする。
// - ワークスペースのクレートの incremental キャッシュはコピーしない (パスが変わるため再利用できない)
// - 更新時刻を保つ (cargo は更新時刻で再ビルドの要否を判断するため)
// - reflink に対応したファイルシステム (btrfs / XFS / APFS) ではコピーオンライトで一瞬で終わる
// node_modules は pnpm のストアからハードリンクされるので、`pnpm install --offline` で十分速い。

import { execFileSync } from "node:child_process";
import { constants, cpSync, existsSync, readdirSync, statSync } from "node:fs";
import { basename, dirname, join, relative, sep } from "node:path";

const run = (cmd, args, opts = {}) => execFileSync(cmd, args, { stdio: "inherit", ...opts });
const capture = (cmd, args, opts = {}) =>
  execFileSync(cmd, args, { encoding: "utf8", ...opts }).trim();

// main の作業ツリーのルート (どの worktree から実行しても同じ場所を指す)
const root = dirname(capture("git", ["rev-parse", "--path-format=absolute", "--git-common-dir"]));

function usage() {
  console.error("usage: worktree.mjs add <TASK_ID> [slug] | remove <TASK_ID> [--delete-branch]");
  process.exit(2);
}

function taskSlug(id) {
  const file = readdirSync(join(root, "docs", "tasks")).find((f) => f.startsWith(`${id}-`));
  if (!file) {
    console.error(`error: docs/tasks/${id}-*.md が見つかりません`);
    process.exit(1);
  }
  return basename(file, ".md").slice(id.length + 1);
}

// コピーしないもの: incremental キャッシュと rust-analyzer 用の一時ディレクトリ
function copyFilter(src) {
  const parts = relative(join(root, "target"), src).split(sep);
  return !parts.includes("incremental") && !parts[0]?.startsWith("flycheck");
}

function add(id, slug = taskSlug(id)) {
  const path = join(root, ".worktrees", id);
  const branch = `task/${id}-${slug}`;
  if (existsSync(path)) {
    console.error(`error: ${relative(root, path)} は既に存在します`);
    process.exit(1);
  }
  run("git", ["worktree", "add", path, "-b", branch, "main"], { cwd: root });

  const target = join(root, "target");
  if (existsSync(target) && statSync(target).isDirectory()) {
    const started = Date.now();
    console.log("copying target/ from main (incremental caches excluded)...");
    cpSync(target, join(path, "target"), {
      recursive: true,
      preserveTimestamps: true,
      filter: copyFilter,
      mode: constants.COPYFILE_FICLONE, // reflink が使えれば使い、使えなければ通常のコピー
    });
    console.log(`copied target/ in ${((Date.now() - started) / 1000).toFixed(1)}s`);
  } else {
    console.log("main に target/ がないためコピーを省略します (初回ビルドは時間がかかります)");
  }

  run("pnpm", ["install", "--offline"], { cwd: path });
  console.log(`\nready: ${relative(root, path)} (branch ${branch})`);
}

function remove(id, deleteBranch) {
  const path = join(root, ".worktrees", id);
  const branch = capture("git", ["rev-parse", "--abbrev-ref", "HEAD"], { cwd: path });
  // 未コミットの変更があれば git が拒否する (--force は使わない)
  run("git", ["worktree", "remove", path], { cwd: root });
  if (deleteBranch) run("git", ["branch", "-D", branch], { cwd: root });
  console.log(`removed ${relative(root, path)}${deleteBranch ? ` and branch ${branch}` : ""}`);
}

const [command, id, ...rest] = process.argv.slice(2);
if (!id || !/^T\d{4}$/.test(id)) usage();
if (command === "add") add(id, rest[0]);
else if (command === "remove") remove(id, rest.includes("--delete-branch"));
else usage();
