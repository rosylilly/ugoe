---
id: T0734
title: "ビューア: モーションのイベントの欄に、発火したモーションの名前と番号を出す"
status: done
milestone: M9
crates: [packages/player, apps/viewer, docs]
depends_on: []
assignee: claude-worker-T0734
---

# T0734: ビューア: モーションのイベントの欄に、発火したモーションの名前と番号を出す

## 目的

T0462 で、モーションのイベントを発火したモーション (名前と開始の番号) をすべての組み込み先で読めるようにした
(wasm は `WebPuppet.motion_events_json()`)。ビューアのモーションのパネル (`apps/viewer/src/lib/MotionPanel.svelte`) は、
`Player.motionEvents` (`packages/player/src/player.svelte.ts`、`motion_events()` の値だけ) を出すので、クロスフェード中にどちらのモーションの
イベントかが分からない (T0462 の候補)。

## 受け入れ条件

- [x] `Player` が `motion_events_json()` で値・時刻・モーションの名前・番号を持ち、モーションのパネルの欄に名前と番号を出す (同じ名前を始め直したものを番号で見分けられる)
- [x] `packages/player` の Vitest と、パネルの部品のテストがある
- [x] 文言は `@ugoe/i18n` のカタログ (日英) にある。docs/manual のビューアの節を直した (画像の撮り直しは T0919 のメモに書き足す)
- [x] `just ci` が通る

## 参考

- T0462 のメモ (起票の候補)
- crates/ugoe-wasm/README.md (`motion_events_json`)、packages/player/README.md

## メモ

- T0036 で起票した (元の候補: T0462)。
- やったこと:
  - packages/player: `parseMotionEvents` と型 `MotionEventInfo` (`{value, time, motion, number}`) を `src/animation.ts` に足し、
    `Player.motionEvents` を `MotionEventInfo[]` にした (`motion_events_json()` を読む。`src/wasm.ts` の口も `motion_events` から
    `motion_events_json` に替えた)。偽の WebPuppet に `state.events` を足し、Vitest で読み取り (壊れた項目を飛ばす) と、Player が
    新しい順に 5 つまで持つこと・クロスフェード中の 2 つ・同じ名前の始め直しを番号で見分けること・読み直しで消えることを確かめた。
  - apps/viewer: モーションのパネルの「イベント:」を `値 (モーション #番号)` にし、欄のツールチップで番号の意味を説明した
    (カタログ `motions.event`・`motions.eventsTooltip`、日英)。部品のテスト `src/lib/MotionPanel.svelte.test.ts` のため、
    ビューアにも `vitest.config.ts` (スタジオと同じ。happy-dom と `browser` の条件) と devDependency の happy-dom を足した (pnpm-lock.yaml も変わる)。
  - docs/manual/motions.md の「ビューアで再生する」と apps/viewer/README.md、packages/player/README.md を直した。
    T0919 のメモに画像の撮り直しを書き足した (今の motions-viewer.png にはイベントの欄が写っていない)。
- やらなかったこと: イベントの時刻 (`time`) は Player に持つが画面には出していない。実際のビューア (Tauri の webview) では見ていない
  (部品のテストと型検査で確かめた)。スタジオの操作ウィンドウはイベントを出していないので変えていない。
- 起票の候補: なし。
