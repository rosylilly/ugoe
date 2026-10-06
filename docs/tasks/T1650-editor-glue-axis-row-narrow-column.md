---
id: T1650
title: "エディタ: グルーの軸を足す行で、右の列が狭いとき選択欄の矢印が数値の欄の下に隠れるのを直す"
status: todo
milestone: M10
crates: [apps/editor, docs]
depends_on: []
assignee:
---

# T1650: エディタ: グルーの軸を足す行で、右の列が狭いとき選択欄の矢印が数値の欄の下に隠れるのを直す

## 目的

グルーのパネル (`GluesPanel.svelte`) の強さの軸を足す行 (`.row`) は、右の列が狭いとき (`*-zoom200`。列の中が約 235px)、
パラメータの `Select` が `.grow` の `min-width: 0` で縮み、矢印が右の数値の欄の下に出る (T0044 の `ja-light-zoom200-glue-edit.png`)。
監査の `overlap` は文字の重なりだけを数えるので出ない。

## 受け入れ条件

- [ ] 狭い列で、選択欄と数値の欄が重ならない (折り返す・選択欄に最小の幅を持たせる、など。選んだ形と理由をメモに書いた)。
      広い列の見た目は変わらない
- [ ] グルーのパネルのほかの行 (キーを足す欄・組の一覧の「Add pair」の行) にも同じ形が無いかを確かめ、あれば直した
- [ ] `just ui-audit --only editor` の `-glue-edit` の `*-zoom200` の画像で重なりが無いことを確かめ、`--compare` で増えた指摘が無い
- [ ] `just e2e --only editor` と `just ci` が通る

## 参考

- T0044 のメモ (新しい場面で見つけたもの・起票の候補)
- apps/editor/src/lib/panels/glues/GluesPanel.svelte、tests/ui-audit/README.md

## メモ

- T0046 で起票した (元の候補: T0044 (グルーの軸を足す行の重なり))。
- T0049 (重なりの検査) を先に入れるなら、その検査が 0 件になることも確かめる。依存は付けていない。
