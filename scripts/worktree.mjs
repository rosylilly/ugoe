#!/usr/bin/env node
// タスク用 git worktree の作成と削除。手順の正は docs/workflow/multi-agent.md。
//
//   node scripts/worktree.mjs add <TASK_ID> [slug]     .worktrees/<TASK_ID> を作る
//   node scripts/worktree.mjs remove <TASK_ID> [--delete-branch]
//
// add では、ビルドをやり直さずに済むよう main の作業ツリーの target/ をコピーする。
// - mbx が cargo を包んでいるとき (判定は scripts/lib/mbx.mjs) はコピーしない。依存クレートの成果物は
//   mbx の置き場から戻るので要らない (docs/workflow/build-cache.md)
// - main の target/ がシンボリックリンク (mbx の managed target) のときもコピーしない。リンクを写すと
//   worktree が main と同じ target を指してしまい、実体を写すと置き場の全体 (数十 GB) を写すことになる
// - ワークスペースのクレートの incremental キャッシュはコピーしない (パスが変わるため再利用できない)
// - 更新時刻を保つ (cargo は更新時刻で再ビルドの要否を判断するため)
// - reflink に対応したファイルシステム (btrfs / XFS / APFS) ではコピーオンライトで一瞬で終わる
// node_modules は pnpm のストアからハードリンクされるので、`pnpm install --offline` で十分速い。
//
// add では、mbx が有効なら .worktrees/ の下の worktree だけに効く cargo の設定 `.worktrees/.cargo/config.toml` も書く
// (dev のデバッグ情報を行の表だけにする。scripts/lib/worktree-cargo-config.mjs)。

import { execFileSync } from "node:child_process";
import {
  constants,
  cpSync,
  existsSync,
  lstatSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  writeFileSync,
} from "node:fs";
import { basename, dirname, join, relative, sep } from "node:path";
import { mbxEnabled } from "./lib/mbx.mjs";
import { CARGO_CONFIG, cargoConfigAction } from "./lib/worktree-cargo-config.mjs";

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

  // mbx が無効なときは書かない。コピーする main の target/ は `debug = true` なので、設定が違うと使われない
  if (mbxEnabled()) writeCargoConfig();
  copyTarget(path);
  run("pnpm", ["install", "--offline"], { cwd: path });
  console.log(`\nready: ${relative(root, path)} (branch ${branch})`);
}

// .worktrees/.cargo/config.toml を書く。手で書いたものは上書きしない
function writeCargoConfig() {
  const dir = join(root, ".worktrees", ".cargo");
  const file = join(dir, "config.toml");
  const current = existsSync(file) ? readFileSync(file, "utf8") : undefined;
  const action = cargoConfigAction(current);
  if (action === "write") {
    mkdirSync(dir, { recursive: true });
    writeFileSync(file, CARGO_CONFIG);
    console.log(`wrote ${relative(root, file)} (dev のデバッグ情報を行の表だけにする)`);
  } else if (action === "skip") {
    console.log(`${relative(root, file)} は手で書かれているので書き直しません`);
  }
}

function copyTarget(path) {
  if (mbxEnabled()) {
    console.log(
      "mbx が有効なため target/ のコピーを省略します (依存クレートは mbx の置き場から戻ります)",
    );
    return;
  }
  const target = join(root, "target");
  const stat = existsSync(target) ? lstatSync(target) : undefined;
  if (stat?.isSymbolicLink()) {
    console.log(
      "main の target/ は mbx の managed target (シンボリックリンク) なのでコピーを省略します。" +
        "MBX_DISABLE を外して mbx を通すと、依存クレートは置き場から戻ります",
    );
  } else if (stat?.isDirectory()) {
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
}

function remove(id, deleteBranch) {
  const path = join(root, ".worktrees", id);
  const branch = capture("git", ["rev-parse", "--abbrev-ref", "HEAD"], { cwd: path });
  const managedTarget = lstatSync(join(path, "target"), {
    throwIfNoEntry: false,
  })?.isSymbolicLink();
  // 未コミットの変更があれば git が拒否する (--force は使わない)
  run("git", ["worktree", "remove", path], { cwd: root });
  // mbx の managed target は worktree の外 (~/.cache/mbx/targets) にあるので、ここで片付ける。
  // 失敗しても mbx の gc がいずれ集める (target.max_age)
  if (managedTarget) {
    try {
      run("mbx", ["clean", path]);
    } catch {
      console.warn(
        `warning: mbx clean ${path} に失敗しました。あとで \`mbx gc\` で片付けてください`,
      );
    }
  }
  if (deleteBranch) run("git", ["branch", "-D", branch], { cwd: root });
  console.log(`removed ${relative(root, path)}${deleteBranch ? ` and branch ${branch}` : ""}`);
}

const [command, id, ...rest] = process.argv.slice(2);
if (!id || !/^T\d{4}$/.test(id)) usage();
if (command === "add") add(id, rest[0]);
else if (command === "remove") remove(id, rest.includes("--delete-branch"));
else usage();
