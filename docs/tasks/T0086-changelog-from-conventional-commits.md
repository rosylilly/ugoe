---
id: T0086
title: "Conventional Commits から変更履歴 (CHANGELOG) とリリースノートを自動で作る方式を決めて入れる"
status: done
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

- [x] 道具を決めた。推奨は git-cliff (MIT/Apache-2.0、Rust 製、設定ファイル `cliff.toml`)。外部の道具を足さず scripts/ の Node のスクリプトで作る案と比べ、選んだ理由をメモと docs/release.md に書いた
      (どちらでも、手元と CI で同じ結果になること・版の固定・`just licenses` との関係を確かめる)
- [x] 分け方を決めた: type ごとの節 (feat・fix・perf・…。docs・chore・test・ci・style は載せないか畳む)、scope の表示、件名の末尾のタスク ID `(T0123)` の扱い (タスクファイルへのリンクにするか)、
      `BREAKING CHANGE` / `!` の扱い。英語で書く (コミットが英語のため)
- [x] `just changelog` (全体の CHANGELOG.md を作り直す) と、あるタグの範囲のリリースノートを出すレシピ (例: `just release-notes v1.0.0`) を足した
- [x] タグのときのワークフロー (T0084 の GitHub Release の下書き、または既存の editor-build.yml・crates-release.yml) で、下書きの Release の本文にリリースノートを入れる手順を足すか、
      T0084 の入り方に合わせて後続の候補に書いた (T0084 と並行なので、先に入った方に合わせる)
- [x] 最初の CHANGELOG.md を作ってコミットするか、タグを打つまで作らないかを決めて書いた (1.0 より前はタグが無いので、「Unreleased」の扱い)
- [x] docs/release.md に手順を書いた
- [x] `just ci` が通る

## 参考

- commitlint.config.mjs (type と SCOPES)、docs/release.md、.github/workflows/editor-build.yml・crates-release.yml・web-runtime-release.yml、T0084

## メモ

- 利用者の決定 (2026-10-11): 「conventional commit から自動で生成できる方式を固める」。オーケストレーターが起票した。
- やったこと (claude-worker-T0086):
  - 道具は git-cliff 2.14.2 (MIT OR Apache-2.0)。Node のスクリプトの案と比べ、フッター (`BREAKING CHANGE:`)・`!`・タグの範囲の
    扱いを自分で持たずに設定とテンプレートだけで書けるので選んだ (docs/release.md#道具)。版は scripts/changelog.mjs の
    `GIT_CLIFF_VERSION` に 1 つだけ書き、違う版なら入れ方を出して止まる。CI は `just git-cliff-version` の版を
    taiki-e/install-action で入れる。手元のツールでワークスペースや npm の依存に入らないので `just licenses` の対象外。
  - 設定は scripts/cliff.toml (タスクの `crates:` に合わせてルートではなく scripts/ に置いた)。節は Breaking changes・Features・
    Bug fixes・Performance・Reverts。build・chore・ci・docs・refactor・style・test は載せない (破壊的なものは Breaking changes に載る)。
    scope は太字の接頭辞、タスク ID は文字のまま (ファイル名に slug が付き、ID だけから URL を作れないため)、各行に短いハッシュと
    コミットへのリンク。英語。
  - `just changelog [<タグ>]`・`just release-notes <タグ> [--output <file>]`・`just git-cliff-version` を足した (Node の
    scripts/changelog.mjs)。最初の版は根のコミットから、まだ無いタグは未リリースの下見。仮のタグを打った clone で、範囲・最初の版・
    下見・破壊的な変更 (`!` とフッター) の出力を確かめた。
  - 単体テスト scripts/changelog.test.mjs (`just script-check` に追加): 版の読み取り、タグの形と tag_pattern の一致、
    範囲の引数、commitlint のすべての type が parser のどれかに当たり載るのが feat・fix・perf・revert だけであること。
  - .github/workflows/release-notes.yml: `v*` のタグでそのタグのノートを作り、ジョブの要約と成果物に置く。設定・スクリプトを
    変えた PR と手での起動では下見を作る。
  - CHANGELOG.md は最初のタグまで作らない・コミットしない。リリースのときは、タグを打つ前に `just changelog v<版>` で作り直して
    版を上げるコミットに含める (docs/release.md#changelogmd-の扱い)。
- T0084 が main に入った後 (オーケストレーターの指示で、このタスクの中で済ませた):
  - scripts/github-release.mjs に `notes <タグ> <ファイル>` を足した。下書きを見つける・作る・重なりを寄せる処理を `ensureDraft` に
    まとめ、`upload` (添付だけ) と `notes` (本文だけ。id で PATCH) の両方が使う。どちらが先に動いても同じ下書きに寄る。
    引数の読み取り `parseArgs` を単体テストに足した。
  - release-notes.yml に、タグのときだけ動く `draft` のジョブ (contents: write) を足し、生成したノートを下書きの本文にする。
    actionlint 1.7.12 (shellcheck 0.9.0) で release-notes.yml と editor-build.yml を検査して通った。
  - docs/release.md の「版を上げる」に `just changelog v<版>` の段を足し、「GitHub Release の下書き」に `notes` を書いた。
  - AGENTS.md のコマンドの表に `just changelog`・`just release-notes` を足した (タスクの `crates:` の外。指示による)。
- 確かめていないこと: release-notes.yml と `github-release.mjs notes` は実際の GitHub のランナー・Release では動かしていない
  (gh api の PATCH と `-F body=@file` は手元で呼んでいない)。
- 起票の候補: なし。
