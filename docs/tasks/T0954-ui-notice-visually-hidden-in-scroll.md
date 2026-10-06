---
id: T0954
title: "@ugoe/ui: Notice の中の見せずに読ませる文字が、送る枠の外を位置の基準にしないようにする"
status: done
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

- [x] `Notice` 側で直す (`position: relative` を付けるなど) か、各アプリの送る枠を直すかを決め、理由をメモに書いた
- [x] `position` を持たない送る枠の中に `Notice` を置いたとき、枠の外に送りやはみ出しが出ないことを確かめるテスト (部品のテストか `just ui-audit` の検査) がある
- [x] エディタで T0674 が入れた個別の打ち消しが要らなくなれば外した。3 つのアプリの `just ui-audit` の指摘が増えない
- [x] docs/ui.md (送り) を直した
- [x] `just ci` が通る

## 参考

- T0674 のメモ (起票の候補)、docs/ui.md (送り)
- packages/ui/src/components/Notice.svelte、packages/ui の `.ugoe-visually-hidden`

## メモ

- T0041 で起票した (元の候補: T0674 (`Notice` の見せずに読ませる文字))。

- **決めたこと:** `Notice` 側で直した (`.ugoe-notice` に `position: relative`)。理由: 見せずに読ませる文字は部品の中の作りで、置く側が知らなくてよいようにするため。
  各アプリの送る枠を直す形だと、`Notice` を新しい送る枠に置くたびに同じ誤りが起きうる。`Toaster` は `position: fixed` の領域が基準なので触っていない。
- **テスト:**
  - 部品の見本 (`just ui-shots`): 知らせのカードに `position` の無い送る枠 (`.gallery__scroller`) と、その下の方に `Notice` を置き、
    `gallery/main.ts` で見せずに読ませる文字の `offsetParent` が枠の内側かを測る。直す前は 21 枚すべてで NG、直した後はすべて ok。
  - `just ui-audit` の `overflow` に、送る (切り取る) 枠の外を位置の基準にして枠の外へ出ている absolute の要素 (`… escapes …`) を足した
    (`tests/ui-audit/lib/page-checks.js`。範囲外の小さな変更。README と単体テストも)。直す前のビルドに当てると、スタジオの
    `aside.studio__panels` の中の `Notice` (「Warning:」「Information:」) を 7 場面で見つけた (ビューアは 0。エディタは打ち消しがあるので 0)。
    枠に重ねて置いた絵 (ビューポートの上の SVG) は枠の内側にあるので数えない。
- **エディタの打ち消し (T0674) は外さなかった:** 外して `just ui-audit --only editor --quick` を流すと、`overflow` に 4 種類出た
  (描く順の欄の名前・プレビューの表の見出し `thead`・光の欄の色の値 (`LightColor`)・スライダーの名前 (`PreviewLightsSection`)。どれもアプリが自分で
  見せずに読ませる形にした要素)。`Notice` の修正だけでは足りないので残し、docs/ui.md に理由を書いた。
- **`just ui-audit` (3 つのアプリ、すべての場面、`--compare` で直す前 (a944c028) の基準線と比べた):** editor +0 -0 ~0・viewer +0 -0 ~0・studio +0 -0 ~0
  (新しい検査を入れた後でも 0 件。スタジオのコントラストの 2 種類は前からある)。main に rebase する前に流した。
- **`just e2e`:** editor・viewer・studio すべて通った (rebase の前)。
- **`just ci`:** 通った (main ddabec2c に rebase した後)。
- **文書:** docs/ui.md (レイアウトの「送り」・エディタのパネル (T0674) の「送り」)、docs/testing.md、packages/ui/README.md、tests/ui-audit/README.md。
- **起票の候補:**
  - アプリが自分で書いている見せずに読ませる形 (`LightColor`・`PreviewLightsSection`・`ProposalsPanel` の `position: absolute; width: 1px; …`) を
    `.ugoe-visually-hidden` にそろえるか、部品 (`Slider`・`ColorPicker`) に名前を見せない口を足す。
