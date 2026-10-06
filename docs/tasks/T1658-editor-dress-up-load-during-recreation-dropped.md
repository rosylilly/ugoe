---
id: T1658
title: "エディタ: プレビューの着せ替えで、GPU の作り直しの途中 (WebPuppet が無い間) に読み込んだモジュールが捨てられる"
status: in-progress
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

- [ ] 作り直しの途中に読み込んだモジュールも取っておき、作り直した後に読み込んで (付けたい組なら) 付けるようにした
- [ ] 部品のテスト (`dressUp.test.ts` の偽の WebPuppet) で、WebPuppet が無い間の読み込みが捨てられないことを確かめた。直す前に落ちることも確かめた
- [ ] できれば `just editor-browser` (T1656) にシナリオを足した。足さないなら理由をメモに書く
- [ ] docs/editor.md (プレビューの着せ替えの GPU の喪失) を直した
- [ ] `just e2e --only editor` と `just ci` が通る

## 参考

- T1644・T1656、apps/editor/src/lib/preview/dressUp.svelte.ts、apps/editor/browser-test/

## メモ

- T1656 のワーカーの報告から起票した (2026-10-06)。
