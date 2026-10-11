---
id: T0738
title: "ビューア: トラッキングの調整 (感度と境目) を覚え、起動し直しても戻す"
status: todo
milestone: M8
crates: [apps/viewer, packages/face-tracking, tests/ui-audit, docs]
depends_on: []
assignee:
---

# T0738: ビューアでトラッキングの調整を覚える

## 目的

T0877 で、トラッキングのパネル (`@ugoe/face-tracking` の `TrackingPanel`) に「調整 (感度と境目)」を足した。スタジオはプロファイルの `input.tuning` に保存するが、
同じパネルを使うビューアでは画面は出るのに覚えないので、起動し直す (ビューアを開き直す) と既定値に戻る。ビューアでも調整を覚える。

今のビューアは、トラッキングの設定 (カメラ・平滑化・左右の入れ替え・目パチを止める・体) を何も覚えていない (`localStorage` のキーは
apps/viewer/README.md#覚える値 の表のとおりで、トラッキングのものは無い)。

## 作るもの (案。細部は作業者が決めて理由をメモに書く)

- `localStorage` のキー `ugoe.viewer.tracking.tuning` (名前の形は docs/ui.md#名前と切り替え) に調整を JSON で覚え、起動のときに読んで `tracker.setTuning` に当てる。
  読むときは `sanitizeTuning` を通す (壊れた値・知らない項目は既定値)。読み書きは `@ugoe/i18n` の `readStoredItem` などを通す (T0937)。
- ほかのトラッキングの設定 (平滑化・左右の入れ替えなど) も一緒に覚えるかを決める。推奨は、このタスクでは調整だけにして、ほかの設定は
  要望が出たら同じキーの分野 (`tracking.*`) に足す (今まで覚えずに困った報告が無く、覚える項目を一度に増やすと「戻す」の画面の扱いも要るため)。
- エディタのプレビューでも同じパネルが出るが、このタスクでは扱わない (エディタで覚えるかは、プレビューの設定をどう覚えているかに合わせて別に決める)。

## 受け入れ条件

- [ ] ビューアで調整を変えると `localStorage` に覚え、開き直すと同じ値になる。壊れた値・範囲の外は既定値に戻して読む (フロントエンドのテスト)
- [ ] 「既定に戻す」で覚えた値も既定値になる (キーを消すか既定値を書く。どちらかを決めて書く)
- [ ] `storageKeys.ts` の `STORAGE_KEYS` に足し、`storageKeys.test.ts` の形と重なりの検査が通る。apps/viewer/README.md#覚える値 の表に行を足した
- [ ] docs/manual (ビューアのトラッキングの節) に、調整を覚えることを書いた
- [ ] `just ui-audit --only viewer` が通る (画面が変わらなければ、その旨をメモに書く)
- [ ] `just ci` が通る

## 参考

- T0877 のメモ (決めたこと・やらなかったこと・起票の候補)、packages/face-tracking/src/tuning.ts (`sanitizeTuning`・`DEFAULT_TUNING`)・tracker.svelte.ts (`setTuning`)
- apps/viewer/src/App.svelte・src/lib/storageKeys.ts、apps/viewer/README.md#覚える値、docs/ui.md#名前と切り替え

## メモ

- T0083 で起票した (元の候補: T0877 (ビューアでもトラッキングの調整を保存する。平滑化などほかのトラッキングの設定をビューアがどう覚えているかに合わせる))。
- M8 の後続 (docs/roadmap.md の M8 の「後続のタスク」)。M8 の完了の条件には含めない。
- T0879・T0880 (調整の項目を増やす・首を非対称にする) で `TrackingTuning` の形が変わっても、読むときに `sanitizeTuning` を通していれば知らない項目と足りない項目は既定値になる。
