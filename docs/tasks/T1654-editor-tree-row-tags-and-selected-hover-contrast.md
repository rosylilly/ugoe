---
id: T1654
title: "エディタ: ツリーで base と hidden の札を両方持つ行の名前が 1 文字に縮むのと、選んだ非表示・本体の行のホバーのコントラストを直す"
status: in-progress
milestone: M10
crates: [apps/editor, docs]
depends_on: [T0048]
assignee: claude-worker-T1654
---

# T1654: エディタ: ツリーで base と hidden の札を両方持つ行の名前が 1 文字に縮むのと、選んだ非表示・本体の行のホバーのコントラストを直す

## 目的

ツリーの行 (`apps/editor/src/lib/panels/tree/TreeItem.svelte`) の 2 つの不具合を直す。

- モジュールのプロジェクトで、`base` と `hidden` の 2 つの札 (`.ed-tag`) が付く行の名前が 1 文字 (「E…」) まで縮む (T0919。本物の WebKitGTK で見つけた)。
- 非表示 (`.row.hidden`)・本体 (`.row.base`) の要素を選んだ行にポインタを乗せると、名前 (text-muted) が accent-soft と hover の重ねの上で
  ダーク 4.29 になる (T0851 の計算。4.5:1 に届かない)。T0851 のスタジオのシーンの一覧と同じ直し方 (選んだ行では本文の色へ寄せる) が使える。

## 受け入れ条件

- [ ] 札を 2 つ持つ行でも、既定の幅の右の列で名前が読める長さを保つ (札を縮める・1 つの印にまとめる・名前を優先して札を省く、など。
      選んだ形と理由をメモに書いた)。札の意味は読み上げ (`title` か見せずに読ませる文字) で残る
- [ ] 選んだ非表示・本体の行の hover・押下でも名前が 4.5:1 以上 (ライト・ダーク)。トークン (`--ugoe-overlay-hover`) は変えない
- [ ] T0048 で足した場面を含む `just ui-audit --only editor` で、ツリーのコントラストの指摘が 0 件、`--compare` で増えた指摘が無い
- [ ] 部品のテスト (TreePanel.test.ts など) で、札を 2 つ持つ行の名前と札の読み上げを確かめた
- [ ] docs/ui.md (エディタのパネル) に直し方を書いた。マニュアルの更新の候補をメモに書いた
- [ ] `just e2e --only editor` と `just ci` が通る

## 参考

- T0919 のメモ (起票の候補)・docs/roadmap.md の M10 の「後続の候補 (T0919)」、T0851 のメモ (直し方・起票の候補)
- apps/editor/src/lib/panels/tree/TreeItem.svelte、docs/ui.md

## メモ

- T0046 で起票した (元の候補: T0919 (札 2 つの行の名前が 1 文字に縮む)・T0851 (選んだ非表示・本体の行の hover のコントラスト))。
  同じ TreeItem.svelte の行の見た目なので 1 つにまとめた。
- 監査に場面が無いので、場面を足す T0048 の後にした。
- T0048 で場面 `-tree-hidden-hover`・`-tree-base-hover` を足した。非表示の要素を選んだ行のホバーはダークの 4 組で 4.29 (名前「Wave」と札「hidden」)。本体の要素を選んだ行のホバーは**ライト**の 6 組で 4.44 (名前「Head」と札「base」。地が sunken のため)、ダークは 0 件。両方を直す対象にする (T0048 のメモ)。
