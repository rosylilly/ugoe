---
id: T1679
title: "エディタのプレビュー: トラッキングのほかの設定 (平滑化・左右の入れ替え・目パチを止める・体) も覚える"
status: done
milestone: M8
crates: [apps/editor, packages/face-tracking, docs]
depends_on: []
assignee: claude-worker-T1679
---

# T1679: エディタのプレビューでトラッキングの設定を覚える

## 目的

T0739 でビューアは、トラッキングの平滑化・左右の入れ替え・目パチを止める・体を `ugoe.viewer.tracking.settings` に覚えるようになった
(共有の関数は @ugoe/face-tracking の `settingsStorage.ts`)。T1678 でエディタのプレビューは調整だけを覚える。エディタでもほかの設定を覚え、開き直したときに同じ設定から始める。

## 受け入れ条件

- [x] エディタの `STORAGE_KEYS` にキーを足し、プレビューを作るときに覚えた設定を当て、変わるたびに覚え直す (T1678 の調整と同じ形)。壊れた値は既定値
- [x] テスト (覚えていないとき既定値・覚えて読み直す・既定に戻すとキーを消す・壊れた値)
- [x] docs/editor.md (覚える値の表) と docs/manual/preview.md を直した
- [x] `just ui-audit --only editor` と エディタの e2e が通る
- [x] `just ci` が通る

## 参考

- T0739・T1678 のメモ、packages/face-tracking/src/settingsStorage.ts・tuningStorage.ts、apps/editor の preview と App.svelte

## メモ

- 元の候補: T0739 (エディタのプレビューでも、ほかのトラッキングの設定を覚える)。オーケストレーターが起票した。
- **やったこと (T1679):**
  - エディタの `STORAGE_KEYS.trackingSettings` (`ugoe.editor.trackingSettings`) を足した。`PreviewMode` は作るときに `loadStoredSettings` で読んで
    `applyTrackerSettings` で当て (調整と同じく 3 つ目の引数の `storage` を使う)、`saveSettings()` を App.svelte の `$effect` から呼んで覚え直す
    (T1678 の `saveTuning` と同じ形。調整とは別の `$effect`・別のキー)。読み書きは T0739 の `@ugoe/face-tracking` の `settingsStorage.ts` をそのまま使う。
  - テスト (`apps/editor/src/lib/preview/tracking.test.ts`): 覚えていないとき既定値・覚えて読み直す・調整とキーが別・既定に戻すとキーを消す・壊れた値と範囲の外。
  - 文書: docs/editor.md (プレビューモードのフェイストラッキングと覚える値の表)、docs/manual/preview.md (エディタも覚える、に直した)、
    packages/face-tracking/README.md と `settingsStorage.ts` の冒頭のコメント (エディタのキー)。
  - 確かめたこと: `just ui-audit --only editor` (すべて 0)、`just e2e --only editor` (1 回目は見本を開く場面が 10 秒で時間切れ。ほかのワーカーと並行した
    負荷 (load average 約 30) によるもので、流し直すと 16 件すべて通った)、`just ci`。
- **やらなかったこと:** 画面は変えていない。カメラの選択・鏡像・ランドマークの表示は覚えない (ビューアと同じ)。
- **起票の候補:** なし。
