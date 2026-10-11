---
id: T0739
title: "ビューア: トラッキングのほかの設定 (平滑化・左右の入れ替え・目パチを止める・体) も覚え、起動し直しても戻す"
status: done
milestone: M8
crates: [apps/viewer, packages/face-tracking, docs]
depends_on: [T1678]
assignee: claude-worker-T0739
---

# T0739: ビューアでトラッキングのほかの設定も覚える

## 目的

T0738 でビューアはトラッキングの調整 (感度と境目) だけを `localStorage` (`ugoe.viewer.tracking.tuning`) に覚えるようにした。
平滑化・左右の入れ替え・目パチを止める・体 (`trackBody`) は覚えないので、開き直すたびに既定値に戻る。これらも同じキーの分野 (`tracking.*`) に覚える。

## 決まっていること (利用者、2026-10-11)

- ほかのトラッキングの設定も今覚える (T0738 の起票の候補「要望が出たら」は、今作ると決まった)。

## 作るもの (案。細部は作業者が決めて理由をメモに書く)

- 覚えるもの: 平滑化 (`smoothing`)・左右の入れ替え (`swapSides`)・目パチを止める (`stopEyeBlink`)・体 (`trackBody`)。
  キーを 1 つにまとめる (`ugoe.viewer.tracking.settings` に JSON) か、項目ごとに分けるかを決める。読むときは型と範囲を確かめ、壊れた値は項目ごとに既定値。
- 覚えないもの: カメラ (機器の ID は機械や接続で変わる。覚えるなら、無い機器のときに既定のカメラに戻すことまで作る)・ランドマークの表示・
  鏡の表示。覚えるかを決めて、覚えないものは理由をメモに書く。
- 戻し方: 調整の「既定に戻す」は調整だけを戻す。ほかの設定は各スイッチ・スライダーで戻せるので、既定値に戻したらキーを消す (調整と同じ考え) だけで足りるかを確かめる。
- T1678 で共有にした読み書きの関数 (`@ugoe/face-tracking`) があれば、同じ形で足す。エディタでも同じ設定を覚えるかは、このタスクでは扱わない (メモに書く)。

## 受け入れ条件

- [x] ビューアで上の設定を変えると `localStorage` に覚え、開き直すと同じ値になる。壊れた値・範囲の外・使えない localStorage でも起動できる (フロントエンドのテスト)
- [x] `storageKeys.ts` の `STORAGE_KEYS` に足し、`storageKeys.test.ts` の形と重なりの検査が通る。apps/viewer/README.md#覚える値 の表に行を足した
- [x] docs/manual/preview.md (ビューアのトラッキングの説明) に、覚える設定を書いた
- [x] 画面を変えたら `just ui-audit --only viewer` と `just e2e --only viewer` が通る (変えなければ、その旨をメモに書く)
- [x] `just ci` が通る

## 参考

- T0738 のメモ、apps/viewer/src/App.svelte・src/lib/trackingTuning.ts・src/lib/storageKeys.ts、packages/face-tracking/src/tracker.svelte.ts (`smoothing`・`swapSides`・
  `setStopEyeBlink`・`setTrackBody`)、apps/viewer/README.md#覚える値、docs/ui.md#名前と切り替え

## メモ

- T0089 で起票した (元の候補: T0738 (ビューアで、トラッキングのほかの設定 (平滑化・左右の入れ替え・目パチを止める・体) も `tracking.*` のキーに覚える)。
  利用者の決定 2026-10-11: 今作る)。
- M8 の後続 (docs/roadmap.md の M8 の「後続のタスク」)。
- **並行の注意:** T1678 (エディタのプレビューで調整を覚える) が読み書きの関数を `@ugoe/face-tracking` に移し、ビューアの `trackingTuning.ts` を書き換えるので、その後に行う。

### 作業のメモ (T0739)

- **やったこと:** `@ugoe/face-tracking` に `src/settingsStorage.ts` を足した (調整の `tuningStorage.ts` と同じ形)。
  `TrackingSettings` (`smoothing`・`swapSides`・`stopEyeBlink`・`trackBody`)・`DEFAULT_TRACKING_SETTINGS`・`sanitizeTrackingSettings`・
  `loadStoredSettings(key, storage?)`・`saveStoredSettings(key, settings, storage?)`・`FaceTracker` との受け渡しの `trackerSettings`・`applyTrackerSettings`。
  ビューアは `ugoe.viewer.tracking.settings` (`STORAGE_KEYS["tracking.settings"]`) に覚え、起動のときに当てる (`App.svelte`)。
- **キーは 1 つにまとめた (JSON):** 4 つはいつも一緒に読み書きし、調整 (`tracking.tuning`) と同じ形になり、`tracking.*` のキーを増やさないため。
  壊れた値は項目ごとに既定値 (型の違う値は既定値、平滑化は 0〜1 に収め、NaN・無限大は既定値)。JSON でない・オブジェクトでない値は全体を既定値。
  すべて既定値になったらキーを消す。
- **戻し方:** 調整の「既定に戻す」は調整だけを戻す。ほかの設定は各スイッチとスライダーで既定の値に戻せば、キーが消える (調整と同じ考え) ので、別の戻し方は足していない。
- **覚えないもの:** カメラ (機器の ID は機械や接続で変わり、無い機器のときに既定に戻す処理まで要る)・ランドマークと鏡像の表示 (表示だけで、その場で切り替えるもの)。
- **`tracker.svelte.ts` は変えていない** (T0882 との衝突を避けるため)。`FaceTracker` の初めの値と `DEFAULT_TRACKING_SETTINGS` の一致は `settingsStorage.test.ts` で検査する。
- **画面は変えていない。** 念のため `just ui-audit --only viewer` (指摘 0) と `just e2e --only viewer` (6 件通過) を流した。`just ci` は通った。
- **エディタのプレビュー** でほかの設定を覚えるかは扱っていない (エディタは調整だけを覚える。manual/preview.md にそう書いた)。
- **起票の候補:** エディタのプレビューでも、平滑化・左右の入れ替え・目パチを止める・体を `ugoe.editor.*` に覚える (`settingsStorage.ts` を使えば App 側の数行で済む)。
