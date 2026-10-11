---
id: T1679
title: "エディタのプレビュー: トラッキングのほかの設定 (平滑化・左右の入れ替え・目パチを止める・体) も覚える"
status: todo
milestone: M8
crates: [apps/editor, packages/face-tracking, docs]
depends_on: []
assignee:
---

# T1679: エディタのプレビューでトラッキングの設定を覚える

## 目的

T0739 でビューアは、トラッキングの平滑化・左右の入れ替え・目パチを止める・体を `ugoe.viewer.tracking.settings` に覚えるようになった
(共有の関数は @ugoe/face-tracking の `settingsStorage.ts`)。T1678 でエディタのプレビューは調整だけを覚える。エディタでもほかの設定を覚え、開き直したときに同じ設定から始める。

## 受け入れ条件

- [ ] エディタの `STORAGE_KEYS` にキーを足し、プレビューを作るときに覚えた設定を当て、変わるたびに覚え直す (T1678 の調整と同じ形)。壊れた値は既定値
- [ ] テスト (覚えていないとき既定値・覚えて読み直す・既定に戻すとキーを消す・壊れた値)
- [ ] docs/editor.md (覚える値の表) と docs/manual/preview.md を直した
- [ ] `just ui-audit --only editor` と エディタの e2e が通る
- [ ] `just ci` が通る

## 参考

- T0739・T1678 のメモ、packages/face-tracking/src/settingsStorage.ts・tuningStorage.ts、apps/editor の preview と App.svelte

## メモ

- 元の候補: T0739 (エディタのプレビューでも、ほかのトラッキングの設定を覚える)。オーケストレーターが起票した。
