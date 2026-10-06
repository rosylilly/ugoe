---
id: T1640
title: "エディタ: プレビューのポーズの欄を @ugoe/player の PosePicker に移す"
status: todo
milestone: M9
crates: [apps/editor, packages/player, docs]
depends_on: [T0676]
assignee:
---

# T1640: エディタ: プレビューのポーズの欄を @ugoe/player の PosePicker に移す

## 目的

T0948 で、ビューアとスタジオのポーズの欄を `@ugoe/player` の `PosePicker` にまとめた。エディタのプレビューのポーズの欄は、見出しと要素の表示名が違うので
自前のまま残った (T0948 の候補)。

## 受け入れ条件

- [ ] `PosePicker` に見出しと要素の表示名を差し替える props があり (ビューア・スタジオの表示は変わらない)、部品のテストがある
- [ ] エディタのプレビューのポーズの欄が `PosePicker` を使う。要素が 2〜4 個のグループは SegmentedControl になるので、プレビューのテスト (`posePanel.test.ts`) と
      エディタの e2e を直した。T0736 のモーションへの追従のテストが通る
- [ ] packages/player/README.md と docs/editor.md を直し、画面が変わったところを T0919 のメモに書き足した
- [ ] `just e2e --only editor` と `just ci` が通る

## 参考

- T0948 のメモ (やったこと・起票の候補)、T0736 (欄の追従)
- packages/player/src/PosePicker.svelte、apps/editor/src (プレビューのポーズの欄)

## メモ

- T0041 で起票した (元の候補: T0948 (エディタのプレビューのポーズの欄))。T0676 (作業中) がモジュール・着せ替えのパネルを触るので、その後に着手する。
