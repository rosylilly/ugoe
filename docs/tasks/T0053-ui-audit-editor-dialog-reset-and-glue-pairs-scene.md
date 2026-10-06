---
id: T0053
title: "ui-audit: エディタの場面ごとに外での変更のダイアログを閉じ、グルーの組の一覧と作り直しの行を撮る場面を足す"
status: todo
milestone: M10
crates: [tests, docs]
depends_on: []
assignee:
---

# T0053: ui-audit: エディタの場面ごとに外での変更のダイアログを閉じ、グルーの組の一覧と作り直しの行を撮る場面を足す

## 目的

`just ui-audit --only editor` の 2 つの穴を埋める (どちらも `tests/ui-audit/lib/apps.mjs` のエディタの場面)。

- 外での変更を知らせるダイアログ (T0044 の場面 `-external-change`) が次の場面に残り、`ja-light-zoom200` で `locator.click` が時間切れに
  なることがあった (T1655)。場面の失敗として数えられ、組を流し直さないと結果がそろわない。
- `-glue-edit` は軸を足す欄までしか写さないので、組の一覧 (「Pairs」の行) と組の作り直しの行が狭い列でどう見えるかは撮られていない (T1650)。
  組の行を強調した形 (T1651) も撮られていない。

## 受け入れ条件

- [ ] `-external-change` の後 (と、ダイアログ・メニューを開いたままにする場面の後) に、ダイアログを閉じて状態を戻してから次の場面へ進む
      (戻せたことを待つ。閉じ方は場面の側か `ctx.scene` の共通の後始末のどちらかで、選んだ理由をメモに書いた)
- [ ] 外での変更のダイアログが残る形 (後始末を外した一時の変更) で、次の場面が失敗するか、残ったことを報告に出すのを確かめた
- [ ] 場面 `-glue-edit-pairs` (グルーのパネルを下まで送り、組の一覧と作り直しの行を写す。`scrollToElement`・`scrollToSection` を使う) と、
      組の行にフォーカスしてビューポートでその組を強調した場面を足した (同じ場面にまとめてもよい)
- [ ] tests/ui-audit/README.md の場面の表を直した。`just ui-audit-check` が通る
- [ ] `just ui-audit --only editor` (600 秒に収まらなければ `--variants` で分けて、すべての組) で、場面の失敗が 0、新しい場面の指摘を
      メモに書いた (直すものは起票の候補にする)
- [ ] `just ci` が通る

## 参考

- T1655 のメモ (ui-audit の `--compare` の失敗)、T1650 のメモ (起票の候補)、T1651 のメモ (やらなかったこと)、T0044 (`-external-change`)
- tests/ui-audit/lib/apps.mjs、tests/ui-audit/README.md

## メモ

- T0052 で起票した (元の候補: T1655 (外での変更のダイアログが次の場面に残る)・T1650 (`-glue-edit-pairs`)・T1651 (行の強調の場面は足していない))。
  同じ apps.mjs のエディタの場面なので 1 つにまとめた。
