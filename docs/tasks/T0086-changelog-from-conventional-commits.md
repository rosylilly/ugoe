---
id: T0086
title: "Conventional Commits から変更履歴 (CHANGELOG) とリリースノートを自動で作る方式を決めて入れる"
status: in-progress
milestone: Release
crates: [scripts, justfile, .github/workflows, docs]
depends_on: []
assignee: claude-worker-T0086
---

# T0086: Conventional Commits から変更履歴を自動で作る

## 目的

リポジトリに CHANGELOG もリリースノートの作り方も無い。コミットはすべて Conventional Commits (commitlint で検査) なので、そこから自動で作る方式を固める
(利用者の決定、2026-10-11)。

## 受け入れ条件

- [ ] 道具を決めた。推奨は git-cliff (MIT/Apache-2.0、Rust 製、設定ファイル `cliff.toml`)。外部の道具を足さず scripts/ の Node のスクリプトで作る案と比べ、選んだ理由をメモと docs/release.md に書いた
      (どちらでも、手元と CI で同じ結果になること・版の固定・`just licenses` との関係を確かめる)
- [ ] 分け方を決めた: type ごとの節 (feat・fix・perf・…。docs・chore・test・ci・style は載せないか畳む)、scope の表示、件名の末尾のタスク ID `(T0123)` の扱い (タスクファイルへのリンクにするか)、
      `BREAKING CHANGE` / `!` の扱い。英語で書く (コミットが英語のため)
- [ ] `just changelog` (全体の CHANGELOG.md を作り直す) と、あるタグの範囲のリリースノートを出すレシピ (例: `just release-notes v1.0.0`) を足した
- [ ] タグのときのワークフロー (T0084 の GitHub Release の下書き、または既存の editor-build.yml・crates-release.yml) で、下書きの Release の本文にリリースノートを入れる手順を足すか、
      T0084 の入り方に合わせて後続の候補に書いた (T0084 と並行なので、先に入った方に合わせる)
- [ ] 最初の CHANGELOG.md を作ってコミットするか、タグを打つまで作らないかを決めて書いた (1.0 より前はタグが無いので、「Unreleased」の扱い)
- [ ] docs/release.md に手順を書いた
- [ ] `just ci` が通る

## 参考

- commitlint.config.mjs (type と SCOPES)、docs/release.md、.github/workflows/editor-build.yml・crates-release.yml・web-runtime-release.yml、T0084

## メモ

- 利用者の決定 (2026-10-11): 「conventional commit から自動で生成できる方式を固める」。オーケストレーターが起票した。
