---
id: T1678
title: "エディタ: プレビューのトラッキングの調整 (感度と境目) を覚え、起動し直しても戻す"
status: in-progress
milestone: M8
crates: [apps/editor, apps/viewer, packages/face-tracking, docs]
depends_on: []
assignee: claude-worker-T1678
---

# T1678: エディタのプレビューでトラッキングの調整を覚える

## 目的

T0738 でビューアはトラッキングの調整 (`@ugoe/face-tracking` の `TrackingPanel` の「調整 (感度と境目)」) を `localStorage` に覚えるようになった。
エディタのプレビュー (`src/lib/preview/`) にも同じパネルが出るが、覚えないので、エディタを開き直すと既定値に戻る。エディタでも調整を覚える。

## 決まっていること (利用者、2026-10-11)

- エディタのプレビューでも調整を覚える (T0738 の起票の候補「エディタのプレビューでも調整を覚えるか決める」は、覚えると決まった)。

## 作るもの (案。細部は作業者が決めて理由をメモに書く)

- ビューアの `apps/viewer/src/lib/trackingTuning.ts` (`loadTrackingTuning`・`saveTrackingTuning`。既定値ならキーを消す・`sanitizeTuning` を通して読む) を、
  キーを引数で受け取る形で `@ugoe/face-tracking` に移し (例: `loadStoredTuning(key, storage?)`・`saveStoredTuning(key, tuning, storage?)`)、
  ビューアとエディタで共有する。`localStorage` の読み書きは `@ugoe/i18n` の `readStoredItem` など (例外を出さない)。
  `@ugoe/face-tracking` が `@ugoe/i18n` に依存しない形が良ければ、Storage を引数で受け取るだけにして、アプリの側で包む。
- エディタのキーは `ugoe.editor.trackingTuning` (`STORAGE_KEYS` の形は camelCase。docs/editor.md#覚える値)。起動のとき (プレビューの tracker を作るとき) に読んで
  `tracker.setTuning` に当て、変わったら覚え直す。

## 受け入れ条件

- [ ] エディタのプレビューで調整を変えると `localStorage` に覚え、開き直すと同じ値になる。壊れた値・範囲の外は既定値に戻して読む。「既定に戻す」で
      キーを消す (ビューアと同じ) (フロントエンドのテスト)
- [ ] 読み書きの関数をビューアとエディタで共有し、ビューアの振る舞いは変わらない (ビューアの `trackingTuning.test.ts` が通る。移したならテストも移す)
- [ ] エディタの `storageKeys.ts` の `STORAGE_KEYS` に足し、`storageKeys.test.ts` が通る。docs/editor.md#覚える値 の表に行を足した
- [ ] docs/manual/preview.md の調整の表 (ビューアでは覚える、の行) を、エディタでも覚えると直した。packages/face-tracking/README.md の「保存」も
- [ ] 画面は変えない。`just e2e --only editor`・`just e2e --only viewer` が通る
- [ ] `just ci` が通る

## 参考

- T0738 のメモ (やったこと・起票の候補)、apps/viewer/src/lib/trackingTuning.ts・storageKeys.ts、apps/editor/src/lib/storageKeys.ts・preview/PreviewPanel.svelte・preview/preview.svelte.ts
- docs/editor.md#覚える値、apps/viewer/README.md#覚える値、docs/ui.md#名前と切り替え

## メモ

- T0089 で起票した (元の候補: T0738 (エディタのプレビューでもトラッキングの調整を覚えるかを決める)。利用者の決定 2026-10-11: 覚える)。
- M8 の後続 (docs/roadmap.md の M8 の「後続のタスク」)。T0738 と同じ。
- 番号: エディタの帯 (T1678〜)。
- **並行の注意:** T0739 (ビューアでトラッキングのほかの設定も覚える) は、このタスクで共有にした読み書きの関数を使うので、このタスクの後に行う (T0739 の depends_on)。
  T0881・T0882 (調整の項目を増やす) は `TrackingTuning` の形を変えるが、読むときに `sanitizeTuning` を通していれば追従は要らない (テストが形を書いていれば合わせる)。
