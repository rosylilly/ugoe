---
id: T1634
title: "エディタ: キーフォームの軸のボタンとパラメータの下のキーの印を 24px の押せる範囲にする"
status: in-progress
milestone: M10
crates: [apps/editor, docs]
depends_on: [T0675]
assignee: claude-worker-T1634
---

# T1634: エディタ: キーフォームの軸のボタンとパラメータの下のキーの印を 24px の押せる範囲にする

## 目的

T0675 で足した `just ui-audit` の場面 `-gizmo` (デフォーマを選んだ画面) で、インスペクタのキーフォームの軸の「キーへ移る」「キーを消す」
(`button.go`・`button.remove`。高さ 16px) と、パラメータのスライダーの下のキーの印 (`button.marker`。8px) が 24px 未満として数えられた (T0675 の候補)。
パネルの中なので T0675 では直していない。

## 受け入れ条件

- [ ] キーフォームの軸の 2 つのボタンとキーの印の押せる範囲が 24px 以上 (見た目の大きさは変えなくてよい。隣の印と範囲が重なるときの選び方をメモに書く)
- [ ] `just ui-audit --only editor` の `-gizmo` の場面で、この 3 種類の指摘が 0 件。docs/ui.md の「残したもの」から消した
- [ ] 部品のテスト (押せる範囲の端で押してもその操作になる) がある
- [ ] `just ci` が通る

## 参考

- T0675 のメモ (監査・起票の候補)、docs/ui.md
- apps/editor/src/lib/panels/ParametersPanel.svelte、インスペクタのキーフォームの節

## メモ

- T0041 で起票した (元の候補: T0675 (24px の押せる範囲))。
