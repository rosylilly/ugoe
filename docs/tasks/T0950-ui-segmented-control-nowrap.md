---
id: T0950
title: "@ugoe/ui: SegmentedControl の選択肢の名前を字の途中で折らない"
status: done
milestone: M10
crates: [packages/ui, apps/studio, docs]
depends_on: []
assignee: claude-worker-T0950
---

# T0950: @ugoe/ui: SegmentedControl の選択肢の名前を字の途中で折らない

## 目的

`SegmentedControl` は狭い所で選択肢の名前が字の途中で折れる (T0818 で、スタジオの操作 API の承認の知らせの「権限」(Permission) が 360px で折れた)。
スタジオでは承認の知らせだけ CSS で避けた (T0818 の候補)。部品の側で、名前を折らずに選択肢を次の行へ送る。

## 受け入れ条件

- [x] `SegmentedControl` の選択肢の名前を `nowrap` にし、幅が足りないときは選択肢ごと次の行へ折り返す (見た目の枠と選択の印が崩れない)。部品のテストか見本の画面がある
- [x] スタジオの承認の知らせの回避の CSS を外しても 360px で折れない
- [x] `just ui-audit` (3 つのアプリ) の指摘が増えない
- [x] docs/ui.md (部品の一覧) を直した
- [x] `just ci` が通る

## 参考

- T0818 のメモ (狭い幅の比べ・起票の候補)
- packages/ui の `SegmentedControl`、apps/studio の操作 API の承認の知らせ

## メモ

- T0040 で起票した (元の候補: T0818 (`SegmentedControl`))。
- やったこと:
  - `SegmentedControl` (packages/ui) の全体と選択肢のまとまりを `flex-wrap: wrap`・`max-width: 100%` にし、名前を `nowrap` に
    (選択肢はもともと `nowrap`)。幅が足りないと、名前の後ろでまとまりごと、さらに選択肢ごとに次の行へ送る。枠と選択の印はそのまま。
  - スタジオの承認の知らせ (`ApiApproval.svelte`) の回避の CSS (T0818) を外した。`@ugoe/player` の `PosePicker` にあった同じ回避
    (`flex-wrap: wrap`) も、部品の側と重なるので外した (`crates:` の外だが 6 行の削除だけ)。
  - 見本の画面に幅 7rem の「狭い所」の例を足し、見本の画面が描き終えたときに描いた形を測る (`gallery/main.ts`): `SegmentedControl` の名前と
    選択肢が 2 行以上に分かれていないか、狭い所の例がはみ出さずに次の行へ送られたか。`just ui-shots` が問題を報告する。部品を直す前は
    21 枚すべてで「折れた・はみ出した・送られていない」と出て、直した後は 21 枚とも通ることを確かめた。
  - docs/ui.md (部品の一覧・T0818 の節) と packages/ui/README.md (`ui-shots` が確かめること) を直した。
- 確かめたこと: `just ui-shots` (21 枚すべて ok)、`just ui-audit --compare` (3 つのアプリ、前も後も 0 件、+0 -0 ~0)、スタジオの
  360px の承認の知らせで「Permission」が折れずに次の行へ送られること (撮った画面)、`just e2e --only studio`・`viewer`・`editor` が通る、
  `just ci` が通る。
- やらなかったこと: Vitest (happy-dom) は描いた形を測れないので、部品のテストではなく見本の画面の検査で確かめた。
