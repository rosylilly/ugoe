---
id: T1002
title: "アプリのアイコンを SVG で描き直す (エディタ・ビューア・スタジオ。スタジオは専用のものに)。ライセンスは MIT"
status: done
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

- [x] 3 つのアプリのアイコンを SVG で描き、リポジトリに置いた (置き場所を決めて書く。例: `assets/brand/` など MIT の場所)。同じ系統の形で、アプリごとに見分けがつく (色か印で分ける)。小さい大きさ (16・32 px) でもつぶれない
- [x] SVG から `pnpm tauri icon` などで大きさ違いのファイルを作り直す手順を just のレシピか README に書き、作り直したファイルを置き換えた
- [x] docs/release.md#アプリのアイコン の表を、作者 (このリポジトリで作った)・ライセンス (MIT)・元の SVG の場所に直し、「利用者に確かめる」の項を消した
- [x] `apps/studio/src-tauri/tests/fixtures/red-blue.jpg` (T0808 のテスト用の画像) の出所を確かめ、出所の表に載せた (作り直せる単純な画像なら作り方を書く)
- [x] マニュアルの画像やスクリーンショットにアイコンが写っていれば、撮り直すか、そのままでよい理由を書いた
- [x] `just ui-audit` (アプリごとに `--only`) と `just ci` が通る。`DRY_RUN=1 just app::build` が通る

## 参考

- T0097 のメモ、docs/release.md#アプリのアイコン、apps/*/src-tauri/icons、apps/viewer/README.md (アイコンの作り直しの手順)

## メモ

- 利用者の決定 (2026-10-11) を受けてオーケストレーターが起票した。元の候補: T0097 (スタジオに専用のアイコン・red-blue.jpg の出所)。

### やったこと (T1002)

- 3 つのアイコンを SVG で手で書き、`assets/brand/` (`editor.svg`・`viewer.svg`・`studio.svg`。出所の表は `assets/brand/README.md`) に置いた。
  外部の画像・フォント・アイコン集は使っていない (四角・楕円・2 次ベジェだけ。文字なし)。
  - 共通の形: 角丸の四角に、少し (-10°) 傾けた白い顔 (目の 2 つの点と口)。動かす 2D の人形を表す。
  - 見分け: 背景の色 (エディタ 紫・ビューア 青・スタジオ 赤) と印 (エディタ = 顔の輪郭の上下左右の頂点のハンドル (リギング)、
    スタジオ = 右上の配信中の印 (白い輪に赤い点)、ビューア = 印なし)。
  - Tauri の CLI (resvg) で 16・32・128・512 px に描き出して目で確かめた (明るい背景と暗い背景の上、拡大)。16 px では印は
    つぶれるが、色と白い顔 (目の点) で見分けられる。32 px ではスタジオの印が見え、エディタのハンドルは輪郭の凹凸として見える。
- 大きさ違いの作り直しを `just app::icons` (`scripts/app.just`) にし、3 つのアプリの `src-tauri/icons/` を作り直した
  (ファイルの組は前と同じ。Android・iOS 用のディレクトリは消す)。エディタの `.mcpb` のアイコン (`icon.png` を `include_bytes!`)
  も新しい絵になる (`just crate ugoe-editor` は通る)。
- docs/release.md#アプリのアイコン の表を、作者 (このリポジトリで作った)・ライセンス (MIT)・元の SVG の場所に直し、
  「人がすること」のアイコンの項と「まだ決まっていない」の書き込みを消した。apps/viewer/README.md の作り直しの手順も直した。
- `red-blue.jpg`: T0808 のメモ (`09f13a7f`) に「Chrome で書いた 16x8 の JPEG」とあり、左が赤・右が青の 2 色の四角だけの絵。
  docs/release.md に「テスト用の画像」の表を足して載せ、ImageMagick で作り直す命令を書いた (作った画像がテストの画素の許容誤差に
  収まることを確かめた)。Chrome のどの機能で書いたかの記録は無いので、そこは書いていない。
- マニュアルの画像 (`docs/manual/images/`) は Web ビューの中だけを撮ったもので、窓の枠・タスクバーを含まずアイコンは写っていない
  (`mcp-claude-desktop.png` もエディタの設定の画面だけ)。撮り直しは要らないと docs/release.md に書いた。

### やらなかったこと・確かめていないこと

- 実際の Windows のタスクバー・macOS の Dock・Linux のデスクトップでの見え方は確かめていない (`icon.ico`・`icon.icns` の中の
  大きさは Tauri の CLI が作るもの)。macOS のアイコンの決まり (角丸の四角の周りの余白) には合わせず、3 つの OS で同じ絵にしている。
- `scripts/app.just` (`crates:` の外) に `icons` のレシピを足した (小さな変更。受け入れ条件の「just のレシピか README」のため)。
- AGENTS.md のリポジトリ構成の図に `assets/brand/` は足していない (`assets/samples/` も載っていないのに合わせた)。

### 起票の候補

- 実際の Windows・macOS でタスクバー・Dock・インストーラのアイコンの見え方を確かめる (リリースの前の手作業の確認の一覧に入れる)。
- 検査: `just ui-audit --only editor|viewer|studio` (どれも 0 件・エラー 0)、`DRY_RUN=1 just app::build`、`just ci` が通った (main に追従した後)。
