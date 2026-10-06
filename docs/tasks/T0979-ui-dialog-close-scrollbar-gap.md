---
id: T0979
title: "@ugoe/ui: Dialog の閉じるボタンを、重ねて描くスクロールバーから離す"
status: done
milestone: M10
crates: [packages/ui, tests, docs]
depends_on: []
assignee: claude-worker-T0979
---

# T0979: @ugoe/ui: Dialog の閉じるボタンを、重ねて描くスクロールバーから離す

## 目的

T0055 で足した ui-audit の `scrollbar` の検査で、エディタの 200% の拡大の設定のダイアログ (`*-zoom200-settings-ai`) の閉じるボタン
(`.ugoe-dialog__close`) が指摘として残った。ダイアログの全体が縦に送るときだけ、ボタンの右が枠の右端から `space-4` (12px) しか空いておらず、
指を乗せて太ったスクロールバー (WebKitGTK・macOS で 14px まで) が 2px 掛かる。決め (docs/ui.md#レイアウト の「送り」) は、
縦に送る枠の行末を `--ugoe-scrollbar-gap` 空けること。

## 受け入れ条件

- [x] packages/ui の Dialog で、閉じるボタンの右を `--ugoe-scrollbar-gap` 以上空ける。ダイアログが送るときだけ空ける
      (`scrollGap.ts` の `markScrolling` の `data-scrolling` などで) か、いつも空けるかを決め、理由をメモと docs/ui.md に書いた
      (いつも空けるとすべてのダイアログの見た目が変わる。コンパクトの密度でも見る)
- [x] 振る舞いを変えたらテスト (packages/ui の components.test.ts など) を書いた
- [x] `just ui-audit --only editor` (少なくとも `zoom200` の組) で `scrollbar` の `.ugoe-dialog__close` の指摘が消え、ほかの指摘が増えていない
      (直す前と `--compare` で比べる)。ビューア・スタジオのダイアログも `--quick` で見た
- [x] tests/ui-audit/README.md の「重ねて描くスクロールバー」の「残る指摘 (直していないもの)」を今の形にした
- [x] 画面の部品・クラス名を変えたら 3 つのアプリの `just e2e` を流した。`just ci` が通る

## 参考

- T0055 のメモ (直していない指摘)、tests/ui-audit/README.md の「重ねて描くスクロールバー」
- packages/ui/src/components/Dialog.svelte (`.ugoe-dialog__close`)、packages/ui/src/scrollGap.ts (T0976 の一覧の送り)
- docs/ui.md#レイアウト の「送り」(T0970)

## メモ

- T0063 で起票した (元の候補: T0055 (Dialog の閉じるボタンの空き))。
- やったこと:
  - packages/ui の Dialog で、閉じるボタンの右を `--ugoe-dialog-close-inset` (= `max(space-4, --ugoe-scrollbar-gap)`) にした。
    **いつも空ける**ことにした: ダイアログの行末の padding (`space-5`) はもともといつも `--ugoe-scrollbar-gap` 以上空けていて、
    ボタンの右が中身・タブの線・決定のボタンの右の端にそろう。変わるのはコンパクトの密度の 4px だけ (ゆったりは `space-4` が 16px で
    変わらない)。送るときだけ (`markScrolling`) にすると、窓の高さ・拡大でボタンが 4px 動き、ダイアログごとに ResizeObserver が要る。
    理由は docs/ui.md#レイアウト の「送り」とコメントに書いた。
  - 見出しの右の空きを、ボタンの位置と大きさ (`max(control-height, target-min)`) から求めるようにした (ボタンの左から `space-3`)。
    前は `target-min + space-2` で、ゆったりの密度ではボタン (30px) と見出しの間が 2px しかなかった。今はどちらの密度でも 8px。
  - テスト: packages/ui の `tokens.test.ts` に、閉じるボタンの `right` が `--ugoe-scrollbar-gap` を含む `max()` であることを足した
    (直す前の Dialog.svelte で失敗することを確かめた)。CSS の計算は happy-dom ではできないので、ソースの文で見る。
  - `just ui-audit --only editor` (全部の組。568 の場面) を直す前と後で `--compare`: `-1` (`.ugoe-dialog__close` の 2 つの場面の指摘) だけで、
    増えた指摘は無い。ビューア・スタジオは `--quick` で前と後を比べて `+0 -0`。
  - 本物の WebKitGTK (e2e のエディタに一時の手順を足して、1000×420 の窓で設定の AI provider を開き、ダイアログの右端の閉じるボタンの高さに
    指を乗せて撮った。手順はコミットしていない): 直す前はボタンの右の空きが 12px で、太ったスクロールバーの左の端にボタンが 2px 掛かり、
    直した後は空き 16px で 2px 離れた。部品の見本 (`just ui-shots`) もコンパクト・ゆったりで見た。
  - tests/ui-audit/README.md の「残る指摘 (直していないもの)」を「ありません」にした。
  - `just e2e --only editor,viewer,studio`: エディタ・スタジオは通った。ビューアは 1 回目に WebDriver の中継の接続の失敗
    (`tcp connect error: Cannot assign requested address (os error 99)`。並行のワーカーでローカルのポートが尽きたものと見られ、変更とは無関係) で
    落ち、`--no-build --only viewer` で流し直して通った。`just ci` は通った。
- やらなかったこと: docs/manual/images の撮り直し (T1671 が撮り直し中。ダイアログの閉じるボタンがコンパクトで 4px 左へ動くので、
  ダイアログが写る画像は T1671 の撮り直しで変わりうる)。
- 起票の候補: なし。
