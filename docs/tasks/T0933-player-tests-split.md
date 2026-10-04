---
id: T0933
title: "@ugoe/player: player.test.ts を流れごとに分け、偽の WebPuppet をテスト用の部品にする"
status: done
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

- [x] テストを流れごとのファイルに分け、どれも 500 行程度以下にした
- [x] 偽の WebPuppet を `src/testing/` に置き、配布物 (パッケージの公開の範囲) に入らない
- [x] テストの数が分ける前と同じ (件数をメモに書いた)
- [x] `just ci` が通る

## 参考

- T0931 のメモ (起票の候補)

## メモ

- T0019 で起票した (元の候補: T0931)。player の覚えるものを触る T0930 の後に着手する。

### やったこと (T0933)

- `src/testing/` にテスト用の部品を置いた: `fakePuppet.ts` (偽の WebPuppet・FakeCanvas・偽のバンドルとモジュール)、
  `player.ts` (偽の wasm と requestAnimationFrame・document で Player を動かす準備 `setupPlayer()` と `ready`・`step` など)、
  `faceTracking.ts` (`@ugoe/face-tracking` の代わり。各テストで `vi.mock(..., () => import("./testing/faceTracking"))`)、
  `memoryStorage.ts` (localStorage の代わり)。`src/index.ts` からは出さず、パッケージの `exports` は `./src/index.ts` だけ
  (private) なので配布物の公開の範囲に入らない。README のファイル一覧に足した。
- `player.test.ts` (1039 行) を流れごとに分けた: `player.test.ts` (作成と破棄、139 行)・`playerDressUp.test.ts` (209)・
  `playerMotion.test.ts` (93)・`playerTracking.test.ts` (72)・`playerFollower.test.ts` (192)・`playerLighting.test.ts` (102)。
  テストの中身は変えていない。
- T0930 の候補「重複している MemoryStorage を共有のテスト用の部品に」も packages/player の中で行った
  (`modelMemory.test.ts`・`lighting.test.ts`・`dressUpMemory.test.ts`・Player のテスト)。
- テストの数: 分ける前も後も packages/player 全体で 159 件 (Player のものは 31 件)。名前の一覧も前後で一致することを確かめた。
- `just ci` が通った。

### やらなかったこと

- `storage.test.ts` の `memoryStorage()` (中の Map を見るもの) と `rememberedModels.test.ts` の最小の Storage は、
  形が違う (中身を直接見る・一部のメソッドだけ) ので置き換えていない。
