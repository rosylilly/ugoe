---
id: T0048
title: "ui-audit: 右の列へ送るときに頭のタブの高さを引き、パラメータの絞り込みとツリーの選んだ行のホバーの場面を足す"
status: done
milestone: M10
crates: [tests, docs]
depends_on: [T0047]
assignee: claude-worker-T0048
---

# T0048: ui-audit: 右の列へ送るときに頭のタブの高さを引き、パラメータの絞り込みとツリーの選んだ行のホバーの場面を足す

## 目的

- `tests/ui-audit/lib/apps.mjs` の `scrollToElement` で右の列の要素へ送ると、頭に固定されたタブ (sticky) の下に隠れる
  (`-ai-list` で見出しが切れる)。T0044 はグルーの場面だけでタブの高さを引いた。共通の送り方で引く (T0044)。
- パラメータの絞り込みの欄 (T1646) は、欄が出ている `-many-parameters` だけで見ていて、一致なしの文と絞り込み中の一覧を撮っていない (T1646)。
- エディタのツリーで、非表示 (`.row.hidden`)・本体 (`.row.base`) の要素を選んだ行にポインタを乗せた場面が無い。計算ではダークで名前の
  文字 (text-muted) が 4.29 (T0851)。T1654 が直す前に、監査で見つけられるようにする。

## 受け入れ条件

- [x] `scrollToElement` (か共通の送り方) が、送る枠の頭に固定された要素の高さを引いて送る。グルーの場面だけの引き方は共通の形に置き換えた。
      `-ai-list` の見出しが切れないことを画像で確かめた
- [x] エディタの場面 `-parameter-filter` (語で絞った一覧) と `-parameter-filter-none` (一致なしの文) を足した
- [x] エディタの場面に、非表示の要素と本体の要素 (モジュールのプロジェクト) を選んだ行にポインタを乗せたものを足した。今のコードで
      ダークの組にコントラストの指摘が出る (出なければ、測れない理由をメモに書く)
- [x] tests/ui-audit/README.md の場面の表を直した
- [x] `just ui-audit --only editor` を流し、新しい場面の指摘をメモに書いた (直すのは T1654)
- [x] `just ui-audit-check` と `just ci` が通る

## 参考

- T0044 のメモ (起票の候補)、T1646 のメモ (起票の候補 (2))、T0851 のメモ (起票の候補)
- tests/ui-audit/README.md、tests/ui-audit/lib/apps.mjs

## メモ

- T0046 で起票した (元の候補: T0044 (sticky のタブの下に隠れる)・T1646 (絞り込みの場面)・T0851 (ツリーの選んだ行のホバー。監査に場面が無い))。
- 同じ apps.mjs を触るので T0047 の後にした。

### やったこと (claude-worker-T0048)

- **sticky の高さを引く送り方:** `tests/ui-audit/lib/apps.mjs` の `scrollToElement` と `scrollToSection` を、ページの中で動く 1 つの関数
  `scrollInBox` にまとめた。送れる祖先 (枠) の頭まで送った後、枠の中の `position: sticky` (`top` が `auto` でない) の要素のうち、
  間に別の送る枠が無く、送った要素を含まず、送った要素の下端より上にあるものの下端を求め、その分と 8px だけ戻す。
  グルーの場面だけの引き方 (`.pane .tabs` の高さ) は消した。送った後の位置は画素に丸められるので、要素の頭ではなく下端と比べる
  (頭と比べると、丸めで 0.5px 上にずれたときにタブを見落として引かなかった)。`-ai-list` は見出し「Proposals」がタブの下に出ることを、
  `-glue-edit` は軸の欄が前と同じ位置に出ることを画像で確かめた (`en-dark-wide`)。
- **場面を足した:** エディタ `-parameter-filter` (`manyParameters` のモデルで絞り込みの欄に「long」: 7 個に絞った Head の区切り) と
  `-parameter-filter-none` (「no-such-parameter」: 一致なしの文。`role="status"` を待って撮る。aria も流す)。
  `-tree-hidden-hover` (見本の不透明度 0 のパーツ「Wave」を選んで乗せる。ツリーの見た目の場面の後) と `-tree-base-hover`
  (モジュールのプロジェクトで本体の「Head」を選んで乗せる。`-base-change` の後)。左の列がデフォーマの木になっている場面の後なので、
  パーツの木のタブに切り替えてから選ぶ (`treeSelectedHoverScene`)。選んだのはそのまま残す (後の場面は選び直すか、選びに依らない)。
- README の場面の表と、場面を足すときの説明 (送るときは `scrollToElement`・`scrollToSection`) を直した。`just ui-audit-check` が通る。

### `just ui-audit --only editor` の結果

- 前面の 600 秒では 13 の組を通しで終えられない (590 秒で打ち切り) ので、ビルドした後 `--no-build --variants dark` (5 組・221 場面) と
  `--variants light` (8 組・311 場面) の 2 回に分けて流した (rebase の前)。rebase の後 (T0049 の重なりの検査が入った main) に、ビルドし直して
  `--variants en-dark-wide,en-light-wide` (89 場面) を流した。場面の失敗は 0。
- 新しい場面の指摘 (コントラストだけ。直すのは T1654):
  - `-tree-hidden-hover` のダーク (`en-dark-wide`・`en-dark-narrow`・`ja-dark-wide`・`ja-dark-narrow`): 名前 `span.name "Wave"` と札
    `span.ed-tag "hidden"/"非表示"` が 4.29 (text-muted #a1a1aa の上、accent-soft + hover の重ね rgb(62, 58, 75))。T0851 の計算どおり。
    ライトは 0 件。
  - `-tree-base-hover` のライト (`en/ja-light-wide`・`-narrow`・`-zoom200` の 6 組): 名前 `span.name "Head"` と札 `span.ed-tag "base"/"本体"`
    が 4.44 (text-muted #5f5f68 の上、sunken + hover の重ね rgb(216, 216, 218))。**ダークは 0 件:** 本体の要素を選んだ行の地は
    accent-soft ではなく sunken (`.row.base.selected`) で、ダークの sunken (#111113) + hover の上では text-muted が約 6.3 (計算) になる。
    T0851 の候補の「本体の要素もダークで 4.29」は当たらず、本体の要素はライトで届かない。
  - `-parameter-filter`・`-parameter-filter-none`・(送り方を変えた) `-ai-list`・`-glue-edit` は、すべての組で 0 件。
  - 強制色の組は新しい場面も 0 件 (本体の行の文字 1 つは canvas の上で `unmeasured`)。
- 前からある指摘: `en-dark-forced-wide-tab` の `div[tree] "Parts"` の `faint focus ring` (T0045 のメモにあるもの。この変更とは関係ない)。

### やらなかったこと

- 見つけたコントラストの直し (T1654)。T1654 には、本体の要素はライトの sunken + hover で 4.44 になること (ダークではなく) を伝えたい。
- 前と後の `--compare` は取っていない (足した場面のほかに変わるのは送った位置だけで、`-ai-list`・`-glue-edit`・`-preview-dress-up`・
  `-module-preview`・`-many-parameters-groups` の指摘は 0 件のまま)。

### 起票の候補

- T1654 の範囲の確認: ツリーの本体の要素を選んだ行 (`.row.base.selected`) に乗せたとき、ライトで text-muted が 4.44 (上の結果)。
- ui-audit: `just ui-audit --only editor` が 13 の組で 10 分ほどかかり、前面の 600 秒に収まらない。組を並行に流すか、組ごとの場面を減らす。
