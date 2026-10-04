---
id: T0933
title: "@ugoe/player: player.test.ts を流れごとに分け、偽の WebPuppet をテスト用の部品にする"
status: in-progress
milestone: M9
crates: [packages]
depends_on: [T0930]
assignee: claude-worker-T0933
---

# T0933: @ugoe/player: player.test.ts を流れごとに分け、偽の WebPuppet をテスト用の部品にする

## 目的

T0931 で `@ugoe/player` の振る舞いのテストを足し、`packages/player/src/player.test.ts` は約 1000 行になった。
読み込み・着せ替え・トラッキング・覚えるものなどの流れごとにファイルを分け、ファイルの中にある偽の WebPuppet を
テスト用の部品 (`src/testing/fakePuppet.ts` など) に出して共有する (振る舞いは変えない)。

## 受け入れ条件

- [ ] テストを流れごとのファイルに分け、どれも 500 行程度以下にした
- [ ] 偽の WebPuppet を `src/testing/` に置き、配布物 (パッケージの公開の範囲) に入らない
- [ ] テストの数が分ける前と同じ (件数をメモに書いた)
- [ ] `just ci` が通る

## 参考

- T0931 のメモ (起票の候補)

## メモ

- T0019 で起票した (元の候補: T0931)。player の覚えるものを触る T0930 の後に着手する。
