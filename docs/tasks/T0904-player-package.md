---
id: T0904
title: 共有: ビューアのモデル再生の層を @ugoe/player に切り出す
status: in-progress
milestone: M8
crates: [packages, apps/viewer, docs]
depends_on: [T0900]
assignee: claude-worker-T0904
---

# T0904: 共有: ビューアのモデル再生の層を @ugoe/player に切り出す

## 目的

スタジオはビューアと同じ再生の層 (モデルの読み込み・モーション・表情・トラッキング・ライティング・着せ替え・覚えた設定) を使う。
コピーせずに共有するため、ビューアから切り出す。

## 受け入れ条件

- [ ] apps/viewer/src/lib の player.svelte.ts・animation・lighting・dressUp・drivers・modelMemory などを packages/player に移し、ビューアはそれを使う
- [ ] 画面に依存する部分 (Svelte の部品) とそうでない部分を分ける。wasm の読み込みはアプリが渡す形にする
- [ ] 振る舞いを変えない (ビューアの Vitest がそのまま通り、ヘッドレスの Chrome で見た目が同じ)
- [ ] `just ci`・ビューアのビルドが通る

## 参考

- apps/viewer/README.md

## メモ
