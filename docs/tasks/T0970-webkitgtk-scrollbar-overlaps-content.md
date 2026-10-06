---
id: T0970
title: "ビューア・スタジオ: WebKitGTK のスクロールバーが中身 (パラメータのグループの数・スタジオの右の列の端) に重なるのを直す"
status: done
milestone: M10
crates: [apps/viewer, apps/studio, packages/ui, packages/player, docs]
depends_on: []
assignee: claude-worker-T0970
---

# T0970: ビューア・スタジオ: WebKitGTK のスクロールバーが中身 (パラメータのグループの数・スタジオの右の列の端) に重なるのを直す

## 目的

T0919 で本物の WebKitGTK (`just e2e --manual`) で撮ると、GTK のスクロールバーが送る枠の中身の上に重なった
(ビューアのパラメータのグループの見出しの数、スタジオの右の列の右端)。ヘッドレスの Chrome (`just ui-audit`) では重ならないので監査は出さない。
`scrollbar-gutter` などで、スクロールバーの分の幅を中身から空ける。

## 受け入れ条件

- [x] 重なる送る枠を洗い出した (ビューア・スタジオ。エディタにもあれば含める)。直し方を共通にできるなら @ugoe/ui のトークンかクラスにした
- [x] 直した後の `just e2e --manual` の画像 (ビューアのパラメータ・スタジオの右の列) で、数や右端の部品がスクロールバーの下に隠れない
- [x] 送らない (中身が収まる) ときに、空けた幅が見た目を崩さない (`just ui-audit --only viewer,studio` の `--compare` で増えた指摘が無い)
- [x] docs/ui.md (レイアウトの「送り」) と tests/e2e/README.md#ヘッドレスの-chrome-just-ui-audit-との違い を直した
- [x] `just e2e --only viewer`・`just e2e --only studio` と `just ci` が通る

## 参考

- T0919 のメモ (起票の候補)・docs/roadmap.md の M10 の「後続の候補 (T0919)」
- tests/e2e/README.md、docs/ui.md

## メモ

- T0046 で起票した (元の候補: T0919 (WebKitGTK のスクロールバーが中身に重なる))。
- WebView2・WKWebView のスクロールバーは重ならない形 (overlay) のことがある。確かめられないので、確かめていないことをメモに書く。
- やったこと:
  - 本物の WebKitGTK 2.52 (Xvfb) で測った: スクロールバーは幅を取らずに重ねて描く (`offsetWidth - clientWidth` が 0)。
    `scrollbar-gutter: stable` (`both-edges` も)・`scrollbar-width: thin` も 0 で、幅を空けない。細い印は右端から 3〜6px、
    指を乗せると 6〜14px に太る。なので `scrollbar-gutter` ではなく padding で空けた。
  - @ugoe/ui の tokens.css に `--ugoe-scrollbar-gap` (16px。密度に依らない) を足した。クラスにしなかったのは、送る枠ごとに
    padding の書き方が違い、アプリの scoped な規則が共通のクラスより強いため (トークンを行末の padding に使う決めにした)。
  - 洗い出し (ビューア・スタジオ): ビューアの機能のパネルの段 (`.features`。12px)・パラメータの一覧 (`.list`。0px。グループの数が重なっていた)・
    覚えたものの表・ライセンスの文、スタジオの右の列 (`.studio__panels`。8px)・シーンの一覧 (8px)・ライセンスの文。右の padding を
    `--ugoe-scrollbar-gap` にした。パラメータの一覧はパネルの右の padding まで広げ (`margin-inline-end` を負に)、その中に描かせる
    (見出し・絞り込みの欄と右の端がそろう)。スタジオの中央 (space-5 = 16px 以上) と、狭い幅の画面の全体の送り (中の段が空ける) はそのまま。
  - シーンの一覧の中身が 8px 狭くなり「avatar-lit.ugoe」が省略されたので、シーンの列を 0.5rem 広げた (13.5〜17.5rem。docs/studio.md・docs/ui.md)。
  - packages/ui の `tokens.test.ts` に、ビューアとスタジオの送る枠 (`overflow(-y): auto|scroll`) の規則が `--ugoe-scrollbar-gap` を使うか、
    使わない理由をスクロールバーの言葉で書いているかの検査を足した (空けていない規則が残っていると落ち、その規則を名前で出すことを確かめた)。
  - `just e2e --manual --only viewer,studio` の前と後の画像で、パラメータのグループの数 (3・5) とスタジオの右の列の右端が
    スクロールバーに掛からないことを見た。`--update-images` でビューアとスタジオのマニュアルの画像を撮り直した。
  - `just ui-audit --only viewer,studio --compare` (直す前の CSS を一時的に戻して撮った基準と比べた): viewer +0 -0 ~0、studio +0 -0 ~0。
  - `just e2e --only viewer,studio` (筋書き) が通った (6 / 13)。`just ci` が通った。
  - 範囲の外の小さな修正: `tests/e2e/specs/{viewer,studio}.manual.mjs` の言語の localStorage のキーが T0957 の改名 (`ugoe.viewer.locale`・
    `ugoe.studio.locale`) に追従しておらず、`just e2e --manual` が main でも最初の手順から落ちていたので直した。
  - `--update-images` の 1 回目で、スタジオの最初の手順 (起動) が 30 秒で時間切れになった (機械の負荷。2 回目は通った)。
- やらなかったこと・確かめていないこと:
  - WebView2 (Windows)・WKWebView (macOS) は確かめていない。macOS の重ねて描くスクロールバーも同じ padding で避けられるはずだが未確認。
  - エディタ (apps/editor は `crates:` の外): パネルの送る枠 (`.pane`・`aside`。右 12px で、指を乗せたときだけ 2px 掛かる)・
    PickList・GluesPanel の対の一覧・LightingPanel のメッシュの一覧・MotionEditor・ExpressionEditor の表・FileMenu の警告の一覧・
    Viewport の空の案内・LicenseDialog が `--ugoe-scrollbar-gap` を使っていない。
  - @ugoe/ui の `Select`・`Menu` の中身 (padding 2px) は直していない。低い窓でしか送らず、空けるとすべてのメニューが 14px 広がるため。
- 起票の候補:
  - エディタの送る枠に `--ugoe-scrollbar-gap` を使い、`tokens.test.ts` の検査の対象にエディタを足す (上の一覧)。
  - `Select`・`Menu` が送るときだけ行末を空ける (送るかどうかで padding を変えるか、行の右の要素 (ショートカット) の側で空ける)。
  - `just ui-audit` に、重ねて描くスクロールバーを真似た検査 (送る枠の中身の右端が、枠の右端から `--ugoe-scrollbar-gap` の中にあれば指摘)。
