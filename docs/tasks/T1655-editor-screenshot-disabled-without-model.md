---
id: T1655
title: "エディタ: 空のプロジェクトでビューポートの「Screenshot」を押せないようにする"
status: in-progress
milestone: M10
crates: [apps/editor, docs]
depends_on: []
assignee: claude-worker-T1655
---

# T1655: エディタ: 空のプロジェクトでビューポートの「Screenshot」を押せないようにする

## 目的

空のプロジェクト (描くものが無い) でも、ビューポートの「Screenshot」が押せる (T0919)。押すと空の画像を保存するか、失敗の知らせになる。
描くものが無いときは押せないようにする。

## 受け入れ条件

- [ ] 描くものが無いとき (空のプロジェクト・モデルを読み込む前・読み込みの失敗) は「Screenshot」が押せない (`disabled`。Tooltip で理由が分かる)。
      モデルがあれば今のまま押せる
- [ ] ショートカットや別の道からも、描くものが無いときは保存の処理に入らない
- [ ] 部品のテスト (Viewport の空の状態でボタンが押せない・モデルを読むと押せる)
- [ ] docs/editor.md (スクリーンショット) を直した。マニュアルに書く必要があればマニュアルの更新の候補をメモに書いた
- [ ] `just ui-audit --only editor` の `--compare` で増えた指摘が無く、`just e2e --only editor` と `just ci` が通る

## 参考

- T0919 のメモ (起票の候補)・docs/roadmap.md の M10 の「後続の候補 (T0919)」
- apps/editor/src/lib/viewport/Viewport.svelte (`screenshot`)・saveScreenshot.ts

## メモ

- T0046 で起票した (元の候補: T0919 (空のプロジェクトでスクリーンショットのボタンが押せる))。
