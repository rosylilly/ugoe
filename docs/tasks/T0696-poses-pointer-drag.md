---
id: T0696
title: ポーズのパネルの要素の並べ替えを、HTML5 のドラッグ & ドロップからポインタの操作にする
status: done
milestone: M9
crates: [apps/editor, docs]
depends_on: [T0671]
assignee: claude-worker-T0696
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
- [x] ツリーと同じくポインタの操作 (押して動かす・離す・取り消し) で並べ替え、キーボードの代わり (▲▼) は残した
- [x] 並べ替えの計算 (`moveEntryTo`) のテストは今のまま通る
- [x] `just ci` が通る

## 参考

- docs/tasks/T0671-review-editor-panels.md (所見の表)
- apps/editor/src/lib/panels/TreePanel.svelte (ポインタでのドラッグ)
- https://v2.tauri.app/reference/config/#dragdropenabled

## メモ

- **やったこと:** `PosesPanel.svelte` の並べ替えを、HTML5 の DnD (`draggable`・`ondragstart`・`ondrop`) からポインタの
  イベントにした (ツリーと同じ形)。要素の行 (`.entry`。Show・▲▼・✕ のボタンの上は除く) を押して 5px 動かすとドラッグになり、
  リスト (`<ol>`) がポインタを取り込む。落とす先は同じグループの要素の上だけ (上半分なら前、下半分なら後ろ)。離したときに
  `moveEntryTo` で 1 回だけ `set_pose` を送る。`pointercancel`・取り込みを失う・Esc では何も送らずに元へ戻す (T0672 の形)。
  ドラッグの後のクリック (名前を押してパーツを選ぶ) は捨てる。動かす前にリストの外で離したとき (pointerup が届かない) は、
  次の pointermove でボタンが離れているのを見て取り消す。
- 落とす先の計算を `poses.ts` の `dropIndex` に出してテストを足した。`moveEntryTo` のテストは変えずに通る。
- キーボードの代わりの ▲▼ はそのまま。案内 (`poses.entry.dragHint`) を「ドラッグか ▲▼ で並べ替え。Esc で取り消し」にした (en・ja)。
  ドラッグの取っ手の `div` は `a11y_no_static_element_interactions` を理由付きで抑えた (キーボードは ▲▼)。
- docs/editor.md (ポーズの編集) と docs/ui.md (文字の選択の節の `draggable` の記述) を直した。
- **確かめていないこと:** 1 つ目の受け入れ条件 (Windows で HTML5 の DnD が効かないこと) は Windows の実機が無いので確かめていない。
  Tauri の `dragDropEnabled` の説明 (https://v2.tauri.app/reference/config/#dragdropenabled) とツリーの実装の理由に従って直した。
  ポインタでのドラッグは型検査と単体テストまでで、実際のエディタ (Linux・Windows) で手で動かしてはいない。
- **起票の候補:** なし (▼ のツールチップが無いのは T0674 で扱う)。
