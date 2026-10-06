---
id: T0976
title: "@ugoe/ui: Select・Menu の一覧が送るときだけ、行末をスクロールバーの分空ける"
status: done
milestone: M10
crates: [packages/ui, docs]
depends_on: [T1662]
assignee: claude-worker-T0976
---

# T0976: @ugoe/ui: Select・Menu の一覧が送るときだけ、行末をスクロールバーの分空ける

## 目的

WebKitGTK のスクロールバーは中身の上に重ねて描く (T0970)。T0970 は送る枠の行末に `--ugoe-scrollbar-gap` (16px) を空ける決めにしたが、
@ugoe/ui の `Select`・`Menu` の一覧 (padding 2px) は直さなかった。低い窓でしか送らず、いつも空けるとすべてのメニューが 14px 広がるため。
一覧が送るときだけ、行末 (行の右の要素。ショートカットやチェック) がスクロールバーに隠れないようにする。

## 受け入れ条件

- [x] `Select`・`Menu` の一覧が縦に送るとき (中身が枠より高いとき) だけ、行末を `--ugoe-scrollbar-gap` 空ける。送らないときの幅と見た目は変わらない
      (送るかどうかで padding を変えるか、行の右の要素の側で空けるかを決め、理由をメモに書いた。大きさが変わったら測り直す)
- [x] 部品のテスト: 送るときに行末を空け、送らないときは空けない (happy-dom は大きさを測らないので、大きさを差し替えるか `just ui-shots` の見本で確かめる)
- [x] packages/ui の `tokens.test.ts` の検査 (送る枠は `--ugoe-scrollbar-gap` を使うか理由を書く) が、この 2 つの部品でも通る
- [x] docs/ui.md (レイアウトの「送り」・部品の一覧) と packages/ui/README.md を直した
- [x] `just ui-shots`・`just ui-audit` (3 つのアプリ。600 秒に収まらなければ分けて) で増えた指摘が無い。`just ci` が通る

## 参考

- T0970 のメモ (やらなかったこと・起票の候補)、docs/ui.md (`--ugoe-scrollbar-gap`)
- packages/ui/src/components (Select・Menu)、packages/ui/src/tokens.test.ts

## メモ

- T0052 で起票した (元の候補: T0970 (Select・Menu が送るときだけ行末を空ける))。
- 同じ tokens.test.ts を触る T1662 (エディタの送る枠) の後にした。
- やったこと:
  - 本物の WebKitGTK 2.52 (Xvfb。部品の見本を WebKit2 4.1 で開き、一覧の max-height を縮めて送らせた) で測った: `Menu` の一覧は
    `offsetWidth - clientWidth` が 2 (枠線だけ) で、スクロールバーが行末のショートカット (「Ctrl+S」の S) に重なっていた。
    `Select` の一覧は Bits UI が中の viewport を送り、そのスクロールバーを隠していた (`scrollbar-width: none`・`::-webkit-scrollbar { display: none }`)。
  - packages/ui に `src/scrollGap.ts` (`markScrolling`。Svelte の attachment) を足した。中身が枠より高い (`scrollHeight - clientHeight > 1`) ときだけ
    `data-scrolling` を付け、枠と直下の項目の ResizeObserver と項目の増減 (MutationObserver) で測り直す。`Menu`・`ContextMenu`・下の段 (SubContent)・
    `Select` の一覧に付け、CSS で `[data-scrolling]` の枠の `padding-inline-end` を `--ugoe-scrollbar-gap` にした。
  - 行の右の要素ではなく枠の padding で空けた理由: 行末の要素が項目の種類ごとに違い (ショートカット・チェック・下の段の印・`Select` の名前の終わり)、
    選んだ行の枠 (outline) もスクロールバーから離したいため。空けると中身は狭く高くなるだけなので、送るかどうかが行き来しない。
  - `Select` は viewport の送りをやめ (`flex: none`・`overflow: visible` を `!important` で。Bits UI の inline の style に勝つため)、外の枠
    (`.ugoe-select__content`。前から `overflow: auto` と max-height) を送ってスクロールバーを見せるようにした。ほかの送る枠と同じく送れることが見え、
    同じ印で行末を空けられる。キーボードで下へ移ると選んだ行が見える所まで送られることを WebKitGTK で確かめた (Bits UI は `scrollIntoView` で送る)。
  - 測り直し (WebKitGTK): 送るとき Menu は幅 251 → 265px (行末の padding 2 → 16px)、Select は 133 → 147px。max-height を戻すと印が外れ、
    幅は 251px・133px に戻る。画像でショートカットと選択肢の名前がスクロールバーから離れたことを見た。
  - `tokens.test.ts` の送る枠の検査の対象に packages/ui/src/components を足した。Menu・Select の規則に「下の [data-scrolling] がスクロールバーの分空ける」、
    Dialog に「padding (space-5 は 16px 以上) が足りる」のコメントを書いた。コメントを消すと 2 つの規則を名前で出して落ちることを確かめた。
  - 部品のテスト `src/scrollGap.test.ts` (高さを差し替える): 送るときだけ印を付ける・1px の丸めは送らないとみなす・項目の増減で測り直す・
    Menu・Select・ContextMenu の一覧が送るときだけ印を付ける。
  - docs/ui.md (レイアウトの「送り」・部品の一覧の Select と Menu の行) と packages/ui/README.md を直した。
  - `just ui-shots` が通った (指摘なし)。`just ui-audit` を直す前 (一時的な revert のコミットで撮り、後で戻した) と比べた:
    viewer・studio +0 -0 ~0、editor は `--variants en-`・`ja-` に分けて +0 -0 ~0。`just e2e --only viewer,studio` (6・13)・`--only editor` が通った。`just ci` が通った。
- やらなかったこと・確かめていないこと:
  - WebView2 (Windows)・WKWebView (macOS) は確かめていない。幅を取るスクロールバーでは、空けた分が左に余白として残るだけのはず。
  - Bits UI の `Select.ScrollUpButton`・`ScrollDownButton` は使っていない (スクロールバーで足りるため)。
- 起票の候補: なし。
