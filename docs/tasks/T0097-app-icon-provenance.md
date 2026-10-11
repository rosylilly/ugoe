---
id: T0097
title: "公開: エディタ・ビューア・スタジオのアプリのアイコンの出所とライセンスを記録する"
status: todo
milestone: Release
crates: [apps/editor, apps/viewer, apps/studio, assets, docs]
depends_on: []
assignee:
---

# T0097: アプリのアイコンの出所

## 目的

T0090 の点検で、アプリのアイコン (`apps/*/src-tauri/icons/`) の出所とライセンスの書き付けが見つからなかった。ほかの画像・モデル・PSD は
assets の README の表に出所がある。公開の前に、アイコンも同じように出所とライセンスを記録する。

## 受け入れ条件

- [ ] 3 つのアプリのアイコン (元の絵と、`tauri icon` などで作った大きさ違いのファイル) の出所 (作者・作り方・使った道具) とライセンスを、
      1 か所の表 (例: 各アプリの README か assets の README) に書いた。出所は利用者に確かめる (分からない・再配布できないものがあれば、
      案 (作り直す・差し替える) と推奨をメモに書いて「判断が要る」で止まる)
- [ ] 元の絵 (大きさ違いを作り直せる SVG か大きな PNG) がリポジトリにあるか、無ければどこにあるかを書いた
- [ ] docs/release.md#自動の検査 の「外部 SDK・ライセンス」の行から表へ辿れる
- [ ] `just ci` が通る

## 参考

- docs/release.md#公開の前の点検、assets/fixtures/README.md・assets/samples/README.md (出所の表の書き方)
- T0090 のメモ (起票の候補)

## メモ

- T0095 で起票した (元の候補: T0090 (アプリのアイコンの出所とライセンスを記録する))。
- Release のマイルストーン。アイコンの絵は GPL-3.0-only のアプリに入るが、アイコンそのもののライセンスは別に書く。
