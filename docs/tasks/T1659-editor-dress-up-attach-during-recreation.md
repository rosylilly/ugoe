---
id: T1659
title: "エディタ: プレビューの着せ替えで、GPU の作り直しの途中 (WebPuppet が無い間) の付け外し・スロットの選択が付けたい組に残らない"
status: done
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

- [x] プレビュー中で WebPuppet が無い間も、付け外し・スロットの選択で `wanted` だけを変え、作り直した後にその組で付けるようにした
- [x] 部品のテスト (`dressUp.test.ts`) で確かめた。直す前に落ちることも確かめた
- [x] `just editor-browser` にシナリオを足した (作り直しの作成を遅らせ、その間に付け外し・差し替え)
- [x] docs/editor.md を直した
- [x] `just e2e --only editor`・`just editor-browser`・`just ci` が通る

## 参考

- T1644・T1656・T1658、apps/editor/src/lib/preview/dressUp.svelte.ts

## メモ

- T1658 のワーカーの報告から起票した (2026-10-06)。
- やったこと (2026-10-06):
  - `DressUp.attach`・`detach` は、プレビュー中で WebPuppet が無い (作り直しの途中) ときも、GPU を失っているときと同じく
    付けたい組 (`wanted`) だけを変える (同じスロットのものとは差し替える。共通の `#want`)。プロジェクトにも残す。作り直した WebPuppet で
    `loaded` → `#reconcile` がその組で付ける。`setSlot(slot, null)` は、付いているものだけでなく付けたい組の行も外す
    (WebPuppet が無いか失っている間は両者が食い違うため。T1644 の失っている間にも効く)。`detach` に `save` を足し、`setSlot` は 1 回だけ残す。
  - 同じ種類の取りこぼし: 作り直しの途中に読み直していた行 (`refresh`・`reload` の `#pending`) を一覧から除く (`remove`) と、作り直した
    WebPuppet に一覧に無いモジュールとして読み込まれていた。`remove` で `#pending` からも除くようにした。
  - `dressUp.test.ts` にテストを 3 つ足した (WebPuppet が無い間のスロットの選択での外す・差し替え、チェックで外す、除いた行を読み込まない)。
    3 つとも直す前に落ちることを確かめた。
  - `just editor-browser` にシナリオ `dress-up-change-during-recreation` を足した (`dress-up-change-while-lost` と同じ付け外しを、作成を
    5000 ms 遅らせた作り直しの途中に行う。共通の手順は関数に分けた)。直す前の `dressUp.svelte.ts` で落ち (猫耳とツインテールが付いたまま)、
    直した後は 4 つのシナリオを 5 回続けて流して揺れない。
  - docs/editor.md (プレビューの着せ替えの GPU の喪失) と apps/editor/browser-test/README.md を直した。
  - `just e2e --only editor`・`just ci` が通った。
- やらなかったこと: 一覧のチェックとスロットの選択は今付いているか (`attached`) を写すので、WebPuppet が無いか失っている間に付け外しても、
  作り直すまで表示は前のまま (付けたい組は変わっている)。T1644 の失っている間も同じ。
- 起票の候補: 「エディタ: プレビューの着せ替えで、GPU を失っている間・作り直しの途中の付け外しが、作り直すまで一覧のチェックとスロットの選択に
  出ない」(その間は `wanted` を写すか、待っていることを行に出す)。
