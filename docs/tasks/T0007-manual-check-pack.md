---
id: T0007
title: 実機で手で確かめるための準備 (インストーラと確認表)
status: in-progress
milestone: M8
crates: [docs, scripts, justfile]
depends_on: [T0004, T0812, T0813]
assignee: claude-worker-T0007
---

# T0007: 手で確かめる準備

## 目的

これまでのタスクの「確かめていないこと」は、各タスクのメモ・docs/studio.md の一覧・T0619 などに散らばっている。利用者が実機 (特に Windows) で効率よく確かめられるように、
インストーラと優先度つきの確認表を 1 つにまとめる。

## 受け入れ条件

- [ ] `just app::build PLATFORM=windows` で 3 つのアプリの Windows 向けのインストーラを作り、置き場所を報告する (WSL2 からのクロスビルド)。Linux 向けも作れるなら作る
- [ ] 確認表 (docs/manual-check.md など。置き場所は決めて README から辿れるようにする): 各タスクのメモと docs/studio.md・T0619 の「確かめていないこと」を集め、
      重複をまとめ、優先度 (配信で使う流れ・壊れると困るもの・OS ごとにしか分からないもの) を付けて並べる。1 項目ごとに「手順・期待する結果・結果を書く欄」
- [ ] 優先度の高い順に、Windows で 30 分ほどで回せる「最初の一巡」を作る (スタジオでモデルを開く・トラッキング・出力を OBS に取り込む・仮想カメラ・ホットキー など)
- [ ] 結果を書き戻す流れ (確認表に書いて報告してもらう → 不具合はタスクにする) を書く。T0619 との関係を整理する (T0619 を確認表に吸収するか、残すか)
- [ ] `just ci` が通る

## 参考

- docs/tasks/*.md のメモの「確かめていないこと」、docs/studio.md の「実機で確かめていないこと」、docs/tasks/T0619-editor-manual-check.md、docs/release.md

## メモ
