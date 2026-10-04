---
id: T0826
title: スタジオ: 出力の受け手のシーンの適用をテストする
status: in-progress
milestone: M9
crates: [apps/studio, docs]
depends_on: [T0815]
assignee: claude-worker-T0826
---

# T0826: スタジオ: 出力の受け手のシーンの適用をテストする

## 目的

`Output.svelte` の `onScene` / `applyScene` / `showLook` (シーンの通知を 1 つずつ順に処理する・読んでいる間に次のシーンが来たら古い方を捨てる・
モデルの読み直しとモジュールの付け直しと見せ方だけの変更・背景の画像の blob URL の作り直しと解放) にはテストが無い。
判断 (`planScene`) と配置の計算 (`stageLayout`) だけが lib/outputLayout.ts でテストされている (T0815 のレビューで見つけた)。

`Player` と配信の口 (`OutputFeed`) を差し替えられる形に処理を lib へ分け、順序と取り消しの振る舞いをテストする。

## 受け入れ条件

- [ ] 続けて届いたシーンのうち、最後のものだけが当たる (途中のモデルを読まない・古いシーンの見せ方を当てない)
- [ ] 同じモデルのシーンでは読み直さず、モジュールの組が変われば付け直す
- [ ] 背景の画像を替えたら前の blob URL を解放する
- [ ] `just ci` が通る

## 参考

- docs/studio.md#シーンの切り替えと受け手
- apps/studio/src/lib/outputLayout.ts

## メモ
