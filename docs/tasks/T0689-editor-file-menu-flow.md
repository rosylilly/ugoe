---
id: T0689
title: エディタのファイルメニューの非同期の操作の排他と、流れのテスト
status: todo
milestone: M9
crates: [apps/editor, docs]
depends_on: [T0670]
assignee:
---

# T0689: エディタのファイルメニューの非同期の操作の排他と、流れのテスト

## 目的

T0670 のレビューで見つけた、`apps/editor/src/lib/FileMenu.svelte` (約 840 行) の非同期の操作の重なりを整理し、
流れをテストできる形にする。いまは流れのロジックがすべて部品の中にあり、テストが無い。

- `save`・`overwriteExternal`・`reloadProject` は `busy` を立てないので、「Save」を続けて押すと保存が 2 回走り、
  保存の途中に外での書き換えの確認 (`checkExternal`) や他の操作が始まりうる。
- `checkExternal` はウィンドウのフォーカスと一定の間隔の両方から呼ばれ、重なると同じ変化に 2 回応じうる
  (自動で開き直す設定なら `reloadProject` が 2 回)。
- モジュールの設定の適用 (`applyModuleSettings`) は `ipc.applyCommand` を直接呼び、適用の報告 (`ApplyReport.warnings`) を捨てる。
  ほかの編集と同じく `EditorDocument.applyWith` を通す。
- 書き出しの設定の欄 (`ExportPanel.svelte` の `update`) は、続けて変えると前の変更が終わる前の古い設定を元に送り、前の変更を戻してしまう。
- 開く・本体の付け替え・外での書き換えの確認の分岐 (`run`・`perform`・`reopenModule`・`relocateBase`・`reloadBase`) に
  テストが無い。

## 受け入れ条件

- [ ] 流れ (`run`・`perform`・保存と外での書き換え・本体の変化) を `.svelte.ts` のクラスか純粋な関数に移し、部品は表示と呼び出しだけにする
- [ ] 保存・上書き・開き直し・外での書き換えの確認が重ならない (実行中の操作があれば、後から来たものは待つか捨てる)
- [ ] モジュールの設定の適用を `EditorDocument.applyWith` に通し、書き出しの設定の変更を重ねない
- [ ] 上の流れを IPC を偽物にした Vitest で確かめる (保存の連打・フォーカスと間隔の確認の重なり・本体の問題で開けないとき)
- [ ] 画面の見た目と配置は変えない (M10 の範囲)
- [ ] `just ci` が通る

## 参考

- docs/editor.md#外での書き換え・#本体の変化・#本体の問題の直し方
- T0670 のメモ

## メモ
