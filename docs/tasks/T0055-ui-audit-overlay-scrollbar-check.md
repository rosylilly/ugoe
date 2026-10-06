---
id: T0055
title: "ui-audit: 重ねて描くスクロールバー (WebKitGTK) が中身に掛かるのを見つける検査を足す"
status: done
milestone: M10
crates: [tests, docs]
depends_on: [T0054, T1662]
assignee: claude-worker-T0055
---

# T0055: ui-audit: 重ねて描くスクロールバー (WebKitGTK) が中身に掛かるのを見つける検査を足す

## 目的

WebKitGTK のスクロールバーは幅を取らずに中身の上に重ねて描き、指を乗せると 6〜14px に太る (T0970 で測った)。ヘッドレスの Chrome は
スクロールバーに幅を取るので、`just ui-audit` では中身が隠れる形を見つけられない。T0970 は `--ugoe-scrollbar-gap` (16px) を送る枠の
行末に空ける決めにした。その決めが守られているかを、画面の上で確かめる検査を足す。

## 受け入れ条件

- [x] 送る枠 (`overflow(-y): auto|scroll` で今送れるもの) の中の文字・押せる部品の右端が、枠の見える右端から `--ugoe-scrollbar-gap`
      の中に入っていれば指摘する (`<部品> is under the overlay scrollbar of <枠>` のような文)。数えないもの (意図して端まで描く地・区切り線など)
      と閾値を決め、tests/ui-audit/README.md に書いた
- [x] 単体テスト (`test/checks.test.mjs`): 行末を空けない枠の中の部品を見つけ、空けた枠・送らない枠・横に送る枠を数えない
- [x] 3 つのアプリのすべての場面を流し、指摘を直したか、意図したもの (理由) としてメモに書いた (T0970・T1662 で直した枠は 0 件のはず)
- [x] `just ui-audit-check` と `just ci` が通る

## 参考

- T0970 のメモ (測ったこと・起票の候補)、docs/ui.md (`--ugoe-scrollbar-gap`)、packages/ui/src/tokens.test.ts
- tests/ui-audit/lib/page-checks.js、tests/ui-audit/README.md

## メモ

- T0052 で起票した (元の候補: T0970 (重ねて描くスクロールバーを真似た検査))。
- エディタの送る枠を直す T1662 と、同じ tests/ui-audit を触る T0054 の後にした。Select・Menu の中 (T0976) が先に入っていなければ、
  それらの指摘はメモに書くだけでよい。
- やったこと:
  - `lib/page-checks.js` に `scrollbarAudit` (DOM から枠と部品を集める) と `underScrollbar` (判定の本体) を足し、新しい検査
    `scrollbar` (報告の見出し「重ねて描くスクロールバーの下の文字・部品」) にした。枠は `overflow-y: auto|scroll` で今縦に送れるもの
    (画面の全体の送りを含む)。横にも今送れる枠は見ない。文字の行の矩形と `target-size` と同じ押せる部品の、見える右端と、枠の見える右端
    (`clientWidth` の右) の空きが `--ugoe-scrollbar-gap` - 0.5px より小さければ数える。部品ごとにいちばん内側の枠で 1 つ。
    数えないもの・閾値・効き目・残る指摘は tests/ui-audit/README.md の「重ねて描くスクロールバー」に書いた。
  - `test/checks.test.mjs` に 4 つのテスト (空けない枠の中の部品・空けた枠/送らない枠/横に送る枠/縦横に送る枠/枠の外・入れ子の枠・
    `pageFindings` と報告の見出し)。
  - 効き目: T0970 (ビューア・スタジオ) と T1662 (エディタ) の CSS の差分を作業ツリーに一時的に逆に当て、`--quick` で流した:
    ビューア 3 種類 (60)・スタジオ 9 種類 (118)・エディタ 23 種類 (2020)。戻した後は下の残りだけ。
  - 3 つのアプリのすべての場面 (エディタは `--variants en-`・`ja-` に分けて 278 + 278、ビューア 89、スタジオ 183) を流した。
    `scrollbar` のほかの検査は 0。`scrollbar` は、ビューア 0・スタジオ 0 (T0976 の Select・Menu も 0)、エディタ 3 種類:
    - 直した (範囲の外の小さな修正): パラメータのパネルのスライダー (`input@.slider`。空き 14px)。標準の `input[type=range]` の左右の
      margin (2px) で、幅 100% の欄が右へ 2px はみ出していた。ParametersPanel.svelte の `.slider input` に `margin-inline: 0` を足した
      (キーの印の位置 `markerLeft` は欄の左端を基準にするので、印とつまみも 2px ずれていたのがそろう)。直した後に en・ja ともに消えた。
      見た目は 2px の違いだけなので `just e2e --only editor` は流していない。
    - 意図したもの: `button.marker` (パラメータの最大の値のキーの印。空き 12px) と `button.diamond` (モーションの表の 4 秒 (終わり) の
      キーの印。空き 8.7px)。押せる範囲 (24px) を見た目の印 (◆) より広げて値・時刻の位置の中心に置くので、範囲の端が帯の端の先へ出る。
      見た目の印は空きの中 (それぞれ約 20px・13.7px) にあり、指を乗せたスクロールバーが押せる範囲の端の数 px を取るだけ。
    - 直していない: `*-zoom200-settings-ai` の大きいダイアログの閉じるボタン (`.ugoe-dialog__close`。空き 12px)。200% の拡大でダイアログの
      全体が送るときだけ、指を乗せたスクロールバーが 2px 掛かる。packages/ui の Dialog で右を `space-4` (12px) から変えるとすべての
      ダイアログの見た目が変わる (コンパクトで 4px) ので、範囲の外では直さず起票の候補にした。
  - docs/ui.md の送りの「確かめ方」に `just ui-audit` の `scrollbar` を足した。
  - `just ui-audit-check` と `just ci` が通った。`just ci` の 1 回目は、worktree を作ったときの wasm の生成物 (gitignore 済み) が
    main の新しい `wasm-types` の検査に合わず落ちたので、`just wasm` で作り直して通した (変更とは無関係)。
- 確かめていないこと: 本物の WebKitGTK・WKWebView で、検査が 0 の画面が本当に隠れないか (この検査はヘッドレスの Chrome での空きを測る
  近似。WebKitGTK の太ったスクロールバーは 14px までなので 16px の閾値で足りる)。左に置くスクロールバー・横のスクロールバー。
- 起票の候補:
  - @ugoe/ui の Dialog の閉じるボタンを、ダイアログが送るとき (か、いつも) 行末から `--ugoe-scrollbar-gap` 離す (上の直していない指摘)。

