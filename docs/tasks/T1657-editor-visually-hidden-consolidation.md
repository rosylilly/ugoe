---
id: T1657
title: "エディタ: 自前で書いた見せずに読ませる形 (光の色・光のスライダー・提案の表・描く順の欄・プレビューの表の見出し) をそろえ、T0674 の打ち消しを外す"
status: todo
milestone: M10
crates: [apps/editor, packages/ui, docs]
depends_on: []
assignee:
---

# T1657: エディタ: 自前で書いた見せずに読ませる形 (光の色・光のスライダー・提案の表・描く順の欄・プレビューの表の見出し) をそろえ、T0674 の打ち消しを外す

## 目的

エディタのいくつかの部品は、見せずに読ませる文字を `.ugoe-visually-hidden` ではなく自前の `position: absolute; width: 1px; …` で書いている
(`LightColor`・`PreviewLightsSection`・`ProposalsPanel`、描く順の欄の名前、プレビューの表の見出し `thead`)。これらは送る枠の外を位置の基準にするので、
T0674 でパネルの送る枠に付けた打ち消し (`position: relative`) を外せない (T0954 で外すと `overflow` に 4 種類出た)。
docs/ui.md は「アプリの CSS に同じ指定を書き直さない」と決めている。

## 受け入れ条件

- [ ] 上の要素を洗い出し、`.ugoe-visually-hidden` (か、部品の名前を見せない口) にそろえた。どちらにしたかを要素ごとにメモに書いた。
      部品 (`Slider`・`ColorPicker`) に名前を見せない口を足すなら、@ugoe/ui の部品のテストと docs/ui.md の部品の一覧を直した
- [ ] 見せずに読ませる文字を包む要素 (か部品) が位置の基準を持ち、T0674 のパネルの送る枠の打ち消しを外しても `just ui-audit --only editor` の
      `overflow` が 0 件。外せないものが残るなら、理由を docs/ui.md に書いて打ち消しを残す
- [ ] 読み上げの名前が変わらないことを部品のテスト (`getByRole(..., { name })` など) で確かめた
- [ ] docs/ui.md (エディタのパネル (T0674) の「送り」) を直した
- [ ] `just ui-audit --only editor` の `--compare` で増えた指摘が無く、`just e2e --only editor` と `just ci` が通る

## 参考

- T0954 のメモ (エディタの打ち消しは外さなかった・起票の候補)、T0674
- docs/ui.md (レイアウトの「送り」)、apps/editor/src/lib/panels/lighting (LightColor.svelte・PreviewLightsSection.svelte)、
  apps/editor/src/lib/ai/ProposalsPanel.svelte、packages/ui (`.ugoe-visually-hidden`・Slider・ColorPicker)

## メモ

- T0046 で起票した (元の候補: T0954 (アプリが自分で書いている見せずに読ませる形))。
- 部品に口を足すか、アプリで `.ugoe-visually-hidden` にするかは docs/ui.md の決め (アプリで同じ指定を書き直さない) から作業者が決めてよい
  (利用者の判断は要らない)。@ugoe/ui の口はアプリの間の内部の API。
