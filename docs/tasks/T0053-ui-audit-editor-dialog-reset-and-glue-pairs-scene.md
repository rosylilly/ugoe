---
id: T0053
title: "ui-audit: エディタの場面ごとに外での変更のダイアログを閉じ、グルーの組の一覧と作り直しの行を撮る場面を足す"
status: done
milestone: M10
crates: [tests, docs]
depends_on: []
assignee: claude-worker-T0053
---

# T0053: ui-audit: エディタの場面ごとに外での変更のダイアログを閉じ、グルーの組の一覧と作り直しの行を撮る場面を足す

## 目的

`just ui-audit --only editor` の 2 つの穴を埋める (どちらも `tests/ui-audit/lib/apps.mjs` のエディタの場面)。

- 外での変更を知らせるダイアログ (T0044 の場面 `-external-change`) が次の場面に残り、`ja-light-zoom200` で `locator.click` が時間切れに
  なることがあった (T1655)。場面の失敗として数えられ、組を流し直さないと結果がそろわない。
- `-glue-edit` は軸を足す欄までしか写さないので、組の一覧 (「Pairs」の行) と組の作り直しの行が狭い列でどう見えるかは撮られていない (T1650)。
  組の行を強調した形 (T1651) も撮られていない。

## 受け入れ条件

- [x] `-external-change` の後 (と、ダイアログ・メニューを開いたままにする場面の後) に、ダイアログを閉じて状態を戻してから次の場面へ進む
      (戻せたことを待つ。閉じ方は場面の側か `ctx.scene` の共通の後始末のどちらかで、選んだ理由をメモに書いた)
- [x] 外での変更のダイアログが残る形 (後始末を外した一時の変更) で、次の場面が失敗するか、残ったことを報告に出すのを確かめた
- [x] 場面 `-glue-edit-pairs` (グルーのパネルを下まで送り、組の一覧と作り直しの行を写す。`scrollToElement`・`scrollToSection` を使う) と、
      組の行にフォーカスしてビューポートでその組を強調した場面を足した (同じ場面にまとめてもよい)
- [x] tests/ui-audit/README.md の場面の表を直した。`just ui-audit-check` が通る
- [x] `just ui-audit --only editor` (600 秒に収まらなければ `--variants` で分けて、すべての組) で、場面の失敗が 0、新しい場面の指摘を
      メモに書いた (直すものは起票の候補にする)
- [x] `just ci` が通る

## 参考

- T1655 のメモ (ui-audit の `--compare` の失敗)、T1650 のメモ (起票の候補)、T1651 のメモ (やらなかったこと)、T0044 (`-external-change`)
- tests/ui-audit/lib/apps.mjs、tests/ui-audit/README.md

## メモ

- T0052 で起票した (元の候補: T1655 (外での変更のダイアログが次の場面に残る)・T1650 (`-glue-edit-pairs`)・T1651 (行の強調の場面は足していない))。
  同じ apps.mjs のエディタの場面なので 1 つにまとめた。

### やったこと (T0053)

- **重ねものの後始末 (`tests/ui-audit/lib/apps.mjs` の `closeOverlay`):** ダイアログ・メニュー・Popover を開いて撮る場面
  (エディタの `-settings`・`-file-menu`・`-export`・`-external-change`・`-motion-settings`・`-settings-ai`・`-base-change`、ビューアの
  `-background`・`-settings`、スタジオの `-settings`・`-profile-menu`) の後は、閉じるまで Esc を 3 回まで押して待ち (閉じられなければ場面の
  失敗)、戻ったフォーカスとポインタを外す (閉じた後にフォーカスが開いたボタンへ戻り、その Tooltip が後の場面の部品に重なるため)。
  `-external-change` は閉じる前に偽のホストの外の変化を消す (閉じた後の一定の間隔・ウィンドウに戻ったときの確かめで開き直さないように)。
  `-export` は Popover だったので、待つ・閉じるのを `.ugoe-popover` にした (前は 600ms 待つだけ)。
  - **場面の側を選んだ理由:** 閉じ方 (Esc か、メニューの項目を選ぶか) と、何が開いたか (dialog・menu・Popover) は場面ごとに違い、
    `ctx.scene` の共通の後始末では「開いたままにしたいのか、閉じ忘れか」を決められない。後始末は場面の側の共通の関数にし、
    `ctx.scene` の側には**閉じ忘れを見つける検査**を置いた: `ctx.scene(name, { overlay: true })` でない場面で、開いているダイアログ・
    メニュー・Popover (page-checks.js の `openOverlays`) があれば `error` に `left open by an earlier scene` として数える
    (audit.mjs の `pageFindings` の `expectOverlay`。テストは checks.test.mjs)。
- **後始末を外した確かめ (一時の変更。コミットしていない):** `-external-change` の `closeOverlay` を外して `en-light-wide` を流すと、
  次の場面 (`-tree-hover`) が `locator.hover: Timeout 30000ms exceeded` で失敗した (T1655 と同じ形)。さらに外した直後に場面を 1 つ足すと、
  その場面に `div.ugoe-dialog.ugoe-dialog--md "The project was changed outside the edit" left open by an earlier scene` が出た。
- **グルーの場面 (`glueScene`):** `-glue-edit-pairs` (編集の欄の終わり「Remove glue」の下端まで送る: 組の一覧・組を足す行・組の作り直しの行。
  `scrollToElement` に下端を合わせる `{ end: true }` を足した。狭い列 (`*-zoom200`) では列の高さが足りず、組の一覧の頭まで送ると作り直しの行が
  写らないため) と、`-glue-pair-focus` (組の一覧の頭 (`scrollToElement` で sticky のタブを引く) へ戻し、2 つ目の組の「×」にフォーカスして
  `li.highlighted` を待つ: 行の強調とビューポートの組の太い線)。
- README の場面の表・「場面を足すとき」・`error` の検査の行を直した。`just ui-audit-check` が通る。
- **`just ui-audit --only editor`:** 前面の 600 秒に収まらないので、ビルドを 1 回 (`--variants build-only`) してから `--no-build` で 4 つに分けて
  13 の組すべてを流した (`en-light-wide,en-light-narrow,en-dark-wide` / `en-dark-narrow,ja-light-wide,ja-light-narrow` (`ja-light-wide-dpr2` を含む) /
  `ja-dark-wide,ja-dark-narrow,en-light-zoom200` / `ja-light-zoom200,en-dark-forced-wide,ja-light-forced-narrow`。main 2df121d2 に rebase した後)。
  556 場面 (139・140・138・139) で**場面の失敗は 0**。新しい場面 (`-glue-edit-pairs`・`-glue-pair-focus`) の指摘は、すべての組で 0。
  - 出た指摘は 1 種類だけ: `*-zoom200` (英語・日本語) の `-base-change`・`-tree-base-hover`・`-module-ai` で、ツリーの本体の札の見えない文
    (`span.ugoe-visually-hidden "base"` / `"本体"`) が `escapes aside.left` (横のはみ出し)。main の ui-audit のスクリプトのまま
    (`git checkout main -- tests/ui-audit`) 同じビルドで `ja-light-zoom200` を流しても同じ数が出るので、この変更とは関係ない
    (T1654 (14859c22) で入ったツリーの札の変更から出ていると見ている)。
  - 新しい場面の画像を見た: 広い幅は組の一覧・作り直しの行・「Remove glue」まで 1 枚に写る。`ja-light-zoom200` では「組を作り直す距離」の行の
    「組にする」が次の行へ折り返すが、重なり・切れは無い。フォーカスした組の「×」の Tooltip が下の組の行に重なる (フォーカスで開く Tooltip の
    ふつうの形)。
- `just ci` が通った (rebase の後)。

### やらなかったこと

- 画面 (アプリ) は変えていないので `just e2e` は流していない。
- ビューアとスタジオ (`closeOverlay` に替えた場面がある) は、すべての組ではなく `--variants light-wide,light-narrow,zoom200,forced`
  (英語・日本語のライトの広い幅・狭い幅・`ja-light-wide-dpr2`・200% の拡大・強制色の 2 組) だけ流した: viewer 53・studio 99 場面で、
  場面の失敗も指摘も 0。

### 起票の候補

- エディタ: 200% の拡大の組 (`*-zoom200`) で、モジュールのプロジェクトのツリーの本体の札の見えない文 (`span.ugoe-visually-hidden`) が
  左の列 (`aside.left`) の外へ出る (`overflow` の `escapes`)。T1654 の札の変更の続き。
