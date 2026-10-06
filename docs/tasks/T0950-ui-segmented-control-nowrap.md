---
id: T0950
title: "@ugoe/ui: SegmentedControl の選択肢の名前を字の途中で折らない"
status: todo
milestone: M10
crates: [packages/ui, apps/studio, docs]
depends_on: []
assignee:
---

# T0950: @ugoe/ui: SegmentedControl の選択肢の名前を字の途中で折らない

## 目的

`SegmentedControl` は狭い所で選択肢の名前が字の途中で折れる (T0818 で、スタジオの操作 API の承認の知らせの「権限」(Permission) が 360px で折れた)。
スタジオでは承認の知らせだけ CSS で避けた (T0818 の候補)。部品の側で、名前を折らずに選択肢を次の行へ送る。

## 受け入れ条件

- [ ] `SegmentedControl` の選択肢の名前を `nowrap` にし、幅が足りないときは選択肢ごと次の行へ折り返す (見た目の枠と選択の印が崩れない)。部品のテストか見本の画面がある
- [ ] スタジオの承認の知らせの回避の CSS を外しても 360px で折れない
- [ ] `just ui-audit` (3 つのアプリ) の指摘が増えない
- [ ] docs/ui.md (部品の一覧) を直した
- [ ] `just ci` が通る

## 参考

- T0818 のメモ (狭い幅の比べ・起票の候補)
- packages/ui の `SegmentedControl`、apps/studio の操作 API の承認の知らせ

## メモ

- T0040 で起票した (元の候補: T0818 (`SegmentedControl`))。
