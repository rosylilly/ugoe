---
id: T0734
title: "ビューア: モーションのイベントの欄に、発火したモーションの名前と番号を出す"
status: todo
milestone: M9
crates: [packages/player, apps/viewer, docs]
depends_on: []
assignee:
---

# T0734: ビューア: モーションのイベントの欄に、発火したモーションの名前と番号を出す

## 目的

T0462 で、モーションのイベントを発火したモーション (名前と開始の番号) をすべての組み込み先で読めるようにした
(wasm は `WebPuppet.motion_events_json()`)。ビューアのモーションのパネル (`apps/viewer/src/lib/MotionPanel.svelte`) は、
`Player.motionEvents` (`packages/player/src/player.svelte.ts`、`motion_events()` の値だけ) を出すので、クロスフェード中にどちらのモーションの
イベントかが分からない (T0462 の候補)。

## 受け入れ条件

- [ ] `Player` が `motion_events_json()` で値・時刻・モーションの名前・番号を持ち、モーションのパネルの欄に名前と番号を出す (同じ名前を始め直したものを番号で見分けられる)
- [ ] `packages/player` の Vitest と、パネルの部品のテストがある
- [ ] 文言は `@ugoe/i18n` のカタログ (日英) にある。docs/manual のビューアの節を直した (画像の撮り直しは T0919 のメモに書き足す)
- [ ] `just ci` が通る

## 参考

- T0462 のメモ (起票の候補)
- crates/ugoe-wasm/README.md (`motion_events_json`)、packages/player/README.md

## メモ

- T0036 で起票した (元の候補: T0462)。
