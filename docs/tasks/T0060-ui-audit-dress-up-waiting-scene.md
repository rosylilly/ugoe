---
id: T0060
title: "ui-audit: エディタに「着せ替えで作り直しを待っている」場面 (待っている文の出る行) を足す"
status: todo
milestone: M10
crates: [tests, docs]
depends_on: [T0055]
assignee:
---

# T0060: ui-audit: エディタに「着せ替えで作り直しを待っている」場面 (待っている文の出る行) を足す

## 目的

T1666 で、プレビューの着せ替えは GPU を失っている間・WebPuppet の作り直しの途中に付け外しをすると、行のチェックが `wanted` を写し、
まだ届いていない行に「Puts this on / Takes this off when the preview is back.」の文が出るようになった。この行は `just ui-audit` のどの場面にも
写っておらず、コントラスト・はみ出し・押せる大きさを確かめていない。

## 受け入れ条件

- [ ] `tests/ui-audit/lib/apps.mjs` のエディタに、着せ替えで作り直しを待っている行 (待っている文の出る行) を写す場面を足した
      (偽のホストで GPU を失った状態を作るなど、作り方をコメントに書く)。tests/ui-audit/README.md の場面の一覧に書いた
- [ ] 場面に待っている文が写っていることを確かめた (撮った画像か、場面の中の検査で)
- [ ] `just ui-audit --only editor` を流し、足した場面の指摘を直したか、意図したもの (理由) としてメモに書いた
- [ ] `just ui-audit-check` と `just ci` が通る

## 参考

- T1666 のメモ (決めたこと・起票の候補)、apps/editor/src/lib/preview/dressUp.svelte.ts (`waiting`・`shown`・`pendingChange`)・DressUpSection.svelte
- tests/ui-audit/lib/apps.mjs、tests/ui-audit/README.md

## メモ

- T0058 で起票した (元の候補: T1666 (ui-audit に作り直しを待っている場面を足す))。
- 同じ tests/ui-audit を触る T0055 の後にした。
