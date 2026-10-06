---
id: T0970
title: "ビューア・スタジオ: WebKitGTK のスクロールバーが中身 (パラメータのグループの数・スタジオの右の列の端) に重なるのを直す"
status: in-progress
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

- [ ] 重なる送る枠を洗い出した (ビューア・スタジオ。エディタにもあれば含める)。直し方を共通にできるなら @ugoe/ui のトークンかクラスにした
- [ ] 直した後の `just e2e --manual` の画像 (ビューアのパラメータ・スタジオの右の列) で、数や右端の部品がスクロールバーの下に隠れない
- [ ] 送らない (中身が収まる) ときに、空けた幅が見た目を崩さない (`just ui-audit --only viewer,studio` の `--compare` で増えた指摘が無い)
- [ ] docs/ui.md (レイアウトの「送り」) と tests/e2e/README.md#ヘッドレスの-chrome-just-ui-audit-との違い を直した
- [ ] `just e2e --only viewer`・`just e2e --only studio` と `just ci` が通る

## 参考

- T0919 のメモ (起票の候補)・docs/roadmap.md の M10 の「後続の候補 (T0919)」
- tests/e2e/README.md、docs/ui.md

## メモ

- T0046 で起票した (元の候補: T0919 (WebKitGTK のスクロールバーが中身に重なる))。
- WebView2・WKWebView のスクロールバーは重ならない形 (overlay) のことがある。確かめられないので、確かめていないことをメモに書く。
