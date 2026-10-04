---
id: T0931
title: "@ugoe/player: Player の振る舞いのテストを足す"
status: in-progress
milestone: M9
crates: [packages, docs]
depends_on: [T0914]
assignee: claude-worker-T0931
---

# T0931: @ugoe/player: Player の振る舞いのテストを足す

## 目的

T0914 のレビューで見つけた。`src/player.svelte.ts` (約 1300 行) は、T0914 で足した作成と破棄 (attach・detach・GPU の
作り直し・devicePixelRatio) のテストのほかにテストが無い。着せ替えの組を覚えて戻す流れ、モーションの繰り返しと待機
モーション、トラッキングとドライバの `driven`、受け手のモード (フレームの適用・描画の負荷の知らせ・シーンの切り替え)、
光源のドラッグの途中と確定は、部品の関数 (dressUpMemory.ts など) だけが確かめられている。

## 受け入れ条件

- [ ] `src/player.test.ts` の偽の WebPuppet (Proxy) を使い、上の流れを Player を通して確かめるテストがある
- [ ] モデルを開き直したとき・別のモデルに替えたとき・開けなかったときの着せ替えの組と `restoreOffer` を確かめる
- [ ] 受け手のモードで、モデルを読む前に届いたフレームが読み終えた後に順に適用されることを確かめる
- [ ] 見つかったバグは直すか起票する
- [ ] `just ci` が通る

## 参考

- packages/player/README.md
- packages/player/src/player.test.ts (T0914)

## メモ
