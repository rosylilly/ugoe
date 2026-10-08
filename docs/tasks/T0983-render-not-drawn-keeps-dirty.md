---
id: T0983
title: "描かずに返った render (サーフェスの作り直し・時間切れ) の後も、描き直しの印を残して次のフレームで描く (Player・<ugoe-avatar>・エディタ)"
status: in-progress
milestone: M8
crates: [ugoe-wasm, packages/player, bindings/web, apps/editor, docs]
depends_on: []
assignee: claude-worker-T0983
---

# T0983: 描かずに返った render の後も、描き直しの印を残して次のフレームで描く

## 目的

T0857・T1402・T0982 で、Player (スタジオの受け手・ビューア)・`<ugoe-avatar>`・エディタのプレビューは、変化があったときだけ `render()` を呼ぶようにした。
しかし wasm の `render` は、サーフェスを作り直したときや時間切れのときに描かずに返ることがある。そのフレームで描き直しの印 (`#dirty` など) を下ろしてしまうと、
形も見せ方も変わらない限り次のフレームでも描かないので、古い画 (や空の画) が残る (T0982 の候補。バグ寄り)。

## 受け入れ条件

- [ ] 原因の経路 (wasm の `render` が描かずに返る場合の一覧) を確かめ、メモに書いた
- [ ] `render` が描けたかを呼び出し側が知れる形にした (例: 描けたかを返す。wasm の口を変えるなら T0492 の方針に従い、型の写し (`just wasm-types`) とそろえる)。描けなかったときは描き直しの印を残し、次のフレームで描く
- [ ] Player・`<ugoe-avatar>`・エディタのビューポートのすべてで直した
- [ ] 部品のテスト (偽の wasm で、描かずに返った後の次のフレームで描く) を足した。直す前に落ちることを確かめた
- [ ] `just player-browser`・`just editor-browser`・`just conformance` (avatar)・`just e2e --only editor,viewer,studio`・`just ci` が通る

## 参考

- T0857・T1402・T0982 のメモ、crates/ugoe-wasm (render)、packages/player/src/player.svelte.ts・surface.ts、bindings/web/src/avatar.ts、apps/editor の ViewportController

## メモ

- T0982 のワーカーの報告から起票した (2026-10-08)。
