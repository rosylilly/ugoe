---
id: T0908
title: i18n: 画面の直書きの文言を見つける検査
status: done
milestone: M8
crates: [scripts, justfile, docs, .github/workflows]
depends_on: [T0901]
assignee: claude-worker-T0908
---

# T0908: i18n: 画面の直書きの文言を見つける検査

## 目的

翻訳を通さない文言が入り込むのを防ぐ。

## 受け入れ条件

- [x] .svelte と画面の .ts の中で、カタログを通さない表示用の文言 (テキストノード・title・aria-label・placeholder など) を見つける検査。
      移行の途中は、アプリごとに許す一覧 (移行が済んだら空にする) を持てる
- [x] `just i18n` に入れ、just ci で動かす
- [x] `just ci` が通る

## 参考

- T0901

## メモ

- やったこと: `scripts/check-hardcoded-strings.mjs` (テストは同じ名前の `.test.mjs`) を作り、`just i18n` に入れた (just ci で動く)。
  `apps/*`・`packages/*` の `.svelte` と画面の `.ts` を `svelte/compiler` の `parse` で読む (`.ts` は `<script lang="ts">` に包む)。
  svelte はルートに無いので、`@ugoe/i18n` の devDependencies から `createRequire` で読む (新しい外部依存なし)。
  見つけるもの・誤検知を減らす約束・`i18n-ignore`・許す一覧の扱いは docs/translation.md#直書きの文言の検査 に書いた。
- 許す一覧 `scripts/hardcoded-strings-allowlist.json` (ファイルと件数。増えたら失敗、減ったら `--update` で縮めるまで失敗):
  apps/editor 80 ファイル 1370 件、apps/viewer 12 ファイル 230 件、packages/player 7 ファイル 62 件。
  packages/ui・packages/i18n・packages/face-tracking は 0 件 (一覧に載せていない)。`packages/ui/gallery` (部品の見本) は対象の外。
- GitHub の CI は just i18n ではなく check-catalogs.mjs を直接呼んでいたため、ci.yml のその直後に直書きの文言の検査とそのテストを足した (team-lead が承認)。
- 後続の候補: アプリごとの移行 (一覧の件数を 0 にする)。エディタの agents/setup.ts のコマンドの例など訳さない文字列には `i18n-ignore` を付ける。
  文言の分割 (`aria-label="{name} を既定値に戻す"` のような文の断片) も移行で 1 つのキーにまとめる。
