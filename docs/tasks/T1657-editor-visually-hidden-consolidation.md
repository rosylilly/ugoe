---
id: T1657
title: "エディタ: 自前で書いた見せずに読ませる形 (光の色・光のスライダー・提案の表・描く順の欄・プレビューの表の見出し) をそろえ、T0674 の打ち消しを外す"
status: done
milestone: M10
crates: [apps/editor, packages/ui, docs]
depends_on: []
assignee: claude-worker-T1657
---

# T1657: エディタ: 自前で書いた見せずに読ませる形 (光の色・光のスライダー・提案の表・描く順の欄・プレビューの表の見出し) をそろえ、T0674 の打ち消しを外す

## 目的

エディタのいくつかの部品は、見せずに読ませる文字を `.ugoe-visually-hidden` ではなく自前の `position: absolute; width: 1px; …` で書いている
(`LightColor`・`PreviewLightsSection`・`ProposalsPanel`、描く順の欄の名前、プレビューの表の見出し `thead`)。これらは送る枠の外を位置の基準にするので、
T0674 でパネルの送る枠に付けた打ち消し (`position: relative`) を外せない (T0954 で外すと `overflow` に 4 種類出た)。
docs/ui.md は「アプリの CSS に同じ指定を書き直さない」と決めている。

## 受け入れ条件

- [x] 上の要素を洗い出し、`.ugoe-visually-hidden` (か、部品の名前を見せない口) にそろえた。どちらにしたかを要素ごとにメモに書いた。
      部品 (`Slider`・`ColorPicker`) に名前を見せない口を足すなら、@ugoe/ui の部品のテストと docs/ui.md の部品の一覧を直した
- [x] 見せずに読ませる文字を包む要素 (か部品) が位置の基準を持ち、T0674 のパネルの送る枠の打ち消しを外しても `just ui-audit --only editor` の
      `overflow` が 0 件。外せないものが残るなら、理由を docs/ui.md に書いて打ち消しを残す
- [x] 読み上げの名前が変わらないことを部品のテスト (`getByRole(..., { name })` など) で確かめた
- [x] docs/ui.md (エディタのパネル (T0674) の「送り」) を直した
- [x] `just ui-audit --only editor` の `--compare` で増えた指摘が無く、`just e2e --only editor` と `just ci` が通る

## 参考

- T0954 のメモ (エディタの打ち消しは外さなかった・起票の候補)、T0674
- docs/ui.md (レイアウトの「送り」)、apps/editor/src/lib/panels/lighting (LightColor.svelte・PreviewLightsSection.svelte)、
  apps/editor/src/lib/ai/ProposalsPanel.svelte、packages/ui (`.ugoe-visually-hidden`・Slider・ColorPicker)

## メモ

- T0046 で起票した (元の候補: T0954 (アプリが自分で書いている見せずに読ませる形))。
- 部品に口を足すか、アプリで `.ugoe-visually-hidden` にするかは docs/ui.md の決め (アプリで同じ指定を書き直さない) から作業者が決めてよい
  (利用者の判断は要らない)。@ugoe/ui の口はアプリの間の内部の API。

- **要素ごとにしたこと:**
  - 光の欄の色の値 (`LightColor` の `ColorPicker` の `#rrggbb`): `ColorPicker` に `hideValue` を足した (値に `.ugoe-visually-hidden`、トリガーが位置の基準)。
    アプリの `:global(.ugoe-color-picker__value)` の指定と包む `<span>` を除いた。
  - 光のスライダーの名前 (`PreviewLightsSection`) と AI の提案の法線マップの強さのスライダーの名前 (`ProposalsPanel`): `Slider` に `hideLabel` を足した
    (名前に `.ugoe-visually-hidden`、`.ugoe-slider` が位置の基準。名前の段を格子から外す)。アプリの `:global(.ugoe-slider__label)` の指定を除いた。
  - 描く順の欄の名前 (`SlotsPanel`): 前から `.ugoe-visually-hidden`。包む `.range` を `position: relative` にした。
  - プレビューの表の見出し (`DressUpSection` の `thead`): 前から `.ugoe-visually-hidden`。表 (`.items`) を `position: relative` にした。
  - 部品の口にしたのは、アプリの CSS が部品の中の要素の作りに頼らないようにするため (docs/ui.md の「アプリで同じ指定を書き直さない」)。
    描く順と表はアプリ自身の要素なので、包む要素を基準にした。
- **打ち消しを外した:** `app.css` の `aside` と `App.svelte` の `.pane` の `position: relative` (T0674) を除いた。残したものは無い。
- **テスト:**
  - @ugoe/ui: `Slider` の `hideLabel` (つまみと数値の欄の名前が変わらない・既定は見せる)、`ColorPicker` の `hideValue` (トリガーの名前に値が入る・既定は見せる)。
    happy-dom は `offsetParent` を計算しないので、位置の基準は部品の見本 (`just ui-shots`) で測る: 見本の `position` の無い送る枠に
    `hideLabel` の `Slider` と `hideValue` の `ColorPicker` を足した (`gallery/main.ts` の検査はそのまま)。部品の `position: relative` を一時的に外すと
    すべての撮影で「送る枠の外を位置の基準にした」が出て、戻すとすべて ok。
  - エディタ: `PreviewLightsSection.test.ts` (スライダーと色の名前が翻訳の文言のままで、名前と値は見せない)。
- **`just ui-audit --only editor`:** すべての場面は 10 分に収まらなかった (並行のワーカーで重い) ので、`--variants en-light,ja-dark` に絞った
  (201 場面。幅 wide・narrow・half・xwide と拡大・強制色などを含む。レイアウトは言語と幅で決まり、テーマでは変わらないので en-dark・ja-light は省いた)。
  直す前 (main bce92544) の基準線と `--compare` で editor +0 -0 ~0、すべての検査で 0 件 (`overflow` も 0)。
- **`just e2e --only editor`:** 16 件すべて通った (rebase の前)。
- **`just ui-shots`:** すべて ok。
- **`just ci`:** 通った (main 5aa50903 に rebase した後)。
- **文書:** docs/ui.md (部品の一覧の `Slider`・`ColorPicker`、レイアウトの「送り」、エディタのパネル (T0674) の「送り」)、packages/ui/README.md。
- **起票の候補:** 無し。

