---
id: T1650
title: "エディタ: グルーの軸を足す行で、右の列が狭いとき選択欄の矢印が数値の欄の下に隠れるのを直す"
status: done
milestone: M10
crates: [apps/editor, docs]
depends_on: []
assignee: claude-worker-T1650
---

# T1650: エディタ: グルーの軸を足す行で、右の列が狭いとき選択欄の矢印が数値の欄の下に隠れるのを直す

## 目的

グルーのパネル (`GluesPanel.svelte`) の強さの軸を足す行 (`.row`) は、右の列が狭いとき (`*-zoom200`。列の中が約 235px)、
パラメータの `Select` が `.grow` の `min-width: 0` で縮み、矢印が右の数値の欄の下に出る (T0044 の `ja-light-zoom200-glue-edit.png`)。
監査の `overlap` は文字の重なりだけを数えるので出ない。

## 受け入れ条件

- [x] 狭い列で、選択欄と数値の欄が重ならない (折り返す・選択欄に最小の幅を持たせる、など。選んだ形と理由をメモに書いた)。
      広い列の見た目は変わらない
- [x] グルーのパネルのほかの行 (キーを足す欄・組の一覧の「Add pair」の行) にも同じ形が無いかを確かめ、あれば直した
- [x] `just ui-audit --only editor` の `-glue-edit` の `*-zoom200` の画像で重なりが無いことを確かめ、`--compare` で増えた指摘が無い
- [x] `just e2e --only editor` と `just ci` が通る

## 参考

- T0044 のメモ (新しい場面で見つけたもの・起票の候補)
- apps/editor/src/lib/panels/glues/GluesPanel.svelte、tests/ui-audit/README.md

## メモ

- T0046 で起票した (元の候補: T0044 (グルーの軸を足す行の重なり))。
- T0049 (重なりの検査) を先に入れるなら、その検査が 0 件になることも確かめる。依存は付けていない。
- やったこと: 軸を足す行の `.grow` (選択欄を包む) から `min-width: 0` を外し、`max-width: 100%` にした。
  - 選んだ形: 折り返し。`.row` はもとから `flex-wrap: wrap` なので、`.grow` が選択欄の幅 (中身か、Select の `min-width: 8em`)
    より狭く縮まなくなれば、収まらないときに数値の欄と「+ 軸」が次の行へ送られる。Select の部品 (packages/ui) は変えない。
    `max-width: 100%` は、列より長いパラメータの名前のときに `.grow` が列の外へ出ないための止め (自動の最小の幅はこれで頭打ちになる)。
  - 狭い列 (`*-zoom200`): 選択欄と数値の欄が 1 行、「+ 軸」が次の行になり、矢印が見える (前後の `ja-light-zoom200-glue-edit.png`
    と `en-light-zoom200-glue-edit.png` を見て確かめた)。
  - 広い列: `en-light-wide`・`en-light-narrow` の `-glue-edit` の画像が、直す前と後でバイト単位で同じ。
- ほかの行: キーを足す欄 (`.keys`) と「Pair」の行・組の作り直しの行 (`.row`) は、どれも `min-width: 0` で縮める子を持たず
  (`NumberField` は `size` の幅の inline-flex)、`flex-wrap: wrap` で折り返すので同じ形は無い。ただし「Pair」の行と作り直しの行は
  `-glue-edit` の画像の下にあって写っていないので、確かめたのは CSS を読んだ範囲。
- 監査: `just ui-audit --only editor --variants zoom200` を前と後で流し、`--compare` で `editor: +0 -0 ~0`。
  `en-light-wide,en-light-half,en-light-narrow` も前と後で summary.json が同じ (生成時刻を除く)。
- `just e2e --only editor` は 15 件すべて通った。`just ci` は通った。
- T0049 (重なりの検査) はまだ入っていないので、その検査では確かめていない。
- 起票の候補: ui-audit の `-glue-edit` は軸を足す欄までで撮るので、組の一覧 (「Pair」の行) と組の作り直しの行が狭い列でどう見えるかは
  撮られていない。`-glue-edit-pairs` のように下まで送った場面を足す。
