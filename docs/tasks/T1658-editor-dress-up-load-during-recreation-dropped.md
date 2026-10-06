---
id: T1658
title: "エディタ: プレビューの着せ替えで、GPU の作り直しの途中 (WebPuppet が無い間) に読み込んだモジュールが捨てられる"
status: done
milestone: M10
crates: [apps/editor, docs]
depends_on: []
assignee: claude-worker-T1658
---

# T1658: エディタ: プレビューの着せ替えで、GPU の作り直しの途中 (WebPuppet が無い間) に読み込んだモジュールが捨てられる

## 目的

T1644 で、読み込みの途中に GPU を失ったモジュールは `#pending` に取っておき、作り直した WebPuppet に読み込むようにした。
しかし作り直しの途中 (古い WebPuppet を捨ててから新しいものができるまで、`host.puppet()` が `null`) に読み込んだモジュールは、
`DressUp.#load` が `#pending` に入れずに捨てる (T1656 の候補)。利用者がその間にモジュールを開くと、知らせも無く何も付かない。

## 受け入れ条件

- [x] 作り直しの途中に読み込んだモジュールも取っておき、作り直した後に読み込んで (付けたい組なら) 付けるようにした
- [x] 部品のテスト (`dressUp.test.ts` の偽の WebPuppet) で、WebPuppet が無い間の読み込みが捨てられないことを確かめた。直す前に落ちることも確かめた
- [x] できれば `just editor-browser` (T1656) にシナリオを足した。足さないなら理由をメモに書く
- [x] docs/editor.md (プレビューの着せ替えの GPU の喪失) を直した
- [x] `just e2e --only editor` と `just ci` が通る

## 参考

- T1644・T1656、apps/editor/src/lib/preview/dressUp.svelte.ts、apps/editor/browser-test/

## メモ

- T1656 のワーカーの報告から起票した (2026-10-06)。
- やったこと (2026-10-06):
  - `DressUp.#load` は、ファイルを読んだ後で WebPuppet を見て、無ければ (作り直しの途中) バイト列を `#pending` に入れる
    (前は読む前に `null` を返して捨てていた)。作り直した WebPuppet で `loaded` → `#loadPending` が読み込み、「Load .ugoemod…」の
    ものは付け、ファイルから読み直していた行 (`refresh`・`reload`) は付けたい組のまま付ける。プロジェクトを開き直す (`load`) と捨てる。
  - `dressUp.test.ts` に「WebPuppet が無い間」のテストを 3 つ足した (`setup` に `drop` を足し、`host.puppet()` が `null` を返す)。
    `add` と `refresh` の 2 つは直す前に落ちることを確かめた。
  - `just editor-browser` にシナリオ `dress-up-load-during-recreation` を足した (復元した後、作成を 5000 ms 遅らせ、古い WebPuppet を
    `free` して新しいものが無い間に猫耳を読み込む)。直す前の `dressUp.svelte.ts` で落ち、直した後は 3 つのシナリオを 5 回続けて流して揺れない。
  - docs/editor.md (プレビューの着せ替えの GPU の喪失) と apps/editor/browser-test/README.md を直した。
  - `just e2e --only editor` (16 件) と `just ci` が通った。
- やらなかったこと: 作り直しの途中 (WebPuppet が無い間) の付け外し (`attach`・`detach`・`setSlot`) は、今も何もしない (付けたい組も変えない)。
  GPU を失っている間 (WebPuppet はある) は T1644 で付けたい組を変えるようになっているが、WebPuppet が無い間は `#puppet()` が `null` で
  早く戻る。このタスクの範囲 (読み込み) の外なので直していない。
- 起票の候補: 「エディタ: プレビューの着せ替えで、作り直しの途中 (WebPuppet が無い間) の付け外しが付けたい組に残らない」
  (`DressUp.attach`・`detach` を、プレビュー中で WebPuppet が無いときも `wanted` だけ変えるようにする。部品のテストと
  `just editor-browser` のシナリオ)。
