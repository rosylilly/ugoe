---
id: T1002
title: "アプリのアイコンを SVG で描き直す (エディタ・ビューア・スタジオ。スタジオは専用のものに)。ライセンスは MIT"
status: in-progress
milestone: Release
crates: [apps/editor, apps/viewer, apps/studio, assets, docs]
depends_on: []
assignee: claude-worker-T1002
---

# T1002: アプリのアイコンを SVG で描き直す

## 目的

T0097 で、エディタのアイコンの作者・作り方が分からないことが分かった (最初のコミットで入り、記録が無い)。ビューアは自作の `icon.png` (元の絵は無い)、スタジオはビューアのものをバイトまで写したもので、タスクバーや Dock で見分けられない。

## 決まっていること (利用者、2026-10-11)

- 出所の分からないエディタのアイコンは使わず、描き直す。
- 3 つのアプリのアイコンのライセンスは MIT。

## 受け入れ条件

- [ ] 3 つのアプリのアイコンを SVG で描き、リポジトリに置いた (置き場所を決めて書く。例: `assets/brand/` など MIT の場所)。同じ系統の形で、アプリごとに見分けがつく (色か印で分ける)。小さい大きさ (16・32 px) でもつぶれない
- [ ] SVG から `pnpm tauri icon` などで大きさ違いのファイルを作り直す手順を just のレシピか README に書き、作り直したファイルを置き換えた
- [ ] docs/release.md#アプリのアイコン の表を、作者 (このリポジトリで作った)・ライセンス (MIT)・元の SVG の場所に直し、「利用者に確かめる」の項を消した
- [ ] `apps/studio/src-tauri/tests/fixtures/red-blue.jpg` (T0808 のテスト用の画像) の出所を確かめ、出所の表に載せた (作り直せる単純な画像なら作り方を書く)
- [ ] マニュアルの画像やスクリーンショットにアイコンが写っていれば、撮り直すか、そのままでよい理由を書いた
- [ ] `just ui-audit` (アプリごとに `--only`) と `just ci` が通る。`DRY_RUN=1 just app::build` が通る

## 参考

- T0097 のメモ、docs/release.md#アプリのアイコン、apps/*/src-tauri/icons、apps/viewer/README.md (アイコンの作り直しの手順)

## メモ

- 利用者の決定 (2026-10-11) を受けてオーケストレーターが起票した。元の候補: T0097 (スタジオに専用のアイコン・red-blue.jpg の出所)。
