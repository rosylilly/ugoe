---
id: T0696
title: ポーズのパネルの要素の並べ替えを、HTML5 のドラッグ & ドロップからポインタの操作にする
status: todo
milestone: M9
crates: [apps/editor, docs]
depends_on: [T0671]
assignee:
---

# T0696: ポーズのパネルの要素の並べ替えを、HTML5 のドラッグ & ドロップからポインタの操作にする

## 目的

T0671 (パネルのコードレビュー) の所見。ポーズのパネル (`apps/editor/src/lib/panels/poses/PosesPanel.svelte`) は、グループの要素の並べ替えに
HTML5 のドラッグ & ドロップ (`draggable`・`ondragstart`・`ondrop`) を使う。エディタは Tauri のファイルのドロップ (`onDragDropEvent`。
着せ替えのプレビュー) を有効にしているので、Windows (WebView2) では HTML5 のドラッグ & ドロップがウィンドウに届かない
(Tauri の `dragDropEnabled` の説明)。ツリー (`TreePanel.svelte`) はこのためにポインタの操作で実装している。
▲▼ のボタンでは並べ替えられるが、ドラッグの案内 (`poses.entry.dragHint`) が出るのに効かない。

## 受け入れ条件

- [ ] Windows で HTML5 のドラッグ & ドロップが効かないことを確かめた (効くなら、その結果をメモに書いて閉じる)
- [ ] ツリーと同じくポインタの操作 (押して動かす・離す・取り消し) で並べ替え、キーボードの代わり (▲▼) は残した
- [ ] 並べ替えの計算 (`moveEntryTo`) のテストは今のまま通る
- [ ] `just ci` が通る

## 参考

- docs/tasks/T0671-review-editor-panels.md (所見の表)
- apps/editor/src/lib/panels/TreePanel.svelte (ポインタでのドラッグ)
- https://v2.tauri.app/reference/config/#dragdropenabled

## メモ
