---
id: T0954
title: "@ugoe/ui: Notice の中の見せずに読ませる文字が、送る枠の外を位置の基準にしないようにする"
status: in-progress
milestone: M10
crates: [packages/ui, apps/editor, apps/viewer, apps/studio, docs]
depends_on: []
assignee: claude-worker-T0954
---

# T0954: @ugoe/ui: Notice の中の見せずに読ませる文字が、送る枠の外を位置の基準にしないようにする

## 目的

`Notice` の中の見せずに読ませる文字 (`.ugoe-visually-hidden`。`position: absolute`) は、`position` を持たない送る枠の中に置くと、枠の外の祖先を位置の基準にし、
余計な送りやはみ出しを生む。T0674 でエディタのパネルでは直した (docs/ui.md の「送り」) が、ビューア・スタジオのパネルでも起きうる (T0674 の候補。バグ寄り)。

## 受け入れ条件

- [ ] `Notice` 側で直す (`position: relative` を付けるなど) か、各アプリの送る枠を直すかを決め、理由をメモに書いた
- [ ] `position` を持たない送る枠の中に `Notice` を置いたとき、枠の外に送りやはみ出しが出ないことを確かめるテスト (部品のテストか `just ui-audit` の検査) がある
- [ ] エディタで T0674 が入れた個別の打ち消しが要らなくなれば外した。3 つのアプリの `just ui-audit` の指摘が増えない
- [ ] docs/ui.md (送り) を直した
- [ ] `just ci` が通る

## 参考

- T0674 のメモ (起票の候補)、docs/ui.md (送り)
- packages/ui/src/components/Notice.svelte、packages/ui の `.ugoe-visually-hidden`

## メモ

- T0041 で起票した (元の候補: T0674 (`Notice` の見せずに読ませる文字))。
