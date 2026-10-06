---
id: T1659
title: "エディタ: プレビューの着せ替えで、GPU の作り直しの途中 (WebPuppet が無い間) の付け外し・スロットの選択が付けたい組に残らない"
status: in-progress
milestone: M10
crates: [apps/editor, docs]
depends_on: []
assignee: claude-worker-T1659
---

# T1659: エディタ: プレビューの着せ替えで、GPU の作り直しの途中 (WebPuppet が無い間) の付け外し・スロットの選択が付けたい組に残らない

## 目的

T1658 で作り直しの途中の読み込みは取っておくようにしたが、同じ間の付け外し (`attach`・`detach`・`setSlot`) は今も何もせず、
付けたい組 (`wanted`) も変わらない (T1658 の候補)。失っている間 (WebPuppet があり `is_lost()`) の付け外しは T1644 で `wanted` を変えるので、
WebPuppet が無い間だけ利用者の操作が黙って消える。

## 受け入れ条件

- [ ] プレビュー中で WebPuppet が無い間も、付け外し・スロットの選択で `wanted` だけを変え、作り直した後にその組で付けるようにした
- [ ] 部品のテスト (`dressUp.test.ts`) で確かめた。直す前に落ちることも確かめた
- [ ] `just editor-browser` にシナリオを足した (作り直しの作成を遅らせ、その間に付け外し・差し替え)
- [ ] docs/editor.md を直した
- [ ] `just e2e --only editor`・`just editor-browser`・`just ci` が通る

## 参考

- T1644・T1656・T1658、apps/editor/src/lib/preview/dressUp.svelte.ts

## メモ

- T1658 のワーカーの報告から起票した (2026-10-06)。
