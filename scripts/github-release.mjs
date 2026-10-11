#!/usr/bin/env node
// タグの GitHub Release (下書き) に配布物を添付する。`v*` のタグで動くワークフローが呼ぶ (docs/release.md#github-release-の下書き)。
//
//   node scripts/github-release.mjs upload <タグ> <ファイル...>
//
// - そのタグの Release が無ければ下書きで作る。公開するのは人の手 (このスクリプトは公開しない)。
// - 公開済みの Release には添付しない (配った物を差し替えないため。止まる)。
// - いくつものワークフロー (エディタのインストーラ・C ABI・Unity・Godot) が同じ下書きに添付する。同時に作って下書きが重なったら、
//   いちばん古いものに寄せ、自分が作った余分な下書きを消す。
// - 同じ名前のファイルがあれば置き換える (ジョブをやり直したとき)。
// - gh (GitHub CLI) と GH_TOKEN (contents: write) が要る。GitHub のランナーには gh が入っている。

import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";

/** Release の題と本文 (下書き。公開するときに人が書き直す)。 */
export function draftNotes(tag) {
  return (
    `Draft for ${tag}. Assets are attached by the release workflows ` +
    "(docs/release.md#github-release-の下書き). Review and edit the notes before publishing."
  );
}

/**
 * Release の一覧 (GitHub の API の形) から、タグの Release を選ぶ (純粋な関数。テストする)。
 * 公開済みがあればそれ (`published: true`)。無ければ下書きのうち id のいちばん小さいもの (いちばん古い)。
 * `extra` は寄せた後に残る、ほかの下書き。
 */
export function pickRelease(releases, tag) {
  const ours = releases.filter((r) => r.tag_name === tag);
  const published = ours.find((r) => !r.draft);
  if (published) return { release: published, published: true, extra: [] };
  const drafts = ours.filter((r) => r.draft).sort((a, b) => a.id - b.id);
  return { release: drafts[0] ?? null, published: false, extra: drafts.slice(1) };
}

function gh(args, { capture = false } = {}) {
  const r = spawnSync("gh", args, {
    encoding: "utf8",
    stdio: capture ? ["ignore", "pipe", "inherit"] : "inherit",
  });
  if (r.error) throw new Error(`failed to run gh: ${r.error.message}`);
  if (r.status !== 0) throw new Error(`gh ${args.slice(0, 2).join(" ")} failed (exit ${r.status})`);
  return r.stdout;
}

/** リポジトリの Release (下書きを含む。下書きはタグからの API では引けないので一覧から探す)。 */
function listReleases() {
  const out = gh(
    ["api", "--paginate", "repos/{owner}/{repo}/releases", "--jq", ".[] | {id, tag_name, draft}"],
    {
      capture: true,
    },
  );
  return out
    .split("\n")
    .filter((line) => line.trim() !== "")
    .map((line) => JSON.parse(line));
}

function upload(tag, files) {
  const missing = files.filter((f) => !existsSync(f));
  if (files.length === 0 || missing.length > 0) {
    throw new Error(
      files.length === 0 ? "no files to upload" : `missing files: ${missing.join(", ")}`,
    );
  }

  let picked = pickRelease(listReleases(), tag);
  if (picked.published) {
    throw new Error(`the release ${tag} is already published; not replacing its assets`);
  }
  if (!picked.release) {
    console.error(`creating a draft release for ${tag}`);
    const url = gh(
      [
        "release",
        "create",
        tag,
        "--draft",
        "--verify-tag",
        "--title",
        tag,
        "--notes",
        draftNotes(tag),
      ],
      { capture: true },
    ).trim();
    console.error(url);
    // 作った後に一覧を読み直し、ほかのワークフローが同時に作った下書きと重なっていないかを見る
    picked = pickRelease(listReleases(), tag);
    for (const extra of picked.extra) {
      // 消すのは添付の無い下書きだけ (ほかのワークフローが添付し始めたものは消さない)
      const assets = gh(
        ["api", `repos/{owner}/{repo}/releases/${extra.id}/assets`, "--jq", "length"],
        {
          capture: true,
        },
      ).trim();
      if (assets !== "0") continue;
      console.error(`removing a duplicate draft release (id ${extra.id})`);
      try {
        gh(["api", "--method", "DELETE", `repos/{owner}/{repo}/releases/${extra.id}`]);
      } catch (e) {
        // ほかのワークフローが先に消したときも失敗する。残っていれば次の一覧で見える
        console.error(`warning: ${e.message}`);
      }
    }
  }
  if (!picked.release) throw new Error(`could not find the draft release for ${tag}`);

  // gh release upload はタグで Release を引く (下書きも引ける)。重なりは上で消してあるので、引けるのは 1 つ
  gh(["release", "upload", tag, ...files, "--clobber"]);
  console.error(`uploaded ${files.length} file(s) to the draft release ${tag}`);
}

function main(argv) {
  const [command, tag, ...files] = argv;
  if (command !== "upload" || !tag) {
    console.error("usage: node scripts/github-release.mjs upload <tag> <files...>");
    return 2;
  }
  try {
    upload(tag, files);
    return 0;
  } catch (e) {
    console.error(`error: ${e.message}`);
    return 1;
  }
}

if (import.meta.main) process.exitCode = main(process.argv.slice(2));
